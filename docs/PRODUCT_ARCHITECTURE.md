# XRoads — Product Architecture

## Mental Model

Users think in cycles, not screens:

```
PLAN ──► EXECUTE ──► REVIEW ──► LEARN
  ▲                                │
  └────────────────────────────────┘
```

## 5 Spaces

| Space | Question it answers | When user sees it |
|-------|--------------------|--------------------|
| **Orchestrator** | "What do I want to build?" | Left panel, always visible |
| **Dashboard** | "Where are my agents now?" | Center, default view |
| **Cockpit** | "Does anything need me?" | Right panel, toggle |
| **Review** | "What was shipped? Is it good?" | Overlay, on completion |
| **Intelligence** | "Is XRoads getting smarter?" | Via Settings or Cmd+Shift+L |

## Design Principles

1. **Progressive disclosure** — simple surface, infinite depth
2. **Notification-driven cockpit** — shows actions needed, not metrics
3. **Intelligence as subtext** — ML works silently, user sees "faster"
4. **Dashboard as truth** — brain + slots + progress = 90% of the answer
5. **Config ≠ Features** — settings configure, features live in the flow
