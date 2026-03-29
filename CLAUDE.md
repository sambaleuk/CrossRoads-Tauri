# CLAUDE.md — XRoads Tauri Development Guide

## Project Overview

**XRoads Tauri** is the cross-platform (Windows/Linux/macOS) port of XRoads, a multi-agent AI coding orchestrator. Coordinates Claude, Gemini, and Codex agents working in parallel on isolated git worktrees with on-device ML intelligence.

## Tech Stack

- **Shell**: Tauri 2.x (Rust)
- **Frontend**: React 19 + TypeScript 5.x + Vite 6
- **State**: Zustand
- **UI**: Tailwind CSS + custom theme (CSS variables)
- **Database**: SQLite via `rusqlite` (21 tables, 17 migrations)
- **Terminal**: xterm.js (PTY via Rust `portable-pty`)
- **ML**: Pure Rust (LinearRegression, NaiveBayes, DecisionTree)
- **IPC**: Tauri commands (95+ Rust ↔ TypeScript)
- **CI/CD**: GitHub Actions (macOS arm64/x64, Windows, Linux)

## Architecture

```
src-tauri/src/
  commands/mod.rs        # 95+ Tauri #[command] handlers
  db/                    # 21 tables, 17 migrations, 16 repos
    manager.rs           # SQLite + migration engine
    *_repo.rs            # Repository per entity
  services/              # 18 business logic services
    agent_lifecycle.rs   # Spawn/monitor/failover/abort
    orchestration_engine.rs  # PRD parse, layers, dispatch
    safe_executor.rs     # Danger detection, SIGSTOP/SIGCONT
    ml_trainer.rs        # On-device ML (3 models)
    learning_engine.rs   # Adaptive assignment, retro
    conflict_prevention.rs   # Predictive conflict prevention
    cockpit_logic.rs     # Session state machine
    event_bus.rs         # Tauri event emission
    org_chart.rs         # Role hierarchy
    budget_engine.rs     # Cost governance
    heartbeat_engine.rs  # Code-aware pulses
    mcp_service.rs       # JSON-RPC 2.0 MCP client
    skill_system.rs      # Skill loader + CLI adapters
    session_persistence.rs   # Crash recovery
  models/                # Rust structs (serde)

src/                     # React frontend
  components/            # NeonBrain, SlotTerminal, CommandPalette, etc.
  views/                 # Dashboard, CockpitPanel, ChatPanel, GitPanel, Settings
  services/api.ts        # Tauri invoke wrappers (95+ functions)
  services/eventBus.ts   # Event subscriptions + log buffer
  stores/appStore.ts     # Zustand state
  models/index.ts        # TypeScript interfaces
```

## Key Patterns

### Rust ↔ TypeScript Communication
```rust
#[tauri::command]
fn my_command(param: String) -> Result<MyStruct, String> { ... }
```
```typescript
const result = await invoke<MyStruct>('my_command', { param: 'value' });
```

### Database Access
Global SQLite via `with_db()`. All repos follow same pattern:
```rust
pub fn create_thing(...) -> Result<Thing> {
    with_db(|conn| { /* INSERT, return struct */ })
}
```

### Event Bus
Events emitted via `event_bus::emit_*()`, received in frontend via `listen()`.

### ML Models
Trained after orchestration from LearningRecords. Persist as JSON in `.crossroads/ml/`.

## Build & Run

```bash
npm install              # Frontend deps
npm run tauri dev        # Dev mode (hot reload)
npm run tauri build      # Production build

cd src-tauri
cargo test -- --test-threads=1   # Run 159+ tests
cargo check              # Quick compilation check
```

## Database Tables (21)

| Migration | Tables |
|-----------|--------|
| v1 | cockpit_session, agent_slot |
| v2 | metier_skill |
| v3 | agent_message |
| v4 | agent_slot.currentTask (ALTER) |
| v5 | execution_gate |
| v6 | cost_event |
| v7 | agent_metrics |
| v8 | orchestration_record |
| v9 | org_role |
| v10 | budget_config, budget_alert |
| v11 | heartbeat_config, scheduled_run |
| v12 | workspace, agent_runtime |
| v13 | config_snapshot |
| v14 | learning_record, performance_profile |
| v15 | (reserved) |
| v16 | agent_memory |
| v17 | trust_score |

## Loop Scripts

Real agent loop scripts in `scripts/`:
- `nexus-loop` — Claude Code agent loop
- `gemini-loop` — Gemini CLI agent loop
- `codex-loop` — Codex agent loop
- `lib/common.sh` — Shared library

## Testing

- 159+ Rust unit tests (repos, services, ML models)
- 19 E2E integration tests (full workflows)
- `cargo test -- --test-threads=1` (required: shared global DB)

## File Naming

- Rust: `snake_case.rs`
- TypeScript: `camelCase.ts` for services, `PascalCase.tsx` for components
- Models: same field names via serde `rename_all = "camelCase"`

## Modragor Integration

Triple source of truth:
- `model.json` — Entity model (21 entities, 14 enums)
- `states.json` — State machines (3 machines)
- `prd-*.json` — User stories (41 PRDs implemented)
