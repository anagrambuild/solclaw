# Vulcan Agent Prompts

Reusable prompt files for AI agents using the Vulcan MCP server to trade on Phoenix Perpetuals DEX.

## Usage

These prompts are agent-agnostic. Include them in your agent's system prompt or reference them as needed:

- **`system.md`** — Core system prompt. Include this in every agent that uses Vulcan MCP tools.
- **`workflows/trade.md`** — Pre-trade checklist and order placement workflow.
- **`workflows/portfolio.md`** — Portfolio overview and position monitoring workflow.
- **`workflows/risk.md`** — Risk management rules and guardrails.

## With Claude Code

Add to your MCP config and reference these files in your system prompt or CLAUDE.md.

## With Other Agents

Include `system.md` content in your agent's system prompt. Reference workflow files as needed for specific tasks.
