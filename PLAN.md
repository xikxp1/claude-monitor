# Implementation Plan

Complete implementation plan for Claude Monitor.

## Completed

### Phase 1: Core Infrastructure

#### 1.1 System Tray Setup
- [x] Configure tray in Tauri 2 (built-in `tray-icon` feature)
- [x] Use default app icon for tray
- [x] Implement tray initialization in `lib.rs`
- [x] Add tray menu items:
  - Show Window
  - Refresh
  - Quit
- [x] Handle tray click events (left-click shows popover on macOS)
- [x] **NSPopover integration for macOS** - Shows window on top of fullscreen apps

#### 1.2 Window Management
- [x] Configure window to hide on close (minimize to tray)
- [x] Implement show/hide toggle from tray
- [x] Set window properties:
  - No decorations (popup style)
  - Fixed size (400x500)
  - Start hidden
  - Skip taskbar
  - Always on top (non-macOS)
- [x] **macOS**: Use `tauri-plugin-nspopover` for proper popover behavior
- [x] **macOS**: Set `ActivationPolicy::Accessory` for tray-only app
- [x] **Non-macOS**: Use `tauri-plugin-positioner` for tray positioning

#### 1.3 Settings Storage
- [x] Add `tauri-plugin-store` for settings persistence
- [x] Implement settings storage:
  - Organization ID
  - Session token (stored in settings.json for now)
  - Refresh interval (default: 5 minutes)

### Phase 2: API Integration

#### 2.1 HTTP Client Setup
- [x] Add `reqwest` with `rustls` support
- [x] Implement request with headers:
  ```
  Cookie: sessionKey={token}
  User-Agent: Claude-Monitor/0.1.0
  ```

#### 2.2 Usage Endpoint Integration
- [x] Define usage response types matching actual API:
  ```rust
  struct UsageData {
      five_hour: Option<UsagePeriod>,
      seven_day: Option<UsagePeriod>,
      seven_day_sonnet: Option<UsagePeriod>,
      seven_day_opus: Option<UsagePeriod>,
  }
  ```
- [x] Implement `fetch_usage_from_api` function
- [x] Handle API errors:
  - 401: Invalid/expired token
  - 429: Rate limited
  - 5xx: Server errors

#### 2.3 Tauri Commands
- [x] `get_usage(org_id, session_token)` - Fetch current usage data
- [x] `get_default_settings()` - Get default settings

### Phase 3: Frontend Dashboard

#### 3.1 Setup Page
- [x] Integrated setup form in main page
- [x] Organization ID input field
- [x] Session token input (password field)
- [x] "How to get your token" instructions (collapsible)
- [x] Save and validate on submit
- [x] Show dashboard on success

#### 3.2 Main Dashboard
- [x] Dashboard layout with header
- [x] Usage cards for each period:
  - 5 Hour usage
  - 7 Day usage
  - Sonnet (7 Day) usage
  - Opus (7 Day) usage
- [x] Progress bar with percentage
- [x] Reset time countdown
- [x] Manual refresh button
- [x] Settings toggle button

#### 3.3 Usage Visualization
- [x] Progress bars for each usage period
- [x] Color coding:
  - Green: < 50% used
  - Yellow: 50-80% used
  - Red: > 80% used
- [x] Percentage display

#### 3.5 Styling
- [x] CSS variables for theming
- [x] Dark mode support (prefers-color-scheme)
- [x] Consistent component styles
- [x] Loading states
- [x] Popup-style container with rounded corners and shadow
- [x] Refactored to daisyUI (Tailwind CSS v4) - minimized custom CSS

### Phase 4: Notifications (Backend-Driven)

#### 4.1 Notification System
- [x] Add `tauri-plugin-notification` (Rust + npm)
- [x] Define notification types in Rust (`types.rs`):
  ```rust
  struct NotificationRule {
      interval_enabled: bool,
      interval_percent: u32,
      threshold_enabled: bool,
      thresholds: Vec<u32>,
      time_remaining_enabled: bool,
      time_remaining_minutes: Vec<u32>,
  }
  ```
- [x] Separate rules for each usage type (5h, 7d, Sonnet, Opus)
- [x] Global enable/disable toggle

#### 4.2 Notification Logic (Rust Backend)
- [x] Create `notifications.rs` module with notification processing
- [x] Interval notifications: Trigger at every X% (configurable: 5%, 10%, 15%, 20%, 25%)
- [x] Threshold notifications: Trigger when crossing specific percentages
- [x] Time-remaining notifications: Trigger when less than X minutes until reset (configurable: e.g., 30, 60 minutes)
- [x] Track notification state in `AppState` to avoid duplicates
- [x] Auto-reset notification state when usage resets (drops > 20%)
- [x] Load notification settings/state from store on startup
- [x] Process notifications in `auto_refresh.rs` after each fetch
- [x] Fire notifications via `tauri-plugin-notification` Rust API
- [x] `set_notification_settings` command to sync frontend settings to backend

#### 4.3 Settings UI
- [x] Tabbed settings interface (Credentials | Notifications)
- [x] Per-usage-type configuration with collapsible sections
- [x] Checkbox toggles for interval/threshold/time-remaining modes
- [x] Dropdown for interval percentage
- [x] Predefined threshold chips (50%, 60%, 70%, 80%, 90%, 95%) - toggle to select
- [x] Predefined time-remaining chips - context-aware options:
  - 5 Hour: 15m, 30m, 1h, 2h
  - 7 Day types: 30m, 1h, 2h, 4h, 12h, 1d, 2d
- [x] Real-time save on change (syncs to backend via command)

### Phase 5: Auto-Refresh (Backend-Driven)

#### 5.1 Backend Auto-Refresh Loop
- [x] Implement background timer in Rust using `tokio::spawn` and `tokio::time::interval`
- [x] Use `watch` channel for restart signals when settings change
- [x] Store configuration in `Mutex<AutoRefreshConfig>` (credentials, interval, enabled)
- [x] Emit `usage-updated` event with usage data and next refresh timestamp
- [x] Emit `usage-error` event on fetch failures
- [x] Update tray tooltip automatically on fetch

#### 5.2 Tauri Commands
- [x] `set_credentials(org_id, session_token)` - Update credentials and restart loop
- [x] `set_auto_refresh(enabled, interval_minutes)` - Update settings and restart loop
- [x] `refresh_now()` - Trigger immediate fetch and reset timer

#### 5.3 Frontend Event Handling
- [x] Listen for `usage-updated` event to receive usage data
- [x] Listen for `usage-error` event to display errors
- [x] Calculate countdown from `nextRefreshAt` timestamp
- [x] Display last update time ("Updated: Just now", "1 min ago", etc.)
- [x] Display countdown to next update ("Next: 4m 32s")

#### 5.4 Auto-Refresh Settings UI
- [x] Add "General" tab to settings page
- [x] Enable/disable auto-refresh toggle
- [x] Refresh interval dropdown (1, 2, 5, 10, 15, 30 minutes)
- [x] Persist settings to store
- [x] Show "Auto-refresh off" when disabled
- [x] Send settings to backend via `set_auto_refresh` command

### Phase 6: Auto-Start

#### 6.1 Auto-Start at Login
- [x] Add `tauri-plugin-autostart` (Rust + npm)
- [x] Add permissions to capabilities (enable, disable, is-enabled)
- [x] Initialize plugin with MacosLauncher::LaunchAgent
- [x] "Start at login" toggle in General settings
- [x] Load current autostart state on init
- [x] Enable/disable autostart on toggle

### Phase 7: Secure Token Storage

#### 7.1 OS Keychain Integration (Rust Backend)
- [x] Add `keyring` crate with platform-native features:
  - `apple-native` for macOS Keychain
  - `windows-native` for Windows Credential Manager
  - `linux-native-sync-persistent` for Linux Secret Service
- [x] Implement Rust functions in `credentials.rs`:
  - `load_credentials()` - Load credentials from OS keychain on app startup
  - `save_credentials()` - Save credentials to OS keychain
  - `delete_credentials()` - Clear credentials from OS keychain
- [x] Load credentials from keychain in setup function (before auto-refresh starts)
- [x] `save_credentials` command - Validates, saves to keychain, updates in-memory state
- [x] `clear_credentials` command - Deletes from keychain and clears state
- [x] `get_is_configured` command - Check if credentials exist without exposing them
- [x] Credentials never pass through frontend - only exist in:
  - User form input (briefly, during setup)
  - Rust backend (memory + OS-native secure storage)

### Phase 8: Charts & Usage Analytics

A comprehensive analytics system to visualize usage trends and patterns over time.

#### 8.1 Dependencies & Setup
- [x] Add d3-scale for chart scaling functions
- [x] Add `rusqlite` for SQLite storage (Rust backend)
- [x] Initialize database in Rust backend setup

#### 8.2 Historical Data Storage (Backend-Driven)
- [x] Design and implement database schema in `history.rs`
- [x] Create Rust functions for storage:
  - `save_usage_snapshot()` - Store current usage with timestamp (called in auto_refresh)
  - `get_usage_history_by_range()` - Query by preset time range
  - `get_usage_stats()` - Get statistics (current, change, velocity) for time range
  - `cleanup_old_data()` - Remove data older than retention period
- [x] Create Tauri commands for frontend access:
  - `get_usage_history_by_range` / `get_usage_stats` / `cleanup_history`
- [x] Auto-save usage snapshot on each successful fetch (backend)
- [x] Add retention period setting in General settings

#### 8.3 Analytics Components
- [x] Create `src/lib/components/charts/` directory
- [x] **UsageLineChart**: All usage types over time
  - Configurable time range (1h, 6h, 24h, 7d, 30d)
  - Responsive sizing with d3-scale
  - Legend with color coding
  - Dark/light mode support
- [x] **UsageStats**: Summary statistics
  - Average usage per period
  - Peak usage (max recorded)
  - Data point count

#### 8.4 Analytics View
- [x] Add "Analytics" button to header (alongside Settings)
- [x] Time range selector (1h, 6h, 24h, 7d, 30d)
- [x] Usage trend chart section
- [x] Statistics summary section
- [x] Usage type filter (show/hide specific types)

#### 8.5 Chart Styling
- [x] Match existing app theme (dark/light mode)
- [x] Use consistent color palette:
  - 5 Hour: Blue (#3b82f6)
  - 7 Day: Purple (#8b5cf6)
  - Sonnet: Green (#22c55e)
  - Opus: Orange (#f59e0b)
- [x] Responsive design for popup window size
- [x] Threshold lines on charts (50%, 80%, 90%)

---

## Pre-Release Fixes (v0.1.0)

Critical issues to fix before the first public release.

### Must Fix (Blocking Release)

#### Code Quality
- [x] **Fix tray icon unwrap panic** - `src-tauri/src/tray.rs:48` uses `.unwrap()` on `default_window_icon()` which could panic if icon is missing. Replace with proper error handling.
- [x] **Remove debug logging** - Production builds should not include debug output:
  - `src-tauri/src/validation.rs:16,36` - `eprintln!` for invalid characters
  - `src-tauri/src/api.rs:35,36,45` - `eprintln!` for parse/HTTP errors
  - `src-tauri/src/auto_refresh.rs:24,67` - `eprintln!` for snapshot/refresh errors
  - `src-tauri/src/lib.rs:91` - `eprintln!` for database init failure
  - `src/lib/composables/useSettings.svelte.ts:96,204` - `console.log` for cleanup
- [x] **Set Content Security Policy** - `tauri.conf.json:26` has `"csp": null`. Add proper CSP for security.

#### Documentation
- [x] **Fix README test command** - README mentions `bun run test` but no test script exists in package.json. Either add tests or remove the section.
- [ ] **Update README project structure** - The structure section is outdated (doesn't show new modules like `composables/`, `charts/`, Rust modules).

### Should Fix (High Priority)

#### Stability
- [x] Add basic unit tests for critical paths:
  - `src/lib/utils/formatting.ts` - Pure functions (18 tests)
  - `src/lib/types.ts` - Factory functions (12 tests)
  - `src-tauri/src/validation.rs` - Security-critical validation (20 tests)
  - `src-tauri/src/notifications.rs` - Notification logic (26 tests)
  - `src-tauri/src/history.rs` - Statistics calculation (12 tests)
- [x] Fix loading state not resetting on successful credential save (`useSettings.svelte.ts:131`)

#### UX Polish
- [x] Add confirmation toast/feedback when settings are saved successfully
- [x] Better error messages for common failures (401 = "Session expired", network errors)

---

## Next Steps (Post-Release)

### Medium Priority - Stability

#### Error Handling
- [x] Add try-catch to all Tauri command invokes (`get_is_configured`, `set_auto_refresh`, `save_credentials`, `clear_credentials`, `refresh_now`)
- [x] Add try-catch to all store operations (`saveNotificationSettings`, `saveGeneralSettings`, `saveDataRetention`, `clearSettings`)
- [x] Session expired prompt with re-login option
- [x] Add exponential backoff on 429 rate limit errors

#### State Management Fixes
- [x] Add debouncing to settings form changes

#### UX Improvements
- [x] Show loading state during initial credential setup
- [x] Split "Clear" into "Log Out" (credentials only) and "Reset All Settings" (factory reset with confirmation)

#### Tray Menu Updates
- [x] Show usage percentage in tray tooltip

### Short Term (Medium Priority - Code Quality)

#### Code Organization
- [x] Break `+page.svelte` into smaller composables:
  - `useSettings.svelte.ts` - Credentials, general settings, autostart, notifications
  - `useAnalytics.svelte.ts` - Analytics data loading and chart filters
  - `useUsageData.svelte.ts` - Usage data fetching, events, countdown timer
  - `utils/formatting.ts` - Pure formatting functions (getUsageColor, formatResetTime, etc.)
- [x] Split `src-tauri/src/lib.rs` into modules:
  - `error.rs` - AppError enum with Serialize impl
  - `types.rs` - All data structures (UsageData, Settings, NotificationRule, etc.)
  - `validation.rs` - Input validation (validate_session_token, validate_org_id)
  - `credentials.rs` - OS keychain storage (keyring crate)
  - `api.rs` - HTTP client (fetch_usage_from_api)
  - `notifications.rs` - Notification processing and firing
  - `tray.rs` - System tray creation and tooltip updates
  - `auto_refresh.rs` - Background refresh loop (includes notifications)
  - `commands.rs` - Tauri commands

#### Type Safety
- [x] Codegen for shared Rust/TypeScript types (specta)
- [x] Type-safe command invocations via tauri-specta (generated `commands` object)

#### Settings Page Enhancements
- [x] Refresh interval slider/dropdown (moved to Phase 5.2)
- [x] Secure token storage (moved to Phase 7)

---

### Short Term (Medium Priority - Testing)

#### Testing Infrastructure
- [x] Add Vitest unit tests for frontend utilities (formatting.ts, types.ts) - 30 tests
- [x] Add Vitest unit tests for composables (useAnalytics, useUsageData, useSettings) - 85 tests
- [x] Add Cargo tests for Rust modules (validation, history, notifications) - 60 tests
- [x] Add integration tests for Tauri commands - 36 tests covering:
  - State management (get_is_configured, set_auto_refresh, set_notification_settings)
  - Input validation (save_credentials validation)
  - Credential management (clear_credentials logic)
  - History commands (error handling when DB not initialized)
  - Type defaults and state management
- [x] Add tests for auto_refresh module - 26 tests covering:
  - FetchResult enum behavior
  - Backoff constants verification
  - Exponential backoff calculation (rate limiting)
  - Wait duration logic
  - Refresh conditions (enabled, has_credentials)
  - Next refresh timestamp calculation
  - Full backoff/recovery cycle integration tests

### Long Term (Lower Priority - Performance & Distribution)

#### Performance Optimizations
- [x] Pause countdown timer when window hidden (reduce CPU wakeups)
- [ ] Add downsampling for analytics with large datasets
- [ ] Conditional plugin loading for platform-specific features

#### Distribution
- [x] App icon design (all sizes)
- [ ] GitHub releases
