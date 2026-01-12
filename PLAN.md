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
- [x] Add `reqwest` with `rustls-tls` support
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

### Phase 4: Notifications

#### 4.1 Notification System
- [x] Add `tauri-plugin-notification` (Rust + npm)
- [x] Define notification types:
  ```typescript
  interface NotificationRule {
    interval_enabled: boolean;    // Enable "every X%" notifications
    interval_percent: number;     // e.g., 10 for every 10%
    threshold_enabled: boolean;   // Enable threshold notifications
    thresholds: number[];         // e.g., [50, 80, 90]
  }
  ```
- [x] Separate rules for each usage type (5h, 7d, Sonnet, Opus)
- [x] Global enable/disable toggle

#### 4.2 Notification Logic
- [x] Interval notifications: Trigger at every X% (configurable: 5%, 10%, 15%, 20%, 25%)
- [x] Threshold notifications: Trigger when crossing specific percentages
- [x] Track notification state to avoid duplicates
- [x] Auto-reset notification state when usage resets (drops > 20%)
- [x] Persist notification state across app restarts

#### 4.3 Settings UI
- [x] Tabbed settings interface (Credentials | Notifications)
- [x] Per-usage-type configuration with collapsible sections
- [x] Checkbox toggles for interval/threshold modes
- [x] Dropdown for interval percentage
- [x] Comma-separated input for custom thresholds
- [x] Real-time save on change

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

#### 7.1 Stronghold Integration
- [x] Add `tauri-plugin-stronghold` (Rust + npm)
- [x] Add permissions to capabilities (initialize, save, create-client, etc.)
- [x] Initialize Stronghold plugin with key derivation
- [x] Create `secureStorage.ts` utility module
- [x] Migrate credentials from plain store to encrypted Stronghold storage
- [x] Update initApp to load credentials from Stronghold
- [x] Update saveSettings to save credentials to Stronghold
- [x] Update clearSettings to delete credentials from Stronghold

### Phase 8: Charts & Usage Analytics

A comprehensive analytics system to visualize usage trends and patterns over time.

#### 8.1 Dependencies & Setup
- [x] Add d3-scale for chart scaling functions
- [x] Add `tauri-plugin-sql` for SQLite storage (better for time-series queries)
- [x] Add SQL plugin permissions to capabilities
- [x] Initialize SQL plugin in Rust backend

#### 8.2 Historical Data Storage
- [x] Design and implement database schema with usage_history table
- [x] Create `historyStorage.ts` utility module:
  - `saveUsageSnapshot(usage: UsageData)` - Store current usage with timestamp
  - `getUsageHistory(from: Date, to: Date)` - Query historical data
  - `getUsageHistoryByRange(range: TimeRange)` - Query by preset time range
  - `getLatestSnapshots(count: number)` - Get recent N snapshots
  - `getUsageStats(range: TimeRange)` - Get statistics (avg, max) for time range
  - `cleanupOldData(retentionDays: number)` - Remove data older than retention period
- [x] Auto-save usage snapshot on each successful fetch
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
- [ ] Export data option (CSV)

#### 8.5 Chart Styling
- [x] Match existing app theme (dark/light mode)
- [x] Use consistent color palette:
  - 5 Hour: Blue (#3b82f6)
  - 7 Day: Purple (#8b5cf6)
  - Sonnet: Green (#22c55e)
  - Opus: Orange (#f59e0b)
- [x] Responsive design for popup window size
- [x] Threshold lines on charts (50%, 80%, 90%)
- [ ] Hover tooltips with exact values

#### 8.6 Advanced Features (Future)
- [ ] Usage predictions based on historical patterns
- [ ] Alerts when approaching limits based on velocity
- [ ] Compare current period to previous periods
- [ ] Heatmap of usage by hour/day of week

---

## Next Steps (Prioritized)

### Immediate (High Priority)

#### Error State Improvements
- [ ] Better error messages for common failures
- [ ] Network offline indicator
- [ ] Session expired prompt with re-login option

#### Tray Menu Updates
- [ ] Add separator between menu items
- [x] Show usage percentage in tray tooltip
- [ ] Add "Open Settings" menu item

### Short Term (Medium Priority)

#### Custom Tray Icons
- [ ] Design tray icon assets (16x16, 32x32, @2x)
- [ ] Different icon states:
  - Normal (gray)
  - Warning (yellow) - > 80% usage
  - Critical (red) - > 95% usage
- [ ] Update tray icon based on usage level

#### Settings Page Enhancements
- [x] Refresh interval slider/dropdown (moved to Phase 5.2)
- [x] Secure token storage (moved to Phase 7)
- [ ] Clear credentials button with confirmation

---

### Long Term (Lower Priority)

#### Distribution
- [x] App icon design (all sizes)
- [ ] macOS notarization
- [ ] Windows code signing
- [ ] Linux packages (AppImage, deb)
- [ ] GitHub releases

---

## Current Dependencies

### Rust (Cargo.toml)
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-opener = "2"
tauri-plugin-store = "2"
tauri-plugin-positioner = { version = "2", features = ["tray-icon"] }
tauri-plugin-nspopover = { git = "https://github.com/freethinkel/tauri-nspopover-plugin.git", version = "4.0.0" }
tauri-plugin-notification = "2"
tauri-plugin-autostart = "2"
tauri-plugin-stronghold = "2"
tauri-plugin-sql = { version = "2", features = ["sqlite"] }  # Phase 8: Analytics
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
thiserror = "1"
chrono = { version = "0.4", features = ["serde"] }
```

### Frontend (package.json)
```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-autostart": "^2",
    "@tauri-apps/plugin-notification": "^2",
    "@tauri-apps/plugin-store": "^2",
    "@tauri-apps/plugin-stronghold": "^2",
    "@tauri-apps/plugin-sql": "^2",
    "d3-scale": "^4"
  },
  "devDependencies": {
    "@types/d3-scale": "^4"
  }
}
```

---

## Current File Structure

```
claude-monitor/
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── NotificationSettings.svelte  # Notification config UI
│   │   │   └── charts/                       # Phase 8: Analytics charts
│   │   │       ├── UsageLineChart.svelte     # Time-series line chart
│   │   │       ├── UsageAreaChart.svelte     # Stacked area chart
│   │   │       ├── UsageStats.svelte         # Summary statistics
│   │   │       └── ChartContainer.svelte     # Reusable wrapper
│   │   ├── notifications.ts                  # Notification logic
│   │   ├── secureStorage.ts                  # Stronghold secure storage utility
│   │   ├── historyStorage.ts                 # Phase 8: SQLite history storage
│   │   └── types.ts                          # TypeScript types
│   ├── routes/
│   │   └── +page.svelte                      # Main dashboard + settings
│   └── app.html
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                            # Rust backend (API, tray, commands)
│   │   └── main.rs                           # Entry point
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

---

## Technical Notes

### NSPopover Plugin (macOS)
- Requires tray with ID "main" to exist before calling `to_popover()`
- Trait names: `AppExt`, `WindowExt` (not `AppHandleExt`, `WebviewWindowExt`)
- Plugin must be initialized with `.plugin(tauri_plugin_nspopover::init())`
- Permissions: `nspopover:allow-show-popover`, `nspopover:allow-hide-popover`, `nspopover:allow-is-popover-shown`

### Backend Auto-Refresh Architecture
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

### Notification System
- Two notification types that can be combined:
  1. **Interval**: Fires every X% (e.g., at 10%, 20%, 30%...)
  2. **Threshold**: Fires once when crossing specific values (e.g., 80%, 90%)
- Each usage type (5h, 7d, Sonnet, Opus) has independent settings
- State tracking prevents duplicate notifications
- State auto-resets when usage drops significantly (> 20% decrease)
- Settings persisted in `settings.json` via `tauri-plugin-store`
- Permissions: `notification:default`, `notification:allow-notify`, `notification:allow-is-permission-granted`, `notification:allow-request-permission`

### API Response Format
The Claude usage API returns:
```json
{
  "five_hour": { "utilization": 9.0, "resets_at": "2025-01-12T..." },
  "seven_day": { "utilization": 5.0, "resets_at": "2025-01-15T..." },
  "seven_day_sonnet": { "utilization": 3.0, "resets_at": "..." },
  "seven_day_opus": { "utilization": 0.0, "resets_at": "..." }
}
```

### Platform-Specific Behavior
- **macOS**: Uses NSPopover for proper fullscreen support, auto-hides on focus loss
- **Windows/Linux**: Uses positioner plugin, manual hide on focus loss, always-on-top window

### Stronghold Secure Storage
- Uses `tauri-plugin-stronghold` for encrypted credential storage
- Credentials stored in `credentials.stronghold` file in app data directory
- Provides cross-platform encrypted storage (not OS keychain, but Tauri's own secure format)
- Key derivation using built-in **argon2** via `Builder::with_argon2(&salt_path)`
- Salt stored in `salt.txt` in app local data directory
- Plugin initialized in setup function to access app paths
- `secureStorage.ts` utility provides: `saveCredentials()`, `getCredentials()`, `deleteCredentials()`, `initSecureStorage()`, `resetSecureStorage()`
- Organization ID and session token stored separately in Stronghold store
- **Performance optimization**: Uses singleton promise pattern to handle slow argon2 initialization; call `initSecureStorage()` early in app startup

### Charts & Analytics (Phase 8)
- **Charting Library**: Layercake recommended for Svelte-native experience
  - Composable, uses Svelte's reactivity
  - SVG-based, lightweight (~12KB)
  - Supports line, area, bar, scatter, and custom visualizations
  - Built-in responsive scaling
- **Data Storage**: SQLite via `tauri-plugin-sql`
  - Better for time-series queries than JSON store
  - Supports aggregation (AVG, MAX, MIN) for statistics
  - Efficient date range queries with indexed timestamp column
  - Database file: `usage_history.db` in app data directory
- **Data Sampling Strategy**:
  - Store one snapshot per fetch (every 1-30 min based on refresh interval)
  - For charts, downsample to reasonable points (e.g., 100-200 points max)
  - Use SQL aggregation for longer time ranges
- **Color Palette** (consistent with usage cards):
  - 5 Hour: `#3b82f6` (Blue)
  - 7 Day: `#8b5cf6` (Purple)
  - Sonnet: `#22c55e` (Green)
  - Opus: `#f59e0b` (Orange)
- **Retention Policy**: Default 30 days, configurable in settings
