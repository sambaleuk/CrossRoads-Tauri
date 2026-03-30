# XRoads — Complete View Audit & Feature Map

**Date:** 2026-03-30
**Scope:** Both platforms (Swift macOS + Tauri cross-platform)
**Purpose:** Map every backend feature to its UI status, identify gaps

---

## 1. LEGACY VIEWS (Dead Code — Never Rendered)

### Swift — 11 dead files (~3,000 LOC)

| File | LOC | Reason |
|------|-----|--------|
| `ContentView.swift` | 16 | Placeholder, never routed |
| `GitDashboardView.swift` | ~500 | Complete but never integrated in nav |
| `PRDWizardSteps.swift` | ~200 | Never instantiated |
| `SkillsBrowserView.swift` | ~400 | Complete feature, never wired to sidebar |
| `SkillRowView.swift` | ~80 | Depends on SkillsBrowser (dead) |
| `SkillDetailSheet.swift` | ~150 | Depends on SkillsBrowser (dead) |
| `SkillTemplateView.swift` | ~100 | Preview-only |
| `ArtDirectionView.swift` | ~830 | Complete pipeline, never wired to sidebar |
| `ArtBiblePreviewView.swift` | ~200 | Depends on ArtDirection (dead) |
| `ArtPipelineProgress.swift` | ~150 | Depends on ArtDirection (dead) |
| `AssetPRDPreviewView.swift` | ~200 | Depends on ArtDirection (dead) |

**Action:** Either wire into nav (Skills + ArtDirection are complete) or delete.

### Tauri — 0 dead files
All React components are imported and rendered.

---

## 2. ACTIVE VIEWS — What users actually see

### Swift — 57 active views

**Core Layout (5)**
- MainWindowView, SidebarView, CommandPaletteView, SettingsView, TerminalView

**Dashboard (7)**
- XRoadsDashboardView, NeonBrainView, OrchestratorCreatureView
- TerminalGridLayout, TerminalSlotView, GitInfoPanel, GitMasterPanel

**Orchestrator Chat (5)**
- OrchestratorChatView (CLI + API modes, real Claude conversation)
- ChatInputBar, ChatMessageView, PRDProposalView, ArtBibleProposalView

**Cockpit (10)**
- CockpitModeView, CockpitSlotCardView, ApprovalCardView
- AuditTrailView, ChairmanFeedPanelView, CostBadgeView
- SlotChatPanelView, OrgChartPanelView, BudgetPanelView, HeartbeatPanelView

**PRD (4)**
- PRDAssistantView, PRDPreviewView, PRDLoaderSheet, PRDPreviewSheet

**Settings (7)**
- General, CLI, MCP, APIKeys, Budget, Runtimes, Advanced

**Sheets & Modals (6)**
- SlotAssignment, StartSession, WorktreeCreate
- ConflictResolution, OrchestrationHistory, SlotLogViewer

**Components (14)**
- ActionPickerMenu, CollapsiblePanel, FloatingInputWindow, LoadingIndicators
- LoopConfigurationPanel, MacTextField, ModalPanel, OrchestrationRecoveryBanner
- QuickActionBar, SkillsBadge, SlotStatusBadge, TerminalInputBar, WorktreeCard

### Tauri — 16 active views/components

**Views (7)**
- Dashboard, ChatPanel, CockpitPanel, GitPanel, SettingsPanel, SkillsBrowser

**Components (9)**
- NeonBrain, SynapseConnections, SlotTerminal, TerminalSlot
- StatusBadge, CostBadge, SlotConfigDialog, CommandPalette, Toolbar + BottomBar

---

## 3. BACKEND → FRONTEND MAPPING

### Features implemented BEFORE this weekend (original codebase)

| Feature | Swift Backend | Swift UI | Tauri Backend | Tauri UI |
|---------|:---:|:---:|:---:|:---:|
| Session lifecycle (idle→active→closed) | ✅ | ✅ | ✅ | ✅ |
| 6-slot agent management | ✅ | ✅ | ✅ | ✅ |
| Agent spawning (Claude/Gemini/Codex) | ✅ | ✅ | ✅ | ⚠️ stub |
| PTY terminal per slot | ✅ | ✅ | ✅ | ✅ xterm.js |
| Git worktree isolation | ✅ | ✅ | ✅ | ❌ no UI |
| Merge coordination | ✅ | ✅ | ✅ | ❌ no UI |
| ExecutionGate (approval gates) | ✅ | ✅ ApprovalCard | ✅ | ❌ **no approval UI** |
| Cost tracking per slot/session | ✅ | ✅ CostBadge | ✅ | ✅ CostBadge |
| Chairman deliberation | ✅ | ✅ ChairmanFeed | ✅ | ✅ CockpitPanel |
| Skill loading + injection | ✅ | ❌ dead (SkillsBrowser) | ✅ | ✅ SkillsBrowser |
| MCP client (JSON-RPC) | ✅ | ❌ (backend only) | ✅ | ❌ no UI |
| PRD parser + layer builder | ✅ | ✅ PRDAssistant | ✅ | ❌ **no PRD UI** |
| Chat (CLI + API modes) | ✅ | ✅ OrchestratorChat | ✅ | ✅ ChatPanel |
| Command palette | ✅ | ✅ | ✅ | ✅ |
| Art direction pipeline | ✅ | ❌ dead (ArtDirection) | ❌ | ❌ |
| Orchestration history | ✅ | ✅ HistorySheet | ✅ | ❌ **no history UI** |
| Crash recovery banner | ✅ | ✅ RecoveryBanner | ✅ | ❌ **no recovery UI** |
| Conflict resolution (git) | ✅ | ✅ ConflictSheet | ✅ | ❌ **no conflict UI** |
| Slot assignment dialog | ✅ | ✅ SlotAssignmentSheet | ✅ | ✅ SlotConfigDialog |
| Start session dialog | ✅ | ✅ StartSessionSheet | ❌ | ❌ |
| Worktree creation | ✅ | ✅ WorktreeCreateSheet | ✅ | ❌ no UI |
| Log viewer per slot | ✅ | ✅ SlotLogViewer | ✅ | ❌ no UI |
| Loop script configuration | ✅ | ✅ LoopConfigPanel | ✅ | ❌ no UI |

### Features added THIS WEEKEND (Phase 5 + P0) — Backend + UI status

| Feature | Swift Backend | Swift UI | Tauri Backend | Tauri UI |
|---------|:---:|:---:|:---:|:---:|
| **Org Chart** (roles, templates, goals) | ✅ | ✅ OrgChartPanel | ✅ | ❌ **MISSING** |
| **Budget Control** (caps, throttle, projection) | ✅ | ✅ BudgetPanel | ✅ | ⚠️ Settings only |
| **Heartbeat** (pulse, monitoring) | ✅ | ✅ HeartbeatPanel | ✅ | ⚠️ Settings only |
| **Cost-aware model routing** | ✅ | ❌ **MISSING** | ✅ | ❌ **MISSING** |
| **Multi-project workspaces** | ✅ | ❌ **MISSING** | ✅ | ❌ **MISSING** |
| **Agent-agnostic runtimes** | ✅ | ✅ RuntimesSettings | ✅ | ✅ RuntimesSettings |
| **Config versioning + rollback** | ✅ | ⚠️ mention only | ✅ | ⚠️ mention only |
| **Scheduled runs (cron)** | ✅ | ⚠️ toggle only, **NO schedule tab** | ✅ | ⚠️ Settings only |
| **Learning engine** (categorize, recommend) | ✅ | ❌ **MISSING** | ✅ | ❌ **MISSING** |
| **ML Trainer** (3 models) | ✅ | ⚠️ toggle only | ✅ | ⚠️ toggle only |
| **Persistent Agent Memory** | ✅ | ❌ **MISSING** | ✅ | ❌ **MISSING** |
| **Trust Scoring + Auto-Merge** | ✅ | ❌ **MISSING** | ✅ | ❌ **MISSING** |
| **Predictive Conflict Prevention** | ✅ | ❌ **MISSING** | ✅ | ❌ **MISSING** |

---

## 4. GAPS — Backend present, Frontend MISSING

### CRITICAL (features users need to see)

| # | Feature | Both platforms | What's needed |
|---|---------|:-:|---------------|
| 1 | **Learning Dashboard** | ❌ NO UI | View showing: agent recommendations, task categories, time estimates, performance profiles |
| 2 | **Agent Memory Browser** | ❌ NO UI | Search/browse memories, see what agents remember, delete old memories |
| 3 | **Trust Score Dashboard** | ❌ NO UI | Per-agent per-domain trust scores, auto-merge thresholds, visual trust matrix |
| 4 | **Conflict Prediction View** | ❌ NO UI | Pre-dispatch conflict matrix showing risky story pairs |
| 5 | **Workspace Switcher** | ❌ NO UI | Sidebar/tab to switch between projects |
| 6 | **Config History + Rollback** | ❌ NO UI | Version list with diff viewer and rollback button |
| 7 | **Schedule Manager** | ❌ NO UI | CRUD for cron schedules, trigger types, run history |
| 8 | **Model Routing Display** | ❌ NO UI | Show which model was selected and why (budget pressure indicator) |

### HIGH (Tauri-only gaps — Swift already has these)

| # | Feature | Swift | Tauri | What's needed in Tauri |
|---|---------|:---:|:---:|------------------------|
| 9 | **Approval Gate Cards** | ✅ ApprovalCard | ❌ | Approve/reject UI for SafeExecutor gates |
| 10 | **PRD Loader/Browser** | ✅ PRDLoaderSheet | ❌ | Load PRD from file, preview stories |
| 11 | **Orchestration History** | ✅ HistorySheet | ❌ | List past orchestrations with results |
| 12 | **Crash Recovery Dialog** | ✅ RecoveryBanner | ❌ | "Resume or discard?" on crashed session |
| 13 | **Conflict Resolution** | ✅ ConflictSheet | ❌ | Git merge conflict resolution UI |
| 14 | **Slot Log Viewer** | ✅ SlotLogViewer | ❌ | View iteration logs per slot |
| 15 | **Start Session Dialog** | ✅ StartSessionSheet | ❌ | Proper session start with project picker |
| 16 | **Worktree Management** | ✅ WorktreeCreateSheet | ❌ | Create/manage git worktrees |
| 17 | **Loop Config Panel** | ✅ LoopConfigPanel | ❌ | Configure loop script parameters |

### MEDIUM (Dead features to resurrect in Swift)

| # | Feature | Status | What's needed |
|---|---------|--------|---------------|
| 18 | **Skills Browser** | Dead (4 files, complete) | Wire SkillsBrowserView into SidebarView nav |
| 19 | **Art Direction Pipeline** | Dead (4 files, complete) | Wire ArtDirectionView into SidebarView nav |

---

## 5. PRIORITY IMPLEMENTATION ORDER

### P0 — Must have for demo (8 views, both platforms)
1. Learning Dashboard (recommendations + profiles + retro)
2. Agent Memory Browser (search + recall)
3. Trust Score Dashboard (agent reliability matrix)
4. Conflict Prediction View (pre-dispatch warning)
5. Approval Gate Cards (Tauri only — Swift has it)
6. PRD Loader (Tauri only — Swift has it)
7. Crash Recovery Dialog (Tauri only — Swift has it)
8. Workspace Switcher (both platforms)

### P1 — Important for completeness (5 views)
9. Config History + Rollback UI
10. Schedule Manager (cron CRUD)
11. Model Routing Display
12. Orchestration History (Tauri)
13. Slot Log Viewer (Tauri)

### P2 — Polish (5 views)
14. Start Session Dialog (Tauri)
15. Worktree Management (Tauri)
16. Conflict Resolution Sheet (Tauri)
17. Wire Skills Browser (Swift, already built)
18. Wire Art Direction (Swift, already built)

---

## 6. SUMMARY STATS

| Metric | Swift | Tauri |
|--------|:-----:|:-----:|
| Total view files | 68 | 16 |
| Active views | 57 | 16 |
| Dead/legacy views | 11 | 0 |
| Backend services | 60+ actors | 18 services |
| Backend features with FULL UI | 15 | 8 |
| Backend features with PARTIAL UI | 4 | 4 |
| Backend features with NO UI | 8 | 10 |
| **Frontend coverage** | **~70%** | **~40%** |

**Bottom line:** Both platforms have production-quality backends. Swift covers ~70% visually. Tauri covers ~40%. The gap is 8 shared missing views + 9 Tauri-only missing views = 17 views to build.
