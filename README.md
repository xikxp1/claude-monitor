# Claude Monitor

A lightweight desktop application for monitoring Claude API usage. Runs in the system tray and provides a window to view real-time usage statistics.

## Features

- **System Tray Integration**: Runs quietly in the background with a tray icon
- **Usage Dashboard**: Visual display of API usage metrics
- **Real-time Updates**: Auto-refreshes usage data at configurable intervals
- **Cross-Platform**: Works on macOS, Windows, and Linux
- **Lightweight**: Built with Tauri for minimal resource usage

## Tech Stack

- **Frontend**: SvelteKit 5 + TypeScript
- **Backend**: Rust (Tauri 2)
- **Build Tool**: Vite
- **Package Manager**: Bun

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) (or Node.js 18+)
- Platform-specific dependencies for Tauri:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools, WebView2
  - **Linux**: See [Tauri prerequisites](https://tauri.app/start/prerequisites/)

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/xikxp1/claude-monitor.git
   cd claude-monitor
   ```

2. Install dependencies:
   ```bash
   bun install
   ```

3. Run in development mode:
   ```bash
   bun run tauri dev
   ```

4. Build for production:
   ```bash
   bun run tauri build
   ```

## Configuration

The application requires authentication with Claude API. On first launch, you'll be prompted to configure:

- **Organization ID**: Your Claude organization identifier
- **Session Token**: Authentication token from claude.ai

## API Endpoint

The app fetches usage data from:
```
https://claude.ai/api/organizations/{org_id}/usage
```

## Project Structure

```
claude-monitor/
├── src/                    # Frontend (SvelteKit)
│   ├── routes/            # Page components
│   └── lib/               # Shared utilities
├── src-tauri/             # Backend (Rust/Tauri)
│   ├── src/
│   │   ├── lib.rs         # Main Tauri logic
│   │   └── main.rs        # Entry point
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── static/                # Static assets
└── package.json           # Frontend dependencies
```

## Development

### Running Tests

```bash
# Frontend tests (Vitest)
bun run test

# Frontend tests in watch mode
bun run test:watch

# Rust tests
cd src-tauri && cargo test
```

### Code Style

- Frontend: Biome
- Backend: rustfmt + clippy

## License

MIT
