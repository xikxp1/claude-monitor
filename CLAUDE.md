# CLAUDE.md - Development Guidelines

This document provides context for AI assistants working on this project.

## Project Overview

Claude Monitor is a Tauri 2 desktop application that monitors Claude API usage. It runs in the system tray and displays usage statistics in a dashboard window.

Always use `bun` commands instead of `npm` commands.

Always use `bun lint:fix` and `bun check` to lint and check the code after making changes.

Always consult PLAN.md to see the current state of the project and the planned features. Update PLAN.md as you make changes.

## Architecture

### Frontend (SvelteKit 5)
- Located in `src/`
- Uses Svelte 5 runes (`$state`, `$derived`, `$effect`)
- TypeScript for type safety
- Communicates with Rust backend via `@tauri-apps/api`

### Backend (Rust/Tauri 2)
- Located in `src-tauri/`
- Handles API requests to Claude
- Manages system tray and window lifecycle
- Stores configuration securely

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/lib.rs` | Main Tauri application logic, commands |
| `src-tauri/tauri.conf.json` | Tauri configuration (window, permissions) |
| `src/routes/+page.svelte` | Main dashboard UI |
| `src/lib/` | Shared components and utilities |

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

## API Integration

The app fetches from Claude's usage API:
```
GET https://claude.ai/api/organizations/{org_id}/usage
```

Authentication requires a session cookie from claude.ai. The backend handles:
1. Secure storage of credentials
2. HTTP requests with proper headers
3. Parsing and caching responses

## System Tray

The app uses Tauri's tray plugin:
- Left-click: Show/hide main window
- Right-click: Context menu with options
- Icon shows connection status (optional)

## Configuration Storage

User settings stored via `tauri-plugin-store`:
- Organization ID
- Refresh interval
- Window preferences

Sensitive data (session token) stored in OS keychain via `tauri-plugin-keychain` (or equivalent).

## Testing Strategy

- **Frontend**: Vitest for unit tests, Playwright for e2e
- **Backend**: Cargo test for Rust unit tests
- **Integration**: Tauri's testing utilities

## Common Tasks

### Adding a new Tauri command
1. Define function in `src-tauri/src/lib.rs` with `#[tauri::command]`
2. Register in `invoke_handler`
3. Call from frontend with `invoke("command_name", { args })`

### Adding a new route
1. Create `src/routes/[route]/+page.svelte`
2. Add navigation if needed

### Updating tray menu
1. Modify tray setup in `lib.rs`
2. Handle menu events in the event loop

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
