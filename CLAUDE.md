# CLAUDE.md - Development Guidelines

This document provides context for AI assistants working on this project.

## Project Overview

Claude Monitor is a Tauri 2 desktop application that monitors Claude API usage. It runs in the system tray and displays usage statistics in a dashboard window.

Always use `bun` commands instead of `npm` commands.

Always use `bun add <package>` to add a new frontend dependency. Don't edit `package.json` manually.
Always use `cargo add <package>` in `src-tauri/` folder to add a new backend dependency. Don't edit `Cargo.toml` manually.

Always use `bun run check` to check the code after making changes.

Always consult PLAN.md to see the current state of the project and the planned features. Update PLAN.md as you make changes.
Always consult ARCHITECTURE.md to see the current architecture of the project. Update ARCHITECTURE.md as you make changes.

## Commands

```bash
bun run dev          # Start Vite dev server only
bun run tauri dev    # Start full Tauri app in dev mode
bun run tauri build  # Build production app
bun run lint         # Run Biome lint
bun run check        # Type-check frontend
```

## Code Conventions

### Svelte/TypeScript
- Use Svelte 5 runes, not legacy reactive statements
- Prefer `$state()` over stores for component state
- Use TypeScript strict mode
- Keep components under 200 lines

### Rust
- Use `thiserror` for custom errors
- Async commands with `#[tauri::command]`
- Handle all Results, no unwrap in production code
- Use `serde` for JSON serialization

## Testing Strategy

- **Frontend**: Vitest for unit tests, Playwright for e2e
- **Backend**: Cargo test for Rust unit tests
- **Integration**: Tauri's testing utilities

## Security Considerations

- Never log or expose session tokens
- Validate all API responses
- Use CSP headers appropriately
- Session tokens should only be stored in secure storage

## Dependencies to Know

### Frontend
- `@tauri-apps/api` - Tauri IPC
- `@tauri-apps/plugin-*` - Official Tauri plugins

### Backend (Cargo.toml)
- `tauri` - Core framework
- `reqwest` - HTTP client (to add)
- `serde`/`serde_json` - Serialization
- `tokio` - Async runtime (via Tauri)
