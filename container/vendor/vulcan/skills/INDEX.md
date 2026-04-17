# Skills Index

20 agent skills for `vulcan`, organized by category.

## Core

Shared runtime contract, safety rules, risk management, and error recovery.

| Skill | Description |
|-------|-------------|
| [vulcan-shared](./vulcan-shared/SKILL.md) | Auth, invocation contract, symbol format, size units, and safety rules. |
| [vulcan-risk-management](./vulcan-risk-management/SKILL.md) | Pre-trade risk checks, leverage tiers, margin health, and when to warn. |
| [vulcan-error-recovery](./vulcan-error-recovery/SKILL.md) | Error category routing, tx_failed recovery, and network error handling. |

## Trading

Order execution, lot size calculation, TP/SL management, and execution strategies.

| Skill | Description |
|-------|-------------|
| [vulcan-trade-execution](./vulcan-trade-execution/SKILL.md) | Safe order execution with pre-trade checks and post-trade verification. |
| [vulcan-lot-size-calculator](./vulcan-lot-size-calculator/SKILL.md) | Convert desired token amounts to base lots with worked examples. |
| [vulcan-tpsl-management](./vulcan-tpsl-management/SKILL.md) | Take-profit and stop-loss: direction rules, constraints, set/cancel flows. |
| [vulcan-twap-execution](./vulcan-twap-execution/SKILL.md) | Execute large orders as time-weighted slices to reduce market impact. |
| [vulcan-grid-trading](./vulcan-grid-trading/SKILL.md) | Grid trading with layered limit orders across a price range. |

## Market Data

Price reads, orderbook analysis, and pre-trade research.

| Skill | Description |
|-------|-------------|
| [vulcan-market-intel](./vulcan-market-intel/SKILL.md) | Ticker, orderbook, candles, market info, and pre-trade analysis patterns. |

## Portfolio & Account

Margin operations, portfolio monitoring, and onboarding.

| Skill | Description |
|-------|-------------|
| [vulcan-portfolio-intel](./vulcan-portfolio-intel/SKILL.md) | Portfolio snapshot: margin status, positions, orders, and funding rates. |
| [vulcan-margin-operations](./vulcan-margin-operations/SKILL.md) | Deposit, withdraw, transfer, isolated margin, and collateral management. |
| [vulcan-onboarding](./vulcan-onboarding/SKILL.md) | New user setup: wallet creation, registration, first deposit. |

## Position

Position monitoring and management.

| Skill | Description |
|-------|-------------|
| [vulcan-position-management](./vulcan-position-management/SKILL.md) | List, show, close, reduce positions and attach TP/SL post-hoc. |

## Recipes

Multi-step workflows combining multiple skills.

| Skill | Description |
|-------|-------------|
| [recipe-emergency-flatten](./recipe-emergency-flatten/SKILL.md) | Cancel all orders and close all positions across all markets. |
| [recipe-open-hedged-position](./recipe-open-hedged-position/SKILL.md) | Open a position with TP/SL protection in one complete flow. |
| [recipe-morning-portfolio-check](./recipe-morning-portfolio-check/SKILL.md) | Daily portfolio review with margin, positions, and funding rates. |
| [recipe-scale-into-position](./recipe-scale-into-position/SKILL.md) | Add to an existing position in calculated increments. |
| [recipe-funding-rate-harvest](./recipe-funding-rate-harvest/SKILL.md) | Scan markets for favorable funding rates and open positions. |
| [recipe-close-and-withdraw](./recipe-close-and-withdraw/SKILL.md) | Close all positions and withdraw collateral to wallet. |
