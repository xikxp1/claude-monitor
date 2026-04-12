# Architecture

## Frontend (SvelteKit 5)
- Located in `src/`
- Uses Svelte 5 runes (`$state`, `$derived`, `$effect`)
- TypeScript for type safety
- Communicates with Rust backend via `@tauri-apps/api`

## Backend (Rust/Tauri 2)
- Located in `src-tauri/`
- Handles API requests to Claude
- Manages system tray and window lifecycle
- Stores configuration securely

# Provider Integration

The app now supports one active provider at a time:
- `Claude` via `GET https://claude.ai/api/organizations/{org_id}/usage`
- `Codex` via `GET https://chatgpt.com/backend-api/wham/usage`

Each provider is mapped into the same backend model:
```rust
UsageSnapshot {
  provider: ProviderKind,
  windows: Vec<UsageWindow>,
  account_email: Option<String>,
  plan_type: Option<String>,
}
```

This keeps tray updates, notifications, analytics, and the dashboard provider-agnostic.

# Configuration Storage

User settings stored via `tauri-plugin-store`:
- Active provider
- Refresh interval
- Notification rules
- Window preferences

Sensitive data (session token) stored in OS keychain via `keyring` crate.

Codex credentials are not copied into app storage in v1; the app reads the local Codex auth file (`~/.codex/auth.json` or `$CODEX_HOME/auth.json`) on demand.

# Current File Structure

```
claude-monitor/
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── NotificationSettings.svelte  # Notification config UI
│   │   │   └── charts/                       # Phase 8: Analytics charts
│   │   │       └── UsageLineChart.svelte     # Time-series line chart
│   │   ├── composables/                      # Svelte 5 composables
│   │   │   ├── index.ts                      # Re-exports
│   │   │   ├── useAnalytics.svelte.ts        # Analytics state & actions
│   │   │   ├── useSettings.svelte.ts         # Settings, credentials, notifications
│   │   │   ├── useUpdates.svelte.ts          # Auto-update state & actions
│   │   │   └── useUsageData.svelte.ts        # Usage data, events, countdown, visibility handling
│   │   ├── utils/                            # Pure utility functions
│   │   │   ├── index.ts                      # Re-exports
│   │   │   └── formatting.ts                 # Formatting (formatSecondsAgo, formatCountdown, etc.)
│   │   ├── historyStorage.ts                 # Frontend API for history (calls Rust)
│   │   └── types.ts                          # TypeScript types
│   ├── routes/
│   │   ├── +layout.svelte                    # Root layout (imports app.css)
│   │   └── +page.svelte                      # Main dashboard (daisyUI components)
│   ├── app.css                               # Tailwind + daisyUI with custom themes
│   └── app.html
├── src-tauri/
│   ├── src/
│   │   ├── api.rs                            # Provider dispatcher
│   │   ├── api/                             # Provider-specific fetchers
│   │   │   ├── claude.rs                    # Claude web usage API
│   │   │   └── codex.rs                     # Codex auth.json + WHAM usage API
│   │   ├── auto_refresh.rs                   # Background refresh loop
│   │   ├── commands.rs                       # Tauri commands
│   │   ├── error.rs                          # AppError enum
│   │   ├── history.rs                        # SQLite history storage with normalized provider/window rows
│   │   ├── lib.rs                            # Module re-exports and app entry point
│   │   ├── main.rs                           # Entry point
│   │   ├── credentials.rs                    # OS keychain storage (keyring)
│   │   ├── notifications.rs                  # Provider/window keyed notification processing
│   │   ├── tray.rs                           # System tray creation and tooltip
│   │   ├── types.rs                          # Shared provider, usage, and notification data structures
│   │   ├── validation.rs                     # Input validation
│   │   └── wake_detection.rs                 # macOS wake detection (objc2)
│   ├── capabilities/
│   │   └── default.json                      # Permissions
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── static/
├── CLAUDE.md
├── PLAN.md
├── README.md
└── package.json
```

# Technical Notes

## Rust Backend Module Structure
The Rust backend (`src-tauri/src/`) is organized into focused modules:
- `error.rs` - Custom `AppError` enum with thiserror and Serialize
- `types.rs` - All shared data structures (UsageData, Settings, NotificationRule, AppState, etc.)
- `validation.rs` - Input sanitization (session token, org ID format validation)
- `credentials.rs` - OS keychain storage via `keyring` crate (load/save/delete)
- `api.rs` - HTTP client for Claude API
- `notifications.rs` - Notification processing and firing
- `tray.rs` - System tray creation and tooltip updates
- `auto_refresh.rs` - Background refresh loop with tokio (includes notification processing)
- `commands.rs` - Tauri command handlers
- `wake_detection.rs` - macOS wake detection via `objc2` (triggers refresh on system wake)
- `lib.rs` - Module declarations, plugin setup, and app entry point

## Backend Auto-Refresh Architecture
The auto-refresh system is implemented in the Rust backend for reliability:

**State Management:**
```rust
pub struct AppState {
    pub config: Mutex<AutoRefreshConfig>,  // Credentials + settings
    pub restart_tx: watch::Sender<()>,     // Signal to restart loop
}
```

**Event Flow:**
```
Frontend                          Backend (Rust)
─────────────────────────────────────────────────────
                                  App starts → spawns auto_refresh_loop
                                  Loop waits for credentials

invoke("set_credentials")     →   Updates config, sends restart signal
                                  Loop fetches immediately, starts interval

                              ←   emit("usage-updated", { usage, nextRefreshAt })
                              ←   emit("usage-error", { error })

invoke("set_auto_refresh")    →   Updates interval/enabled, restarts loop
invoke("refresh_now")         →   Triggers immediate fetch, resets timer
```

**Benefits over frontend setInterval:**
- Survives window hide/show and webview refreshes
- More reliable timing via tokio runtime
- Centralized error handling and retry logic
- Tray tooltip updates happen automatically

## Notification System
- Three notification types that can be combined:
  1. **Interval**: Fires every X% (e.g., at 10%, 20%, 30%...)
  2. **Threshold**: Fires once when crossing specific values (e.g., 80%, 90%)
  3. **Time-Remaining**: Fires when less than X minutes until reset (e.g., 30min, 60min before reset)
- Each usage type (5h, 7d, Sonnet, Opus) has independent settings
- State tracking prevents duplicate notifications
- State auto-resets when usage drops significantly (> 20% decrease)
- Settings persisted in `settings.json` via `tauri-plugin-store`
- Permissions: `notification:default`, `notification:allow-notify`, `notification:allow-is-permission-granted`, `notification:allow-request-permission`

## Provider Mapping

### Claude
- Uses stored `organization_id` + `sessionKey`
- Maps Claude windows directly into generic windows:
  - `five_hour`
  - `seven_day`
  - `seven_day_sonnet`
  - `seven_day_opus`

### Codex
- Reads `tokens.access_token` from `~/.codex/auth.json` or `$CODEX_HOME/auth.json`
- Calls `GET https://chatgpt.com/backend-api/wham/usage`
- Maps:
  - `rate_limit.primary_window` → generic `primary`
  - `rate_limit.secondary_window` → generic `secondary`

## Platform-Specific Behavior
- **All platforms**: Uses positioner plugin for tray-relative window positioning, auto-hides on focus loss, always-on-top window
- **macOS**: Sets activation policy to Accessory for proper tray app behavior (no dock icon)
- **macOS**: Wake detection triggers immediate usage refresh

## Wake Detection (macOS)
The app automatically refreshes usage data when the system wakes from sleep:

- **Implementation**: Uses modern `objc2` crates (not deprecated `objc`)
  - `objc2` for Objective-C runtime interop
  - `objc2-foundation` for NSNotification and NSObject
  - `objc2-app-kit` for NSWorkspace and notification constants
- **Observer Pattern**: Creates a `WakeObserver` class using `define_class!` macro that subscribes to `NSWorkspaceDidWakeNotification`
- **Integration**: When wake is detected, signals the existing `restart_tx` channel, which triggers the auto-refresh loop to fetch immediately
- **Lifecycle**: Observer is stored in `AppState` to keep it alive and properly unregistered on drop
- **Conditional Compilation**: Module is only compiled for macOS (`#[cfg(target_os = "macos")]`)

## OS Keychain Secure Storage (Rust Backend)
- Uses `keyring` crate for cross-platform secure credential storage:
  - **macOS**: Keychain Services
  - **Windows**: Windows Credential Manager
  - **Linux**: Secret Service (libsecret)
- **Credentials never pass through frontend** - stored only in:
  - Rust backend memory (for API calls)
  - OS-native secure storage
- Rust functions in `credentials.rs`:
  - `load_credentials()` - Called in setup, loads on app start
  - `save_credentials()` - Called by `save_credentials` command
  - `delete_credentials()` - Called by `clear_credentials` command
- Tauri commands for frontend:
  - `save_credentials(org_id, session_token)` - Validates, saves, updates state
  - `clear_credentials()` - Deletes and clears state
  - `get_is_configured()` - Returns boolean without exposing credentials
- Service name: `dev.xikxp1.claude-monitor`
- No npm package needed - frontend only calls Tauri commands

## Charts & Analytics
- **Data Storage**: SQLite via `rusqlite` (Rust backend)
  - Backend saves one row per provider window to `usage_history_v2`
  - Schema: `provider`, `timestamp`, `window_key`, `label`, `utilization`, `resets_at`
  - Frontend queries by provider via Tauri commands
  - Legacy Claude snapshots are backfilled from the old wide table on startup
- **Frontend Rendering**:
  - Charts build dynamic series from `window_key`
  - Filter toggles are generated from returned history rows instead of hard-coded metrics
- **Tauri Commands**:
  - `get_usage_history_by_range(range)` - Get history for time preset ("1h", "6h", "24h", "7d", "30d")
  - `get_usage_stats(range)` - Get statistics (current, change, velocity) for time range
  - `cleanup_history(retentionDays)` - Delete old records
- **Retention Policy**: Default 30 days, configurable in settings

## Auto-Update System
- **Backend Plugins**: `tauri-plugin-updater` for checking/downloading updates, `tauri-plugin-process` for app restart
- **Update Endpoints**: GitHub releases - `https://github.com/xikxp1/claude-monitor/releases/latest/download/latest.json`
- **Frontend Composable**: `useUpdates.svelte.ts` manages update state:
  - Status states: `idle`, `checking`, `available`, `downloading`, `ready`, `error`, `up-to-date`
  - Progress tracking for downloads
  - Actions: `checkForUpdates()`, `downloadAndInstall()`, `restartApp()`
- **UI Integration**:
  - Settings "Updates" tab with full update workflow
  - Tray menu "Check for Updates" option (emits `check-for-updates` event)
  - Background check: Runs 3 seconds after app startup
  - Update banner: Clickable info banner on dashboard when update available
  - Settings badge: Blue dot indicator on Settings button when update available
- **Permissions**: `updater:default`, `process:allow-restart`
- **Build Config**: `createUpdaterArtifacts: true` in `tauri.conf.json`

## CI/CD

### GitHub Actions Workflows

**CI (`.github/workflows/ci.yml`)**:
- Triggers on: Push to main, PRs to main
- Jobs:
  - `test`: Lint, type check, frontend tests, Rust tests (Ubuntu only)
  - `build`: Build for all platforms after tests pass

**Release (`.github/workflows/release.yml`)**:
- Triggers on: Version tags (`v*`), manual dispatch
- Builds signed releases for macOS (Universal), Windows, Linux
- Creates draft GitHub release with updater artifacts

### Release Process

1. Update version in `src-tauri/tauri.conf.json`
2. Commit and push
3. Create and push tag: `git tag v0.x.x && git push origin v0.x.x`
4. Release workflow builds and creates draft release
5. Review and publish the draft release

### Required Secrets

- `TAURI_SIGNING_PRIVATE_KEY`: Minisign private key for signing updates
  - Generate with: `bunx tauri signer generate --ci -w ~/.tauri/claude-monitor.key`
  - Add the contents of the `.key` file as the secret
