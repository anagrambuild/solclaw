#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///

"""
Create and upload a git-history-free repository snapshot to S3.

Workflow:
1) Ensure AWS credentials are valid (or trigger `aws sso login`)
2) Ensure the git worktree is clean
3) Ensure the repository is on the target branch (default: master)
4) Create a zip snapshot from git-tracked files only
5) Remove excluded paths (for example `scripts/`) from the archive
6) Upload the archive to S3 with commit hash in the object name
"""

from __future__ import annotations

import argparse
import json
import shlex
import subprocess
import sys
import zipfile
from pathlib import Path
from typing import Sequence

EXCLUDED_ARCHIVE_PREFIXES = ("scripts/",)


def run(
    cmd: Sequence[str],
    *,
    cwd: str | None = None,
    check: bool = True,
    capture_output: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        cmd,
        cwd=cwd,
        text=True,
        capture_output=capture_output,
    )
    if check and result.returncode != 0:
        pretty_cmd = " ".join(shlex.quote(part) for part in cmd)
        stdout = result.stdout.strip() if result.stdout else ""
        stderr = result.stderr.strip() if result.stderr else ""
        details = "\n".join(part for part in [stdout, stderr] if part)
        raise RuntimeError(f"Command failed: {pretty_cmd}\n{details}".strip())
    return result


def aws_base_cmd(profile: str | None) -> list[str]:
    cmd = ["aws"]
    if profile:
        cmd.extend(["--profile", profile])
    return cmd


def ensure_aws_auth(profile: str | None, skip_sso_login: bool) -> None:
    identity_cmd = aws_base_cmd(profile) + ["sts", "get-caller-identity", "--output", "json"]
    identity = run(identity_cmd, check=False)
    if identity.returncode == 0:
        parsed = json.loads(identity.stdout)
        arn = parsed.get("Arn", "<unknown-arn>")
        print(f"AWS auth OK: {arn}")
        return

    if skip_sso_login:
        raise RuntimeError(
            "AWS credentials are not valid and --skip-sso-login was provided. "
            "Run `aws sso login` (or configure credentials) and retry."
        )

    login_cmd = aws_base_cmd(profile) + ["sso", "login"]
    pretty_login = " ".join(shlex.quote(part) for part in login_cmd)
    print(f"No valid AWS session found. Running: {pretty_login}")
    login = subprocess.run(login_cmd, text=True)
    if login.returncode != 0:
        raise RuntimeError("AWS SSO login failed.")

    identity = run(identity_cmd, check=True)
    parsed = json.loads(identity.stdout)
    arn = parsed.get("Arn", "<unknown-arn>")
    print(f"AWS auth OK after login: {arn}")


def get_repo_root(script_path: Path) -> Path:
    script_dir = script_path.resolve().parent
    result = run(["git", "rev-parse", "--show-toplevel"], cwd=str(script_dir))
    return Path(result.stdout.strip())


def ensure_clean_worktree(repo_root: Path) -> None:
    status = run(["git", "status", "--porcelain"], cwd=str(repo_root))
    if not status.stdout.strip():
        return

    preview_lines = status.stdout.strip().splitlines()[:20]
    preview = "\n".join(preview_lines)
    raise RuntimeError(
        "Git worktree is not clean. Commit, stash, or remove changes before running.\n"
        f"{preview}"
    )


def current_branch(repo_root: Path) -> str:
    branch = run(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=str(repo_root))
    return branch.stdout.strip()


def ensure_on_branch(repo_root: Path, target_branch: str) -> None:
    branch = current_branch(repo_root)
    if branch == target_branch:
        print(f"Git branch OK: {branch}")
        return

    print(f"Switching branch from {branch} to {target_branch}")
    run(["git", "switch", target_branch], cwd=str(repo_root), capture_output=False)
    switched = current_branch(repo_root)
    if switched != target_branch:
        raise RuntimeError(f"Failed to switch to {target_branch}; currently on {switched}")
    print(f"Git branch OK: {switched}")


def head_commit(repo_root: Path) -> str:
    commit = run(["git", "rev-parse", "HEAD"], cwd=str(repo_root))
    return commit.stdout.strip()


def should_exclude_path(path: str) -> bool:
    normalized = path.lstrip("./")
    for prefix in EXCLUDED_ARCHIVE_PREFIXES:
        clean_prefix = prefix.strip("/")
        if normalized == clean_prefix or normalized.startswith(f"{clean_prefix}/"):
            return True
    return False


def create_archive(repo_root: Path, commit_hash: str, output_dir: Path) -> Path:
    repo_name = repo_root.name
    archive_path = output_dir / f"{repo_name}-{commit_hash}.zip"
    raw_archive_path = output_dir / f".{repo_name}-{commit_hash}.raw.zip"
    output_dir.mkdir(parents=True, exist_ok=True)

    run(
        [
            "git",
            "archive",
            "--format=zip",
            "--output",
            str(raw_archive_path),
            commit_hash,
        ],
        cwd=str(repo_root),
    )

    with zipfile.ZipFile(raw_archive_path, "r") as source_zip, zipfile.ZipFile(
        archive_path,
        "w",
        compression=zipfile.ZIP_DEFLATED,
    ) as final_zip:
        for item_name in source_zip.namelist():
            if should_exclude_path(item_name):
                continue
            final_zip.writestr(item_name, source_zip.read(item_name))

    raw_archive_path.unlink(missing_ok=True)

    with zipfile.ZipFile(archive_path, "r") as zf:
        names = zf.namelist()
        bad_git_paths = [name for name in names if name == ".git" or name.startswith(".git/")]
        bad_excluded_paths = [name for name in names if should_exclude_path(name)]

    if bad_git_paths:
        raise RuntimeError(f"Archive unexpectedly includes git metadata: {bad_git_paths}")
    if bad_excluded_paths:
        raise RuntimeError(f"Archive unexpectedly includes excluded paths: {bad_excluded_paths}")

    print(f"Created archive: {archive_path}")
    return archive_path


def upload_to_s3(archive_path: Path, bucket: str, key: str, profile: str | None) -> str:
    destination = f"s3://{bucket}/{key}"
    cmd = aws_base_cmd(profile) + ["s3", "cp", str(archive_path), destination]
    pretty_cmd = " ".join(shlex.quote(part) for part in cmd)
    print(f"Uploading archive with: {pretty_cmd}")
    upload = subprocess.run(cmd, text=True)
    if upload.returncode != 0:
        raise RuntimeError("S3 upload failed.")
    return destination

def cp_to_current(rise_s3_location: str, bucket: str, profile: str | None):
    destination = f"s3://{bucket}/rise-current.zip"
    cmd = aws_base_cmd(profile) + ["s3", "cp", rise_s3_location, destination]
    upload = subprocess.run(cmd, text=True)
    if upload.returncode != 0:
        raise RuntimeError("S3 cp failed.")

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create a git-history-free repository zip and upload to S3."
    )
    parser.add_argument(
        "--bucket",
        default="phoenix-rise-public-sdk",
        help="Target S3 bucket name (default: phoenix-rise-public-sdk).",
    )
    parser.add_argument(
        "--profile",
        default="ellipsis",
        help="AWS CLI profile name (default: ellipsis).",
    )
    parser.add_argument(
        "--branch",
        default="master",
        help="Git branch to enforce before archiving (default: master).",
    )
    parser.add_argument(
        "--output-dir",
        default="./dist",
        help="Directory for the generated zip before upload (default: ./dist).",
    )
    parser.add_argument(
        "--skip-clean-worktree-check",
        action="store_true",
        help="Skip git clean-worktree enforcement before archiving.",
    )
    parser.add_argument(
        "--skip-sso-login",
        action="store_true",
        help="Do not call `aws sso login` when credentials are invalid.",
    )
    parser.add_argument(
        "--skip-upload",
        action="store_true",
        help="Create the archive but do not upload to S3.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    repo_root = get_repo_root(Path(__file__))
    output_dir_arg = Path(args.output_dir).expanduser()
    output_dir = (
        (repo_root / output_dir_arg).resolve()
        if not output_dir_arg.is_absolute()
        else output_dir_arg.resolve()
    )

    print(f"Repository root: {repo_root}")
    ensure_aws_auth(args.profile, args.skip_sso_login)
    if not args.skip_clean_worktree_check:
        ensure_clean_worktree(repo_root)
    ensure_on_branch(repo_root, args.branch)
    if not args.skip_clean_worktree_check:
        ensure_clean_worktree(repo_root)

    commit_hash = head_commit(repo_root)
    archive_path = create_archive(repo_root, commit_hash, output_dir)
    key = archive_path.name

    if args.skip_upload:
        print(f"Skipping upload. Archive is available at: {archive_path}")
        print(f"Suggested S3 key: {key}")
        return 0

    destination = upload_to_s3(archive_path, args.bucket, key, args.profile)
    destination = cp_to_current(destination, args.bucket, args.profile)
    print(f"Upload complete: {destination}")
    print(f"Public URL (if bucket/object is public): https://{args.bucket}.s3.amazonaws.com/{key}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
