# Implementation Plan

Complete implementation plan for Claude Monitor.

## Phase 1: Core Infrastructure

### 1.1 System Tray Setup
- [ ] Add `tauri-plugin-tray` dependency to Cargo.toml
- [ ] Configure tray in `tauri.conf.json`
- [ ] Create tray icon assets (16x16, 32x32 for different platforms)
- [ ] Implement tray initialization in `lib.rs`
- [ ] Add tray menu items:
  - Show/Hide Window
  - Refresh Usage
  - Settings
  - Quit
- [ ] Handle tray click events (left-click shows window)

### 1.2 Window Management
- [ ] Configure window to hide on close (minimize to tray)
- [ ] Add `tauri-plugin-window-state` for remembering position/size
- [ ] Implement show/hide toggle from tray
- [ ] Set appropriate window properties:
  - Decorations
  - Resizable with min/max bounds
  - Start hidden option

### 1.3 Secure Storage
- [ ] Add `tauri-plugin-store` for general settings
- [ ] Implement settings storage:
  - Organization ID
  - Refresh interval (default: 5 minutes)
  - Auto-start preference
  - Theme preference
- [ ] Add keychain integration for session token (platform-specific):
  - macOS: Keychain Services
  - Windows: Credential Manager
  - Linux: Secret Service API

## Phase 2: API Integration

### 2.1 HTTP Client Setup
- [ ] Add `reqwest` with TLS support to Cargo.toml
- [ ] Create API client module (`src-tauri/src/api.rs`)
- [ ] Implement base request builder with headers:
  ```
  Cookie: sessionKey={token}
  User-Agent: Claude-Monitor/0.1.0
  ```

### 2.2 Usage Endpoint Integration
- [ ] Define usage response types:
  ```rust
  struct UsageResponse {
      daily_usage: Vec<DailyUsage>,
      billing_period: BillingPeriod,
      limits: UsageLimits,
  }
  ```
- [ ] Implement `fetch_usage` command
- [ ] Add response caching (reduce API calls)
- [ ] Handle API errors gracefully:
  - 401: Invalid/expired token
  - 429: Rate limited
  - 5xx: Server errors

### 2.3 Tauri Commands
- [ ] `get_usage()` - Fetch current usage data
- [ ] `save_settings(settings)` - Save user preferences
- [ ] `get_settings()` - Load user preferences
- [ ] `set_session_token(token)` - Store token securely
- [ ] `validate_token()` - Check if token is valid
- [ ] `clear_credentials()` - Remove stored credentials

## Phase 3: Frontend Dashboard

### 3.1 Setup Page (First Run)
- [ ] Create `/setup` route
- [ ] Organization ID input field
- [ ] Session token input (password field)
- [ ] "How to get your token" instructions
- [ ] Validate and save on submit
- [ ] Redirect to dashboard on success

### 3.2 Main Dashboard
- [ ] Create dashboard layout component
- [ ] Usage summary card:
  - Current period usage
  - Usage limit
  - Percentage used (progress bar)
  - Days remaining in period
- [ ] Daily usage chart (bar or line chart)
- [ ] Last updated timestamp
- [ ] Manual refresh button
- [ ] Auto-refresh indicator

### 3.3 Usage Visualization
- [ ] Add charting library (Chart.js or similar lightweight option)
- [ ] Daily usage bar chart
- [ ] Usage trend line (optional)
- [ ] Color coding:
  - Green: < 50% used
  - Yellow: 50-80% used
  - Red: > 80% used

### 3.4 Settings Page
- [ ] Create `/settings` route
- [ ] Refresh interval selector
- [ ] Theme toggle (light/dark/system)
- [ ] Auto-start on login toggle
- [ ] Clear credentials button
- [ ] About section with version

### 3.5 Styling
- [ ] Define CSS variables for theming
- [ ] Implement dark mode support
- [ ] Responsive layout (if window resizable)
- [ ] Consistent component styles
- [ ] Loading states and skeletons

## Phase 4: Background Operations

### 4.1 Auto-Refresh
- [ ] Implement background timer in Rust
- [ ] Configurable interval (1-60 minutes)
- [ ] Emit events to frontend on data update
- [ ] Pause refresh when window hidden (optional)

### 4.2 Notifications
- [ ] Add `tauri-plugin-notification` dependency
- [ ] Usage threshold alerts:
  - 80% usage warning
  - 90% usage critical
  - Limit reached
- [ ] Make notifications optional in settings

### 4.3 Auto-Start
- [ ] Add `tauri-plugin-autostart` dependency
- [ ] Configure based on user preference
- [ ] Start minimized to tray

## Phase 5: Error Handling & UX

### 5.1 Error States
- [ ] Network error display
- [ ] Authentication error with re-login prompt
- [ ] API error messages
- [ ] Offline mode indicator

### 5.2 Loading States
- [ ] Initial load skeleton
- [ ] Refresh loading indicator
- [ ] Button loading states

### 5.3 Empty States
- [ ] No data available message
- [ ] Setup required prompt

## Phase 6: Polish & Distribution

### 6.1 App Icon & Branding
- [ ] Design app icon (all required sizes)
- [ ] Design tray icons (normal, alert states)
- [ ] Update `tauri.conf.json` with icon paths

### 6.2 Build Configuration
- [ ] Configure bundle settings for each platform
- [ ] Set up code signing (macOS notarization)
- [ ] Configure Windows installer
- [ ] Create Linux packages (AppImage, deb)

### 6.3 Testing
- [ ] Unit tests for API module
- [ ] Unit tests for settings storage
- [ ] Frontend component tests
- [ ] E2E tests for critical flows
- [ ] Manual testing on all platforms

### 6.4 Documentation
- [ ] Complete README with screenshots
- [ ] Add CHANGELOG.md
- [ ] Write CONTRIBUTING.md
- [ ] Add LICENSE file

## Dependencies Summary

### Rust (Cargo.toml additions)
```toml
[dependencies]
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
thiserror = "1.0"
chrono = { version = "0.4", features = ["serde"] }

# Tauri plugins
tauri-plugin-store = "2"
tauri-plugin-notification = "2"
tauri-plugin-autostart = "2"
tauri-plugin-window-state = "2"
```

### Frontend (package.json additions)
```json
{
  "dependencies": {
    "@tauri-apps/plugin-store": "^2",
    "@tauri-apps/plugin-notification": "^2",
    "chart.js": "^4"
  }
}
```

## File Structure (Final)

```
claude-monitor/
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── UsageCard.svelte
│   │   │   ├── UsageChart.svelte
│   │   │   ├── SettingsForm.svelte
│   │   │   └── TrayStatus.svelte
│   │   ├── stores/
│   │   │   └── usage.ts
│   │   ├── api.ts
│   │   └── types.ts
│   ├── routes/
│   │   ├── +layout.svelte
│   │   ├── +page.svelte          # Dashboard
│   │   ├── setup/+page.svelte
│   │   └── settings/+page.svelte
│   └── app.html
├── src-tauri/
│   ├── src/
│   │   ├── api.rs                # API client
│   │   ├── commands.rs           # Tauri commands
│   │   ├── config.rs             # Settings management
│   │   ├── tray.rs               # Tray setup
│   │   ├── lib.rs                # Main app
│   │   └── main.rs
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── static/
├── tests/
├── README.md
├── CLAUDE.md
├── PLAN.md
├── CHANGELOG.md
└── package.json
```

## Implementation Order

Recommended order for implementation:

1. **Phase 1.1-1.2**: Get tray and window working first
2. **Phase 1.3**: Add settings storage
3. **Phase 2.1-2.2**: Implement API client
4. **Phase 3.1**: Setup page for initial configuration
5. **Phase 2.3**: Tauri commands to connect frontend/backend
6. **Phase 3.2-3.3**: Dashboard with usage display
7. **Phase 4.1**: Auto-refresh functionality
8. **Phase 3.4-3.5**: Settings and styling
9. **Phase 4.2-4.3**: Notifications and auto-start
10. **Phase 5**: Error handling polish
11. **Phase 6**: Final polish and distribution

## Notes

- Start with macOS development, then test Windows/Linux
- The session token is obtained from claude.ai browser cookies
- Consider adding a "Login with browser" flow in future versions
- Rate limit API calls to avoid hitting Claude's limits
- Cache usage data locally to reduce API calls
