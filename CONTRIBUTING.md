# Contributing

## Ways to Contribute

### 1. Add a New Skill (Protocol Integration)

Skills teach the agent how to interact with a Solana protocol. Each skill lives in `container/skills/<protocol-name>/`.

#### Create the skill directory

```
container/skills/<your-protocol>/
  SKILL.md              # Required — instructions, examples, guidelines
  resources/            # SDK docs, API references
  examples/             # Copy-paste-ready TypeScript examples
  templates/            # Starter code agents can adapt
  docs/                 # Troubleshooting, architecture notes
```

#### Write SKILL.md

Use this frontmatter:

```yaml
---
name: your-protocol
creator: your-github-username
description: One-line description of what the skill covers.
---
```

The body should include:
- **Overview** — when to use this skill
- **Instructions** — step-by-step decision flow for the agent
- **Examples** — concrete "when user asks X, agent should do Y" patterns
- **Guidelines** — DO/DON'T list
- **Common Errors** — error messages and fixes
- **References** — official docs, SDK links

Look at existing skills (`jupiter`, `drift`, `breeze`, `manifest`) for the pattern.

#### Add program IDs to the transaction preload

Open `container/solana-tx-preload.cjs` and add your protocol's Solana program ID(s) to the `KNOWN_PROGRAMS` map:

```javascript
const KNOWN_PROGRAMS = {
  // ... existing entries ...
  // Your Protocol
  'YourProgramId111111111111111111111111111111': 'your-protocol',
};
```

This is how the auto-logging system detects which protocol a transaction belongs to. Without this, your protocol's transactions won't be attributed correctly.

Find your program ID(s) in:
- `declare_id!()` in the protocol's Rust source
- `Anchor.toml` in the protocol repo
- Solana Explorer — look up any known transaction from the protocol

#### Submit proof that it works

Include a screenshot or log showing the agent successfully executing a transaction using your skill. Any messenger (WhatsApp, Telegram, or terminal output) is accepted as proof.

### 2. Improve the System

Bug fixes, simplifications, and security fixes to source code are welcome.

### 3. Improve Existing Skills

- Fix outdated SDK methods or API endpoints
- Add missing program IDs
- Add more examples or error handling patterns

## Pull Request Process

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Fill out the PR template completely
4. For skills: include proof (screenshot/log) that it works
5. For program IDs: include a link to the source (protocol repo or Solana Explorer)

## Code Style

- Skills are documentation — they teach the agent what to do
- Examples should be minimal and copy-paste-ready
- Don't embed private keys in examples
- Keep SKILL.md focused and actionable
