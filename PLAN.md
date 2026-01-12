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

---

## Next Steps (Prioritized)

### Immediate (High Priority)

#### Error State Improvements
- [ ] Better error messages for common failures
- [ ] Network offline indicator
- [ ] Session expired prompt with re-login option

#### Tray Menu Updates
- [ ] Add separator between menu items
- [ ] Show usage percentage in tray tooltip
- [ ] Add "Open Settings" menu item

#### Auto-Refresh
- [ ] Implement background timer in frontend (setInterval)
- [ ] Use configured refresh interval from settings
- [ ] Emit `refresh-usage` event from timer
- [ ] Pause when popover/window hidden (optional)

### Short Term (Medium Priority)

#### Custom Tray Icons
- [ ] Design tray icon assets (16x16, 32x32, @2x)
- [ ] Different icon states:
  - Normal (gray)
  - Warning (yellow) - > 80% usage
  - Critical (red) - > 95% usage
- [ ] Update tray icon based on usage level

#### Settings Page Enhancements
- [ ] Refresh interval slider/dropdown
- [ ] Clear credentials button with confirmation

#### Secure Token Storage
- [ ] Add `tauri-plugin-keychain` or equivalent
- [ ] Migrate session token from JSON to OS keychain
- [ ] macOS: Keychain Services
- [ ] Windows: Credential Manager
- [ ] Linux: Secret Service API

### Long Term (Lower Priority)

#### Auto-Start
- [ ] Add `tauri-plugin-autostart`
- [ ] Configure based on user preference
- [ ] Start minimized to tray

#### Charts & Analytics
- [ ] Add lightweight charting library
- [ ] Usage trend over time
- [ ] Historical data storage

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
    "@tauri-apps/plugin-notification": "^2",
    "@tauri-apps/plugin-store": "^2"
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
│   │   │   └── NotificationSettings.svelte  # Notification config UI
│   │   ├── notifications.ts                  # Notification logic
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
