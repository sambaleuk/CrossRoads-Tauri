# CLAUDE.md — XRoads Tauri Development Guide

## Project Overview

**XRoads Tauri** is the cross-platform (Windows/Linux/macOS) port of XRoads, a native multi-agent AI coding orchestrator. It uses Tauri 2.x (Rust backend) + React/TypeScript (frontend).

## Tech Stack

- **Shell**: Tauri 2.x (Rust)
- **Frontend**: React 19 + TypeScript 5.x + Vite
- **State**: Zustand
- **UI**: Tailwind CSS + shadcn/ui
- **Database**: SQLite via `rusqlite` (Rust side, exposed via Tauri commands)
- **Terminal**: xterm.js (PTY via Rust `portable-pty`)
- **IPC**: Tauri commands (Rust ↔ TypeScript)

## Architecture

```
src-tauri/src/         # Rust backend
  commands/            # Tauri #[command] handlers
  services/            # Business logic (git, process, mcp, db)
  models/              # Rust structs (serde)
  db/                  # SQLite migrations + queries

src/                   # React frontend
  components/          # Reusable UI components
  views/               # Page-level views (Dashboard, Cockpit, Settings)
  services/            # Frontend services (Tauri invoke wrappers)
  models/              # TypeScript interfaces (mirror Rust structs)
  stores/              # Zustand stores (state management)
```

## Key Patterns

### Rust ↔ TypeScript Communication
All Rust functions exposed via `#[tauri::command]`:
```rust
#[tauri::command]
async fn create_session(project_path: String) -> Result<CockpitSession, String> { ... }
```
Called from TypeScript:
```typescript
const session = await invoke<CockpitSession>('create_session', { projectPath: '/path' });
```

### Database Access
SQLite is managed entirely in Rust. Frontend never touches DB directly.
All queries go through Tauri commands.

### Process Management
PTY processes spawned in Rust via `portable-pty`.
Output streamed to frontend via Tauri events.

### State Management
Zustand stores on frontend. Backend is the source of truth (SQLite).
Frontend polls or subscribes to Tauri events for updates.

## Build & Run

```bash
# Dev mode
cargo tauri dev

# Build for distribution
cargo tauri build

# Run tests
cargo test                    # Rust tests
npm test                      # Frontend tests
```

## File Naming

- Rust: `snake_case.rs`
- TypeScript: `camelCase.ts` for services, `PascalCase.tsx` for components
- Models: same field names in Rust (snake_case) and TypeScript (camelCase) via serde rename

## Modragor Integration

This project uses Modragor triple source of truth:
- `model.json` — Entity model (8 entities, 6 enums)
- `states.json` — State machines (3 machines)
- `prd-*.json` — User stories for implementation

PRDs are numbered 01-12 and must be implemented in order (dependency chain).
