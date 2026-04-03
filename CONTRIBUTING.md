# Contributing to XRoads (Tauri)

Thank you for your interest in contributing to XRoads!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/CrossRoads-Tauri.git`
3. Create a branch: `git checkout -b feat/my-feature`
4. Make your changes
5. Build and test
6. Commit and push
7. Open a Pull Request

## Development Setup

- **Rust** (latest stable)
- **Node.js 18+**
- **Tauri CLI**: `cargo install tauri-cli`

### Linux dependencies
```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev
```

### Build & Run
```bash
npm install
cargo tauri dev        # development mode
cargo tauri build      # production build
cargo test             # Rust tests
npm run build          # frontend build
```

## Architecture

- `src/` — React + TypeScript frontend (Zustand, Tailwind, xterm.js)
- `src-tauri/src/` — Rust backend (Tauri commands, SQLite, services)
- `scripts/` — Agent loop scripts (bash)

## Code Style

- **Rust**: Follow standard Rust conventions, use `cargo clippy`
- **TypeScript**: Follow existing patterns, use functional components
- **CSS**: Tailwind utility classes, custom styles in `index.css`

## What to Contribute

- Bug fixes
- New agent integrations (beyond Claude, Gemini, Codex)
- UI/UX improvements
- Cross-platform fixes (Windows, Linux)
- Documentation and translations
- Tests

## Pull Request Process

1. Ensure `cargo check` and `npm run build` pass
2. Add tests for new functionality
3. Keep PRs focused
4. Write clear commit messages

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.
