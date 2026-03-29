# XRoads -- Multi-Agent AI Coding Orchestrator

**The orchestrator that learns your code patterns and gets smarter every session.**

XRoads is a desktop application that dispatches multiple AI coding agents in parallel, coordinates their work across git worktrees, and merges results with conflict prevention -- all from a single control surface. Unlike wrappers that hide the agent behind a chat interface, XRoads treats agents as first-class workers with memory, trust scores, and performance profiles that persist across sessions.

---

**Key differentiators:**

- **Parallel execution with dependency awareness** -- 6-slot dispatch with layer-based ordering, not sequential prompting.
- **On-device machine learning** -- Pure Rust ML models (LinearRegression, NaiveBayes, DecisionTree) that learn from your project's history. No cloud dependency.
- **Agent-agnostic orchestration** -- Claude Code, Gemini CLI, Codex CLI built-in. Any runtime that speaks the XAP Protocol works out of the box.
- **Safety-first architecture** -- 13 dangerous operation patterns detected before execution. Immutable audit trail. Process suspension for approval gates.

![Build Status](https://github.com/sambaleuk/CrossRoads-Tauri/actions/workflows/build.yml/badge.svg)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-TBD-lightgrey)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-green)

---

## Screenshot

> **NeonBrain Dashboard** -- The main interface features a hexagonal SVG brain visualization with 6 radial agent slots, each connected to its own xterm.js terminal. Slots pulse with activity status; synapse connections animate data flow between agents. The cockpit panel provides real-time orchestration controls, while the command palette (Ctrl+K) offers instant access to every operation.

*Screenshot placeholder: `docs/screenshots/neonbrain-dashboard.png`*

---

## Features

### Core Orchestration

- **6-slot parallel dispatch** -- Agents run simultaneously in isolated git worktrees. Dependency-aware layer system ensures correct execution order.
- **PRD-driven execution** -- Parse a PRD into story layers, generate dispatch plans, and execute across agents with automatic progress tracking.
- **Resume mode** -- Interrupted orchestrations persist to SQLite. Detect recoverable sessions and resume from the last checkpoint.
- **Merge coordination** -- Git worktree creation, branch management, and coordinated merging handled by the orchestration engine.

### Agent Support

- **Built-in runtimes** -- Claude Code, Gemini CLI, and Codex CLI detected and configured automatically.
- **XAP Protocol** -- Extensible Agent Protocol supporting CLI, HTTP, Docker, Script, and Stdio runtime types. Register any agent that conforms to the protocol.
- **Agent lifecycle management** -- Spawn, monitor, failover, and abort agents. Health checks with configurable intervals and automatic recovery.

### Intelligence

- **On-device ML** -- Three pure Rust models (LinearRegression, NaiveBayes, DecisionTree) trained on your project data. No external API calls.
- **Persistent Agent Memory** -- Agents accumulate knowledge across sessions. Memory is indexed, searchable, and scoped to project context.
- **Trust Scoring** -- Each agent builds a trust score based on test pass rates, merge success, and review outcomes. Configurable auto-merge policies for trusted agents.
- **Predictive Conflict Prevention** -- Analyzes planned file modifications across slots before dispatch. Prevents merge conflicts before they happen.
- **Adaptive Assignment** -- Performance profiles track agent strengths by task type. The learning engine recommends the best agent for each story.
- **Cost-aware Routing** -- Budget engine with per-slot caps, cost projections, and auto-throttle when spending approaches limits.

### Security

- **SafeExecutor** -- 13 dangerous operation patterns (rm -rf, DROP TABLE, force push, etc.) detected and blocked before execution.
- **Process Suspension** -- SIGSTOP/SIGCONT used to pause agent processes pending human approval through execution gates.
- **Immutable Audit Trail** -- Every gate decision, cost event, and orchestration action recorded with timestamps and actor identity.
- **Authority-based Gate Routing** -- Approval requests routed through the organization hierarchy. Role-appropriate escalation.

### Organization

- **Org chart with role hierarchy** -- CEO, Lead, Engineer, QA roles with configurable authority levels. Template-based setup.
- **Goal cascading** -- Project-level objectives cascade to team goals, then to individual story assignments.
- **Budget control** -- Per-slot and per-session cost caps with real-time projections and automatic throttling.
- **Config versioning** -- Immutable configuration snapshots with point-in-time rollback.

### Monitoring

- **Code-aware heartbeat** -- Monitors git diff output, test results, and story progress. Detects stalled agents.
- **Scheduled orchestration runs** -- Trigger orchestrations on cron schedules, git push events, or file change detection.
- **Real-time event bus** -- PTY output, agent status changes, log entries, gate events, and cost updates streamed to the frontend via Tauri events.
- **Crash recovery** -- Session state persisted continuously. Orphaned worktrees detected and cleaned up on restart.

### User Interface

- **NeonBrain dashboard** -- SVG visualization with hexagonal slot layout and animated synapse connections showing inter-agent data flow.
- **Integrated terminals** -- xterm.js terminal per slot with full ANSI support, powered by Rust PTY via portable-pty.
- **Command palette** -- Ctrl+K access to all operations with fuzzy search.
- **Multi-project workspaces** -- Isolated workspace contexts with independent sessions, budgets, and agent configurations.
- **Settings** -- 7-tab configuration panel (General, CLI, API Keys, Budget, Runtimes, Advanced, Skills).
- **Cockpit panel** -- Real-time orchestration controls with activate, pause, resume, and close operations.
- **Git panel** -- Branch visualization, recent commits, and merge coordination from within the app.

---

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable toolchain)
- Platform dependencies: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Install and Run

```bash
# Clone the repository
git clone https://github.com/sambaleuk/CrossRoads-Tauri.git
cd CrossRoads-Tauri

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

---

## Architecture

```
src-tauri/src/                  # Rust backend (source of truth)
  main.rs                       # App bootstrap, 122 Tauri IPC commands
  db/
    manager.rs                  # SQLite init, WAL mode, 17 migrations
    *_repo.rs                   # 15 repository modules (CRUD per table)
  services/
    orchestration_engine.rs     # PRD parsing, layer dispatch, merge coordination
    agent_lifecycle.rs          # Spawn, monitor, failover, abort
    safe_executor.rs            # Dangerous operation detection + blocking
    learning_engine.rs          # Performance profiles, agent recommendation
    ml_trainer.rs               # LinearRegression, NaiveBayes, DecisionTree
    conflict_prevention.rs      # Pre-dispatch file conflict analysis
    budget_engine.rs            # Cost caps, projections, auto-throttle
    session_persistence.rs      # Crash recovery, session state snapshots
    event_bus.rs                # Real-time event streaming to frontend
    cockpit_logic.rs            # Orchestration state machine
    git_service.rs              # Worktree management, branch ops, merging
    process_runner.rs           # PTY process spawning
    mcp_service.rs              # MCP client/server, handoff generation
    skill_system.rs             # Skill scanning, registration, injection
    org_chart.rs                # Role hierarchy, goal cascading
    heartbeat_engine.rs         # Health monitoring, scheduled runs
    cli_detector.rs             # Auto-detect installed CLI agents
    loop_launcher.rs            # Iteration loop management
  models/                       # Rust structs with serde serialization
  commands/                     # Tauri command handlers (thin layer over services)

src/                            # React 19 frontend
  views/
    Dashboard.tsx               # NeonBrain visualization + slot terminals
    CockpitPanel.tsx            # Orchestration controls
    ChatPanel.tsx               # Agent communication
    GitPanel.tsx                # Branch and merge visualization
    SettingsPanel.tsx           # 7-tab configuration
    SkillsBrowser.tsx           # Skill discovery and management
  components/
    NeonBrain.tsx               # Hexagonal SVG brain visualization
    SlotTerminal.tsx            # xterm.js terminal per agent slot
    CommandPalette.tsx          # Ctrl+K command launcher
    SynapseConnections.tsx      # Animated inter-slot connections
    Toolbar.tsx                 # Top bar + bottom status bar
  stores/                       # Zustand state management
  services/                     # Tauri invoke wrappers + event bus
  models/                       # TypeScript interfaces (mirror Rust structs)
```

### Data Flow

```
User Action --> React (Zustand) --> invoke() --> Tauri Command --> Service --> SQLite
                    ^                                                |
                    |                                                v
                    +--- Tauri Event <--- Event Bus <--- Agent PTY Output
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust + Tauri 2.x |
| Database | SQLite (rusqlite) with WAL mode |
| Frontend | React 19 + TypeScript 5.6 |
| Styling | Tailwind CSS 3.4 |
| State | Zustand 5 |
| Terminal | xterm.js 5.5 + portable-pty 0.8 |
| ML | Pure Rust (no Python, no external deps) |
| Build | Vite 6 |
| CI/CD | GitHub Actions |
| Icons | Lucide React |

---

## Database Schema

21 SQLite tables managed through 17 versioned migrations:

| Table | Purpose |
|-------|---------|
| `_migrations` | Schema version tracking |
| `cockpit_session` | Orchestration session state and metadata |
| `agent_slot` | Per-slot agent configuration and status |
| `metier_skill` | Registered skills with CLI adaptations |
| `agent_message` | Inter-agent and agent-user message history |
| `execution_gate` | Approval gates with status and reviewer |
| `cost_event` | Token usage and cost tracking per slot |
| `agent_metrics` | Stories completed, failed, pass rates |
| `orchestration_record` | PRD execution history and progress |
| `org_role` | Organization hierarchy and authority levels |
| `budget_config` | Per-slot and per-session cost limits |
| `budget_alert` | Budget threshold breach notifications |
| `heartbeat_config` | Agent health check configuration |
| `scheduled_run` | Cron and trigger-based orchestration schedules |
| `workspace` | Multi-project workspace isolation |
| `agent_runtime` | Registered agent runtimes (CLI, HTTP, Docker, etc.) |
| `config_snapshot` | Immutable configuration snapshots for rollback |
| `learning_record` | Per-story execution data for ML training |
| `performance_profile` | Aggregated agent performance by task type |
| `agent_memory` | Persistent cross-session agent knowledge |
| `trust_score` | Agent trust levels and auto-merge thresholds |

---

## Testing

- **178 unit tests** across 38 modules (Rust)
- **19 end-to-end integration tests** covering cross-service workflows
- All tests use in-memory SQLite for isolation

```bash
# Run the full test suite
cd src-tauri && cargo test -- --test-threads=1

# Run a specific service's tests
cargo test safe_executor -- --test-threads=1

# Run integration tests only
cargo test e2e -- --test-threads=1
```

Tests run single-threaded (`--test-threads=1`) because they share an in-memory SQLite connection.

---

## CI/CD

GitHub Actions pipeline defined in `.github/workflows/build.yml`:

1. **Test** -- Rust test suite on Ubuntu (every push to `main` and every PR)
2. **Build** -- Cross-platform builds on 4 targets:
   - macOS (Apple Silicon -- aarch64)
   - macOS (Intel -- x86_64)
   - Windows (x86_64)
   - Linux (x86_64)
3. **Release** -- Automatic release creation on tag push (`v*`) with platform artifacts and generated changelog

---

## Comparison with Alternatives

| Capability | XRoads | Claude Code | Cursor | Codex CLI | CrewAI |
|------------|--------|-------------|--------|-----------|--------|
| Multi-agent parallel execution | 6 slots, dependency-aware layers | Single agent | Single agent | Single agent | Multi-agent, Python |
| Agent-agnostic | Any CLI/HTTP/Docker via XAP | Claude only | Claude/GPT | OpenAI only | Custom Python agents |
| On-device ML | Rust-native, no cloud | None | None | None | Python ML optional |
| Persistent memory | Cross-session, searchable | Per-conversation | Per-project | Per-session | Custom implementation |
| Trust scoring + auto-merge | Built-in with configurable policies | None | None | None | None |
| Conflict prevention | Pre-dispatch analysis | None | None | None | None |
| SafeExecutor | 13 pattern detection + SIGSTOP gates | Approval prompts | None | Approval prompts | None |
| Git worktree isolation | Automatic per-slot | Manual | Manual | Manual | None |
| Budget control | Per-slot caps, projections, throttle | None | Subscription | API cost tracking | None |
| Org hierarchy + goal cascading | Built-in | None | None | None | Role-based |
| Desktop app | Native (Tauri) | Terminal | Electron | Terminal | Web/Terminal |

---

## Roadmap

### Phase 1 -- Active Development

- CI/CD auto-fix loop (failed tests trigger agent re-dispatch)
- Event-driven automation triggers (file change, git push, test failure)
- A2A Protocol support for inter-agent communication
- Full UI implementation for all backend capabilities

### Phase 2 -- Platform Expansion

- Cloud execution mode (remote agent dispatch)
- Plugin ecosystem with XAP marketplace
- Shared team memory and trust scores
- Multi-repository orchestration

### Phase 3 -- Ecosystem Integration

- IDE extensions (VS Code, JetBrains)
- Voice mode for hands-free orchestration
- Custom model fine-tuning from learning data
- Enterprise SSO and audit compliance

---

## Contributing

Contributions are welcome. To get started:

1. Fork the repository and create a feature branch.
2. Follow existing code patterns -- Rust services in `src-tauri/src/services/`, frontend components in `src/components/`.
3. Add tests for new functionality. Rust tests go in `#[cfg(test)]` modules within the relevant file.
4. Run the test suite before submitting: `cd src-tauri && cargo test -- --test-threads=1`
5. Submit a pull request with a clear description of the change and its motivation.

### Development Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js 20+ (via nvm recommended)
nvm install 20

# Install Tauri CLI
npm install

# Platform-specific dependencies:
# macOS: Xcode Command Line Tools (xcode-select --install)
# Linux: libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
# Windows: Microsoft C++ Build Tools + WebView2

# Run in development
npm run tauri dev
```

---

## License

License TBD.

---

## Credits

Built by [Neurogrid](https://neurogrid.me).

Co-developed with Claude Opus 4.6 (Anthropic).
