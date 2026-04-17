#!/usr/bin/env bash
set -u
set -o pipefail

usage() {
  cat <<USAGE
Usage: $(basename "$0") [options]

Optional:
  --authority <pubkey>     Trader authority pubkey for trader endpoints
                           (default: solana-keygen pubkey ~/.config/solana/id.json)
  --api-url <url>          Phoenix API base URL (optional; falls back to CLI/SDK env defaults)
  --api-key <key>          API key
  --symbol <symbol>        Market symbol (default: SOL)
  --timeframe <tf>         Candle timeframe (default: 1m)
  --limit <n>              Limit for history endpoints (default: 10)
  --cli-cmd <cmd>          CLI invocation prefix (default: "cargo run -q -p phoenix-sdk-cli --")
                           If run from git repo root, script auto-runs cargo in ./rust.

Example:
  $(basename "$0") --symbol SOL
  $(basename "$0") --authority 11111111111111111111111111111111 --symbol SOL
USAGE
}

API_URL=""
AUTHORITY=""
API_KEY=""
SYMBOL="SOL"
TIMEFRAME="1m"
LIMIT="10"
CLI_CMD="cargo run -q -p phoenix-sdk-cli --"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --api-url)
      API_URL="${2:-}"
      shift 2
      ;;
    --authority)
      AUTHORITY="${2:-}"
      shift 2
      ;;
    --api-key)
      API_KEY="${2:-}"
      shift 2
      ;;
    --symbol)
      SYMBOL="${2:-}"
      shift 2
      ;;
    --timeframe)
      TIMEFRAME="${2:-}"
      shift 2
      ;;
    --limit)
      LIMIT="${2:-}"
      shift 2
      ;;
    --cli-cmd)
      CLI_CMD="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$AUTHORITY" ]]; then
  if ! command -v solana-keygen >/dev/null 2>&1; then
    echo "--authority not provided and solana-keygen is not available" >&2
    usage >&2
    exit 1
  fi

  AUTHORITY="$(solana-keygen pubkey ~/.config/solana/id.json 2>/dev/null || true)"
  if [[ -z "$AUTHORITY" ]]; then
    echo "--authority not provided and failed to read ~/.config/solana/id.json via solana-keygen" >&2
    usage >&2
    exit 1
  fi

  echo "Using authority from ~/.config/solana/id.json: $AUTHORITY"
fi

base=( $CLI_CMD )
if [[ -n "$API_URL" ]]; then
  base+=( --api-url "$API_URL" )
fi
if [[ -n "$API_KEY" ]]; then
  base+=( --api-key "$API_KEY" )
fi

RUN_DIR="$(pwd)"
if [[ ! -f "$RUN_DIR/Cargo.toml" ]] && command -v git >/dev/null 2>&1; then
  GIT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -n "$GIT_ROOT" && -f "$GIT_ROOT/rust/Cargo.toml" ]]; then
    RUN_DIR="$GIT_ROOT/rust"
    echo "Detected repo layout; running checks from $RUN_DIR"
  fi
fi

failures=()

run_check() {
  local name="$1"
  shift

  echo "==> $name"
  if ( cd "$RUN_DIR" && "${base[@]}" "$@" >/dev/null ); then
    echo "  OK"
  else
    echo "  FAIL"
    failures+=("$name")
  fi
}

run_check "exchange-keys" http exchange-keys
run_check "markets" http markets
run_check "market" http market --symbol "$SYMBOL"
run_check "exchange" http exchange
run_check "traders" http traders --authority "$AUTHORITY"
run_check "collateral-history" http collateral-history --authority "$AUTHORITY" --pda-index 0 --limit "$LIMIT"
run_check "funding-history" http funding-history --authority "$AUTHORITY" --pda-index 0 --symbol "$SYMBOL" --limit "$LIMIT"
run_check "order-history" http order-history --authority "$AUTHORITY" --limit "$LIMIT" --trader-pda-index 0 --market-symbol "$SYMBOL"
run_check "candles" http candles --symbol "$SYMBOL" --timeframe "$TIMEFRAME" --limit "$LIMIT"
run_check "trade-history" http trade-history --authority "$AUTHORITY" --pda-index 0 --market-symbol "$SYMBOL" --limit "$LIMIT"

echo
if [[ ${#failures[@]} -eq 0 ]]; then
  echo "All HTTP smoke checks passed."
  exit 0
fi

echo "HTTP smoke checks failed (${#failures[@]}):"
for item in "${failures[@]}"; do
  echo "  - $item"
done
exit 1
