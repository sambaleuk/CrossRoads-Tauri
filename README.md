# XRoads

**Give it a PRD. Get code on main.**

XRoads runs 6 AI coding agents in parallel on your codebase. Each agent gets its own git worktree, its own branch, its own terminal. They write code, run tests, and when they're done, XRoads merges everything back. You review the PR. That's it.

No configuration theater. No AI org charts. No "CEO agent delegates to CTO agent." Just code shipped while you sleep.

![Build Status](https://github.com/sambaleuk/CrossRoads-Tauri/actions/workflows/build.yml/badge.svg)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-green)

---

## What it actually does

1. You load a PRD (list of user stories with dependencies)
2. XRoads figures out which stories can run in parallel
3. It spins up agents (Claude, Gemini, Codex, or your own) in isolated git worktrees
4. Each agent writes code, runs tests, reports progress
5. When a layer finishes, XRoads merges and starts the next
6. You get a branch with working code. Merge it.

**The only metric that matters: stories shipped per hour.**

---

## What makes it different

### It ships code, not dashboards

Every other "orchestrator" gives you a pretty UI to watch agents think about thinking. XRoads gives you merged branches with passing tests.

### It learns from your codebase

After each run, XRoads trains lightweight ML models on your project's data. Which agent is fastest on Rust backend stories? Which model is cheapest for simple UI tasks? Next time, it routes automatically. No configuration needed.

### It prevents problems before they happen

Before dispatching two agents to overlapping files, XRoads predicts the merge conflict and resequences. Before an agent runs `rm -rf` or `git push --force`, XRoads suspends the process and asks you. Immutable audit trail on every dangerous operation.

### It works with any agent

Claude Code, Gemini CLI, Codex CLI built in. But anything that can read stdin and write stdout works. Python script? Docker container? HTTP webhook? Ship it.

---

## Quick Start

```bash
git clone https://github.com/sambaleuk/CrossRoads-Tauri.git
cd CrossRoads-Tauri
npm install
npm run tauri dev
```

Or build for production:
```bash
npm run tauri build
```

---

## How it works under the hood

```
Your PRD (stories + dependencies)
        │
        ▼
┌─────────────────┐
│  Layer Builder   │  Groups stories by dependency. Layer 0 = no deps.
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Conflict Check   │  ML predicts file overlaps. Resequences if risky.
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────────┐
│              Parallel Dispatch               │
│                                             │
│  Slot 0: claude ──► feat/auth-api           │
│  Slot 1: claude ──► feat/auth-ui            │
│  Slot 2: gemini ──► feat/auth-tests         │
│                                             │
│  Each slot = own worktree, own terminal,    │
│  own AGENT.md with context injection        │
└────────┬────────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│  Merge & Next    │  Coordinate merge. Start next layer. Repeat.
└────────┬────────┘
         │
         ▼
   Code on main.
```

---

## Safety

Every line of agent terminal output is scanned for dangerous operations:

| Pattern | Risk | Action |
|---------|------|--------|
| `rm -rf /` | Critical | Suspend process, require approval |
| `git push --force` | High | Suspend process, require approval |
| `DROP TABLE` | Critical | Suspend process, require approval |
| `chmod 777` | High | Suspend process, require approval |
| `curl \| bash` | Critical | Suspend process, require approval |

13 patterns total. Process suspension via SIGSTOP/SIGCONT — the agent literally freezes mid-execution until you approve or reject. Every gate decision is recorded in an immutable audit trail.

---

## Intelligence

XRoads trains 3 ML models locally after each orchestration run. No data leaves your machine.

| Model | Purpose | Method |
|-------|---------|--------|
| LinearRegression | Estimate story completion time | OLS with ridge regularization |
| NaiveBayes | Categorize stories by code domain | Laplace-smoothed log probabilities |
| DecisionTree | Predict merge conflicts | Info-gain decision stump |

Plus:
- **Persistent agent memory** — "Claude struggled with TypeScript generics last Tuesday. Route those stories to Gemini."
- **Trust scoring** — Per-agent, per-domain success rate. High trust = auto-merge to staging.
- **Cost-aware model routing** — Budget tight? Auto-downgrade to Sonnet. Story simple? Use Haiku. Complex? Opus.

---

## Tech Stack

| Layer | Tech |
|-------|------|
| Backend | Rust + Tauri 2 + SQLite (21 tables) |
| Frontend | React 19 + TypeScript + Tailwind |
| Terminal | xterm.js + portable-pty |
| ML | Pure Rust (zero external deps) |
| Agents | Bash loop scripts (nexus-loop, gemini-loop, codex-loop) |
| CI/CD | GitHub Actions (macOS, Windows, Linux) |

---

## The numbers

- 161 unit tests + 19 E2E integration tests
- 21 SQLite tables across 17 migrations
- 18 Rust services
- 122 Tauri IPC commands
- 3 on-device ML models
- 4 loop scripts (1,337 lines of real agent execution)
- Builds for macOS (arm64 + x64), Windows, Linux

---

## What we don't do

- **We don't organize AI into fake human org charts.** AI isn't a company with a CEO.
- **We don't do "setup porn."** No spend-3-hours-configuring-before-doing-anything.
- **We don't pretend agents are autonomous.** They write code. You review it. That's the deal.
- **We don't send your code to the cloud for ML.** Everything trains locally.

---

## Cross-platform

XRoads ships as a native desktop app:
- **macOS**: Native SwiftUI version at [sambaleuk/CrossRoads](https://github.com/sambaleuk/CrossRoads)
- **Windows/Linux/macOS**: This Tauri version

Same features, same agent scripts, same ML models. Pick your platform.

---

## Roadmap

**Now**: Ship code faster with parallel agents + ML-driven routing.

**Next**: CI/CD auto-fix loop (agent fixes your failing build automatically), event-driven triggers (start orchestration on git push), consensus-based development (3 agents write the same story, merge the best).

**Later**: Cloud execution for scale beyond 6 local slots. Plugin ecosystem. A2A protocol support.

---

## Contributing

```bash
cd src-tauri
cargo test -- --test-threads=1  # Run all tests
cargo check                      # Quick compilation check
```

PRs welcome. The bar is: does it help ship code faster?

---

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.

---

Built by [Neurogrid](https://neurogrid.me) — Open source under Apache 2.0.
