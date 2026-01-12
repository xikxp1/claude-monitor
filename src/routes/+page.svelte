<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import {
    disable as disableAutostart,
    enable as enableAutostart,
    isEnabled as isAutostartEnabled,
  } from "@tauri-apps/plugin-autostart";
  import { LazyStore } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";
  import UsageLineChart from "$lib/components/charts/UsageLineChart.svelte";
  import NotificationSettingsComponent from "$lib/components/NotificationSettings.svelte";
  import {
    cleanupOldData,
    getUsageHistoryByRange,
    initHistoryStorage,
    saveUsageSnapshot,
    type TimeRange,
    type UsageHistoryRecord,
  } from "$lib/historyStorage";
  import {
    processNotifications,
    resetNotificationStateIfNeeded,
  } from "$lib/notifications";
  import {
    deleteCredentials,
    getCredentials,
    initSecureStorage,
    resetSecureStorage,
    saveCredentials,
  } from "$lib/secureStorage";
  import type {
    NotificationSettings,
    NotificationState,
    Settings,
    UsageData,
    UsagePeriod,
  } from "$lib/types";
  import {
    getDefaultNotificationSettings,
    getDefaultNotificationState,
  } from "$lib/types";

  let settings: Settings = $state({
    organization_id: null,
    session_token: null,
    refresh_interval_minutes: 5,
    auto_refresh_enabled: true,
  });
  let usageData: UsageData | null = $state(null);
  let initializing = $state(true);
  let loading = $state(false);
  let error: string | null = $state(null);
  let showSettings = $state(false);
  let settingsTab: "credentials" | "notifications" | "general" =
    $state("credentials");

  // Notification state
  let notificationSettings: NotificationSettings = $state(
    getDefaultNotificationSettings(),
  );
  let notificationState: NotificationState = $state(
    getDefaultNotificationState(),
  );

  // Form inputs
  let orgIdInput = $state("");
  let tokenInput = $state("");

  // Auto-refresh state (managed by backend, frontend just displays)
  let lastUpdateTime: Date | null = $state(null);
  let nextRefreshAt: number | null = $state(null); // Unix timestamp in ms
  let countdownInterval: ReturnType<typeof setInterval> | null = null;
  let secondsUntilNextUpdate = $state(0);

  // Auto-start state
  let autostartEnabled = $state(false);

  // Data retention
  let dataRetentionDays = $state(30);

  // Analytics state
  let showAnalytics = $state(false);
  let analyticsTimeRange: TimeRange = $state("24h");
  let analyticsHistory: UsageHistoryRecord[] = $state([]);
  let analyticsLoading = $state(false);

  const isConfigured = $derived(
    settings.organization_id !== null && settings.session_token !== null,
  );

  const store = new LazyStore("settings.json", {
    autoSave: true,
    defaults: {},
  });

  let unlistenFns: UnlistenFn[] = [];

  function getUsageColor(utilization: number): string {
    if (utilization >= 90) return "red";
    if (utilization >= 80) return "orange";
    if (utilization >= 50) return "yellow";
    return "green";
  }

  function formatResetTime(resets_at: string): string {
    try {
      const date = new Date(resets_at);
      const now = new Date();
      const diffMs = date.getTime() - now.getTime();
      const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
      const diffMins = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));

      if (diffHours > 24) {
        const days = Math.floor(diffHours / 24);
        return `${days}d ${diffHours % 24}h`;
      }
      if (diffHours > 0) {
        return `${diffHours}h ${diffMins}m`;
      }
      return `${diffMins}m`;
    } catch {
      return "";
    }
  }

  function formatLastUpdate(date: Date | null): string {
    if (!date) return "Never";
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSecs = Math.floor(diffMs / 1000);
    const diffMins = Math.floor(diffSecs / 60);

    if (diffSecs < 60) return "Just now";
    if (diffMins === 1) return "1 min ago";
    return `${diffMins} min ago`;
  }

  function formatCountdown(seconds: number): string {
    if (seconds <= 0) return "now";
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    if (mins > 0) {
      return `${mins}m ${secs}s`;
    }
    return `${secs}s`;
  }

  function startCountdown() {
    stopCountdown();

    // Update countdown every second based on nextRefreshAt
    countdownInterval = setInterval(() => {
      if (nextRefreshAt && settings.auto_refresh_enabled) {
        const remaining = Math.max(0, Math.floor((nextRefreshAt - Date.now()) / 1000));
        secondsUntilNextUpdate = remaining;
      } else {
        secondsUntilNextUpdate = 0;
      }
    }, 1000);
  }

  function stopCountdown() {
    if (countdownInterval) {
      clearInterval(countdownInterval);
      countdownInterval = null;
    }
  }

  onMount(() => {
    // Start secure storage initialization early (argon2 is slow)
    initSecureStorage();
    // Initialize history storage for analytics
    initHistoryStorage();

    initApp();

    return () => {
      for (const unlisten of unlistenFns) {
        unlisten();
      }
      stopCountdown();
    };
  });

  async function initApp() {
    // Set up event listeners for backend events
    unlistenFns.push(
      await listen<{ usage: UsageData; nextRefreshAt: number | null }>(
        "usage-updated",
        async (event) => {
          const { usage, nextRefreshAt: nextAt } = event.payload;
          usageData = usage;
          lastUpdateTime = new Date();
          nextRefreshAt = nextAt;
          error = null;
          loading = false;

          // Save usage snapshot for analytics
          try {
            await saveUsageSnapshot(usage);
          } catch (e) {
            console.error("Failed to save usage snapshot:", e);
          }

          // Check for usage resets and clear notification state if needed
          notificationState = resetNotificationStateIfNeeded(
            usage,
            notificationState,
          );

          // Process notifications
          const newNotificationState = await processNotifications(
            usage,
            notificationSettings,
            notificationState,
          );

          if (newNotificationState !== notificationState) {
            notificationState = newNotificationState;
            await store.set("notification_state", notificationState);
          }
        },
      ),
    );

    unlistenFns.push(
      await listen<{ error: string }>("usage-error", (event) => {
        error = event.payload.error;
        loading = false;
      }),
    );

    // Load credentials from secure storage (Stronghold)
    try {
      const credentials = await getCredentials();
      settings.organization_id = credentials.organizationId;
      settings.session_token = credentials.sessionToken;
      orgIdInput = credentials.organizationId ?? "";
      tokenInput = credentials.sessionToken ?? "";
    } catch (e) {
      console.error("Failed to load credentials:", e);
    } finally {
      initializing = false;
    }

    // Load general settings from store
    const savedInterval = await store.get<number>("refresh_interval_minutes");
    const savedAutoRefresh = await store.get<boolean>("auto_refresh_enabled");

    settings = {
      ...settings,
      refresh_interval_minutes: savedInterval ?? 5,
      auto_refresh_enabled: savedAutoRefresh ?? true,
    };

    // Load notification settings
    const savedNotificationSettings = await store.get<NotificationSettings>(
      "notification_settings",
    );
    if (savedNotificationSettings) {
      notificationSettings = savedNotificationSettings;
    }

    // Load notification state
    const savedNotificationState =
      await store.get<NotificationState>("notification_state");
    if (savedNotificationState) {
      notificationState = savedNotificationState;
    }

    // Load autostart state
    try {
      autostartEnabled = await isAutostartEnabled();
    } catch {
      autostartEnabled = false;
    }

    // Load data retention setting
    const savedRetention = await store.get<number>("data_retention_days");
    dataRetentionDays = savedRetention ?? 30;

    // Cleanup old data based on retention policy
    try {
      const deleted = await cleanupOldData(dataRetentionDays);
      if (deleted > 0) {
        console.log(`Cleaned up ${deleted} old usage records`);
      }
    } catch (e) {
      console.error("Failed to cleanup old data:", e);
    }

    // Send credentials and settings to backend to start auto-refresh
    await invoke("set_credentials", {
      orgId: settings.organization_id,
      sessionToken: settings.session_token,
    });
    await invoke("set_auto_refresh", {
      enabled: settings.auto_refresh_enabled,
      intervalMinutes: settings.refresh_interval_minutes,
    });

    // Start countdown timer
    startCountdown();
  }

  async function saveSettings() {
    loading = true;
    error = null;

    try {
      // Save credentials to secure storage (Stronghold)
      await saveCredentials(orgIdInput, tokenInput);

      settings = {
        ...settings,
        organization_id: orgIdInput,
        session_token: tokenInput,
      };

      // Send credentials to backend (this will trigger auto-refresh)
      await invoke("set_credentials", {
        orgId: orgIdInput,
        sessionToken: tokenInput,
      });

      showSettings = false;
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to save settings";
      loading = false;
    }
  }

  async function saveNotificationSettings(newSettings: NotificationSettings) {
    notificationSettings = newSettings;
    await store.set("notification_settings", newSettings);
  }

  async function saveGeneralSettings(
    autoRefreshEnabled: boolean,
    refreshIntervalMinutes: number,
  ) {
    settings = {
      ...settings,
      auto_refresh_enabled: autoRefreshEnabled,
      refresh_interval_minutes: refreshIntervalMinutes,
    };
    await store.set("auto_refresh_enabled", autoRefreshEnabled);
    await store.set("refresh_interval_minutes", refreshIntervalMinutes);

    // Send settings to backend (this will restart the auto-refresh loop)
    await invoke("set_auto_refresh", {
      enabled: autoRefreshEnabled,
      intervalMinutes: refreshIntervalMinutes,
    });
  }

  async function toggleAutostart(enabled: boolean) {
    try {
      if (enabled) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      autostartEnabled = enabled;
    } catch (e) {
      console.error("Failed to toggle autostart:", e);
    }
  }

  async function saveDataRetention(days: number) {
    dataRetentionDays = days;
    await store.set("data_retention_days", days);

    // Run cleanup with new retention policy
    try {
      const deleted = await cleanupOldData(days);
      if (deleted > 0) {
        console.log(`Cleaned up ${deleted} old usage records`);
      }
    } catch (e) {
      console.error("Failed to cleanup old data:", e);
    }
  }

  async function refreshNow() {
    if (!settings.organization_id || !settings.session_token) {
      return;
    }

    loading = true;
    error = null;

    // Trigger refresh via backend - it will emit usage-updated or usage-error event
    await invoke("refresh_now");
  }

  async function loadAnalytics() {
    analyticsLoading = true;
    try {
      analyticsHistory = await getUsageHistoryByRange(analyticsTimeRange);
    } catch (e) {
      console.error("Failed to load analytics:", e);
    } finally {
      analyticsLoading = false;
    }
  }

  async function changeTimeRange(range: TimeRange) {
    analyticsTimeRange = range;
    await loadAnalytics();
  }

  async function openAnalytics() {
    showAnalytics = true;
    showSettings = false;
    await loadAnalytics();
  }

  async function clearSettings() {
    // Delete credentials from secure storage
    await deleteCredentials();
    resetSecureStorage();

    // Clear other settings from store
    await store.clear();

    settings = {
      organization_id: null,
      session_token: null,
      refresh_interval_minutes: 5,
      auto_refresh_enabled: true,
    };
    orgIdInput = "";
    tokenInput = "";
    usageData = null;
    lastUpdateTime = null;
    nextRefreshAt = null;
    notificationSettings = getDefaultNotificationSettings();
    notificationState = getDefaultNotificationState();
    showSettings = false;

    // Clear credentials in backend (this stops auto-refresh)
    await invoke("set_credentials", {
      orgId: null,
      sessionToken: null,
    });
  }
</script>

{#snippet usageCard(title: string, period: UsagePeriod | null)}
  {#if period}
    {@const color = getUsageColor(period.utilization)}
    <div class="usage-card">
      <div class="card-header">
        <span class="card-title">{title}</span>
        <span class="reset-time"
          >Resets in {formatResetTime(period.resets_at)}</span
        >
      </div>
      <div class="usage-bar-container">
        <div
          class="usage-bar"
          class:green={color === "green"}
          class:yellow={color === "yellow"}
          class:orange={color === "orange"}
          class:red={color === "red"}
          style="width: {Math.min(period.utilization, 100)}%"
        ></div>
      </div>
      <div class="usage-percent">{period.utilization.toFixed(0)}%</div>
    </div>
  {/if}
{/snippet}

<main class="container">
  {#if initializing}
    <div class="init-loading">
      <div class="spinner"></div>
      <span>Loading...</span>
    </div>
  {:else}
    <header>
      <h1>
        <span class="claude">Claude</span> <span class="monitor">Monitor</span>
      </h1>
      {#if isConfigured}
        <div class="header-buttons">
          <button
            class="header-btn"
            class:active={showAnalytics}
            onclick={() => {
              if (showAnalytics) {
                showAnalytics = false;
              } else {
                openAnalytics();
              }
            }}
          >
            {showAnalytics ? "Dashboard" : "Analytics"}
          </button>
          <button
            class="header-btn"
            class:active={showSettings}
            onclick={() => {
              showSettings = !showSettings;
              showAnalytics = false;
            }}
          >
            {showSettings ? "Dashboard" : "Settings"}
          </button>
        </div>
      {/if}
    </header>

    {#if !isConfigured || showSettings}
      <section class="setup">
        <h2>{isConfigured ? "Settings" : "Setup"}</h2>

        {#if isConfigured}
          <div class="tabs">
            <button
              class="tab"
              class:active={settingsTab === "credentials"}
              onclick={() => (settingsTab = "credentials")}
            >
              Credentials
            </button>
            <button
              class="tab"
              class:active={settingsTab === "notifications"}
              onclick={() => (settingsTab = "notifications")}
            >
              Notifications
            </button>
            <button
              class="tab"
              class:active={settingsTab === "general"}
              onclick={() => (settingsTab = "general")}
            >
              General
            </button>
          </div>
        {/if}

        {#if settingsTab === "credentials" || !isConfigured}
          <p class="hint">
            Enter your Claude organization ID and session token to view usage.
          </p>

          <form
            onsubmit={(e) => {
              e.preventDefault();
              saveSettings();
            }}
          >
            <label>
              Organization ID
              <input
                type="text"
                bind:value={orgIdInput}
                placeholder="uuid-format-org-id"
                required
              />
            </label>

            <label>
              Session Token
              <input
                type="password"
                bind:value={tokenInput}
                placeholder="Your session token"
                required
              />
            </label>

            <div class="actions">
              <button type="submit" disabled={loading}>
                {loading ? "Saving..." : "Save"}
              </button>
              {#if isConfigured}
                <button type="button" class="danger" onclick={clearSettings}>
                  Clear
                </button>
              {/if}
            </div>
          </form>

          <details class="help">
            <summary>How to get your session token</summary>
            <ol>
              <li>
                Go to <a href="https://claude.ai" target="_blank">claude.ai</a> and
                log in
              </li>
              <li>Open browser DevTools (F12)</li>
              <li>Go to Application > Cookies > claude.ai</li>
              <li>Find the "sessionKey" cookie and copy its value</li>
            </ol>
          </details>
        {:else if settingsTab === "notifications"}
          <div class="notification-settings-wrapper">
            <NotificationSettingsComponent
              settings={notificationSettings}
              onchange={saveNotificationSettings}
            />
          </div>
        {:else if settingsTab === "general"}
          <div class="general-settings">
            <label class="toggle-row">
              <input
                type="checkbox"
                checked={settings.auto_refresh_enabled}
                onchange={(e) =>
                  saveGeneralSettings(
                    e.currentTarget.checked,
                    settings.refresh_interval_minutes,
                  )}
              />
              <span>Enable auto-refresh</span>
            </label>

            {#if settings.auto_refresh_enabled}
              <label class="select-row">
                <span>Refresh interval</span>
                <select
                  value={settings.refresh_interval_minutes}
                  onchange={(e) =>
                    saveGeneralSettings(
                      settings.auto_refresh_enabled,
                      Number.parseInt(e.currentTarget.value, 10),
                    )}
                >
                  <option value={1}>1 minute</option>
                  <option value={2}>2 minutes</option>
                  <option value={5}>5 minutes</option>
                  <option value={10}>10 minutes</option>
                  <option value={15}>15 minutes</option>
                  <option value={30}>30 minutes</option>
                </select>
              </label>
            {/if}

            <label class="toggle-row">
              <input
                type="checkbox"
                checked={autostartEnabled}
                onchange={(e) => toggleAutostart(e.currentTarget.checked)}
              />
              <span>Start at login</span>
            </label>

            <label class="select-row">
              <span>Data retention</span>
              <select
                value={dataRetentionDays}
                onchange={(e) =>
                  saveDataRetention(Number.parseInt(e.currentTarget.value, 10))}
              >
                <option value={7}>7 days</option>
                <option value={14}>14 days</option>
                <option value={30}>30 days</option>
                <option value={60}>60 days</option>
                <option value={90}>90 days</option>
              </select>
            </label>
          </div>
        {/if}
      </section>
    {:else if showAnalytics}
      <section class="analytics">
        <h2>Usage Analytics</h2>

        <div class="time-range-selector">
          <button
            class="range-btn"
            class:active={analyticsTimeRange === "1h"}
            onclick={() => changeTimeRange("1h")}>1h</button
          >
          <button
            class="range-btn"
            class:active={analyticsTimeRange === "6h"}
            onclick={() => changeTimeRange("6h")}>6h</button
          >
          <button
            class="range-btn"
            class:active={analyticsTimeRange === "24h"}
            onclick={() => changeTimeRange("24h")}>24h</button
          >
          <button
            class="range-btn"
            class:active={analyticsTimeRange === "7d"}
            onclick={() => changeTimeRange("7d")}>7d</button
          >
          <button
            class="range-btn"
            class:active={analyticsTimeRange === "30d"}
            onclick={() => changeTimeRange("30d")}>30d</button
          >
        </div>

        <div class="chart-section">
          <UsageLineChart data={analyticsHistory} height={220} />
        </div>
      </section>
    {:else}
      <section class="dashboard">
        <div class="refresh-row">
          <div class="update-info">
            <span class="last-update"
              >Updated: {formatLastUpdate(lastUpdateTime)}</span
            >
            {#if settings.auto_refresh_enabled}
              <span class="next-update"
                >Next: {formatCountdown(secondsUntilNextUpdate)}</span
              >
            {:else}
              <span class="next-update disabled">Auto-refresh off</span>
            {/if}
          </div>
          <button class="refresh-btn" onclick={refreshNow} disabled={loading}>
            {loading ? "Loading..." : "Refresh"}
          </button>
        </div>

        {#if loading && !usageData}
          <div class="loading">Loading usage data...</div>
        {:else if error}
          <div class="error">
            <p>{error}</p>
            <button onclick={refreshNow}>Retry</button>
          </div>
        {:else if usageData}
          <div class="usage-grid">
            {@render usageCard("5 Hour", usageData.five_hour)}
            {@render usageCard("7 Day", usageData.seven_day)}
            {@render usageCard("Sonnet (7 Day)", usageData.seven_day_sonnet)}
            {@render usageCard("Opus (7 Day)", usageData.seven_day_opus)}
          </div>
        {:else}
          <div class="empty">No usage data available</div>
        {/if}
      </section>
    {/if}
  {/if}
</main>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: transparent;
  }

  :root {
    font-family:
      Inter,
      -apple-system,
      BlinkMacSystemFont,
      "Segoe UI",
      Roboto,
      sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: #1a1a1a;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f0f0f0;
    }
  }

  .container {
    max-width: 100%;
    padding: 16px;
    height: 100vh;
    box-sizing: border-box;
    background-color: #fafafa;
    border-radius: 12px;
    border: 1px solid #d0d0d0;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.15);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  @media (prefers-color-scheme: dark) {
    .container {
      background-color: #1a1a1a;
      border-color: #333;
      box-shadow: 0 4px 24px rgba(0, 0, 0, 0.4);
    }
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid #e0e0e0;
  }

  @media (prefers-color-scheme: dark) {
    header {
      border-bottom-color: #333;
    }
  }

  h1 {
    margin: 0;
    margin-left: 4px;
    margin-right: 12px;
    font-size: 1.4rem;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  h1 .claude {
    color: #1e3a5f;
  }

  h1 .monitor {
    color: #8899a6;
    font-weight: 400;
  }

  @media (prefers-color-scheme: dark) {
    h1 .claude {
      color: #5ba3d9;
    }

    h1 .monitor {
      color: #8899a6;
    }
  }

  h2 {
    margin: 0 0 8px;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .header-buttons {
    display: flex;
    gap: 6px;
  }

  .header-btn {
    padding: 6px 12px;
    font-size: 0.75rem;
    font-weight: 500;
    background: #f0f4f8;
    border: 1px solid #d0d7de;
    border-radius: 6px;
    cursor: pointer;
    color: #1e3a5f;
    transition: all 0.15s ease;
  }

  .header-btn:hover {
    background: #7c3aed;
    border-color: #7c3aed;
    color: #fff;
  }

  .header-btn.active {
    background: #7c3aed;
    border-color: #7c3aed;
    color: #fff;
  }

  @media (prefers-color-scheme: dark) {
    .header-btn {
      background: #2a3a4a;
      border-color: #3a4a5a;
      color: #c0d0e0;
    }

    .header-btn:hover,
    .header-btn.active {
      background: #7c3aed;
      border-color: #7c3aed;
      color: #fff;
    }
  }

  .analytics {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .analytics h2 {
    margin: 0 0 4px;
    margin-left: 4px;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .time-range-selector {
    display: flex;
    gap: 4px;
    background: #e5e5e5;
    border-radius: 8px;
    padding: 4px;
  }

  @media (prefers-color-scheme: dark) {
    .time-range-selector {
      background: #2a2a2a;
    }
  }

  .range-btn {
    flex: 1;
    padding: 6px 8px;
    font-size: 0.75rem;
    font-weight: 500;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    color: #666;
    transition: all 0.15s ease;
  }

  .range-btn:hover {
    background: rgba(124, 58, 237, 0.7);
    color: #fff;
  }

  .range-btn.active {
    background: #7c3aed;
    color: #fff;
  }

  @media (prefers-color-scheme: dark) {
    .range-btn {
      color: #999;
    }

    .range-btn:hover {
      background: rgba(124, 58, 237, 0.7);
      color: #fff;
    }

    .range-btn.active {
      background: #7c3aed;
      color: #fff;
    }
  }

  .chart-section {
    background: #fff;
    border-radius: 10px;
    padding: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }

  @media (prefers-color-scheme: dark) {
    .chart-section {
      background: #2a2a2a;
      box-shadow: none;
    }
  }

  .setup {
    max-width: 320px;
    margin: 0 auto;
    flex: 1;
    overflow-y: auto;
  }

  .tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 16px;
    background: #e5e5e5;
    border-radius: 8px;
    padding: 4px;
  }

  @media (prefers-color-scheme: dark) {
    .tabs {
      background: #2a2a2a;
    }
  }

  .tab {
    flex: 1;
    padding: 8px 12px;
    font-size: 0.85rem;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    color: #666;
    font-weight: 500;
    transition: all 0.15s ease;
  }

  .tab:hover {
    background: #7c3aed;
    color: #fff;
  }

  @media (prefers-color-scheme: dark) {
    .tab {
      color: #999;
    }

    .tab:hover {
      background: #7c3aed;
      color: #fff;
    }
  }

  .tab.active {
    background: #7c3aed;
    color: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  @media (prefers-color-scheme: dark) {
    .tab.active {
      background: #7c3aed;
      color: #fff;
    }
  }

  .hint {
    color: #666;
    font-size: 0.85rem;
    margin-bottom: 16px;
  }

  @media (prefers-color-scheme: dark) {
    .hint {
      color: #999;
    }
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.85rem;
    font-weight: 500;
  }

  input {
    padding: 10px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 0.9rem;
    background: #fff;
  }

  @media (prefers-color-scheme: dark) {
    input {
      background: #2a2a2a;
      border-color: #444;
      color: #f0f0f0;
    }
  }

  input:focus {
    outline: none;
    border-color: #7c3aed;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  button {
    padding: 10px 16px;
    border: none;
    border-radius: 6px;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    background: #7c3aed;
    color: #fff;
  }

  button:hover:not(:disabled) {
    background: #6d28d9;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  button.danger {
    background: #dc2626;
  }

  button.danger:hover:not(:disabled) {
    background: #b91c1c;
  }

  .help {
    margin-top: 20px;
    font-size: 0.85rem;
    color: #666;
  }

  @media (prefers-color-scheme: dark) {
    .help {
      color: #999;
    }
  }

  .help summary {
    cursor: pointer;
    font-weight: 500;
  }

  .help ol {
    margin: 8px 0 0;
    padding-left: 20px;
  }

  .help a {
    color: #7c3aed;
  }

  .notification-settings-wrapper {
    margin-top: 8px;
  }

  .general-settings {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-top: 8px;
  }

  label.toggle-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 500;
  }

  label.toggle-row input[type="checkbox"] {
    width: 14px;
    height: 14px;
    accent-color: #7c3aed;
  }

  label.select-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 0.9rem;
  }

  .select-row select {
    padding: 8px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
    font-size: 0.85rem;
    background: #fff;
    cursor: pointer;
  }

  @media (prefers-color-scheme: dark) {
    .select-row select {
      background: #2a2a2a;
      border-color: #444;
      color: #f0f0f0;
    }
  }

  .next-update.disabled {
    font-style: italic;
    opacity: 0.7;
  }

  .dashboard {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1;
    overflow-y: auto;
  }

  .refresh-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .update-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.75rem;
    color: #888;
    margin-left: 4px;
  }

  .last-update,
  .next-update {
    white-space: nowrap;
  }

  .refresh-btn {
    padding: 6px 14px;
    font-size: 0.85rem;
    background: #f0f0f0;
    color: #333;
  }

  .refresh-btn:hover:not(:disabled) {
    background: #7c3aed;
    color: #fff;
  }

  @media (prefers-color-scheme: dark) {
    .refresh-btn {
      background: #3a3a3a;
      color: #f0f0f0;
    }

    .refresh-btn:hover:not(:disabled) {
      background: #7c3aed;
      color: #fff;
    }
  }

  .loading,
  .empty {
    text-align: center;
    color: #666;
    padding: 40px 20px;
  }

  .error {
    text-align: center;
    color: #dc2626;
    padding: 20px;
  }

  .error button {
    margin-top: 12px;
  }

  .usage-grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .usage-card {
    background: #fff;
    border-radius: 10px;
    padding: 14px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }

  @media (prefers-color-scheme: dark) {
    .usage-card {
      background: #2a2a2a;
      box-shadow: none;
    }
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }

  .card-title {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .reset-time {
    font-size: 0.75rem;
    color: #888;
  }

  .usage-bar-container {
    height: 10px;
    background: #e5e5e5;
    border-radius: 5px;
    overflow: hidden;
  }

  @media (prefers-color-scheme: dark) {
    .usage-bar-container {
      background: #3a3a3a;
    }
  }

  .usage-bar {
    height: 100%;
    border-radius: 5px;
    transition: width 0.3s ease;
  }

  .usage-bar.green {
    background: #22c55e;
  }

  .usage-bar.yellow {
    background: #eab308;
  }

  .usage-bar.orange {
    background: #f97316;
  }

  .usage-bar.red {
    background: #ef4444;
  }

  .usage-percent {
    margin-top: 8px;
    font-size: 1.2rem;
    font-weight: 600;
    text-align: center;
  }

  .init-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 16px;
    color: #666;
    font-size: 0.9rem;
  }

  @media (prefers-color-scheme: dark) {
    .init-loading {
      color: #999;
    }
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #e5e5e5;
    border-top-color: #7c3aed;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @media (prefers-color-scheme: dark) {
    .spinner {
      border-color: #3a3a3a;
      border-top-color: #7c3aed;
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
