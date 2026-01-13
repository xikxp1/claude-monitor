<script lang="ts">
  import { onMount } from "svelte";
  import UsageLineChart from "$lib/components/charts/UsageLineChart.svelte";
  import NotificationSettingsComponent from "$lib/components/NotificationSettings.svelte";
  import { useAnalytics, useSettings, useUsageData } from "$lib/composables";
  import { initHistoryStorage } from "$lib/historyStorage";
  import type { UsagePeriod } from "$lib/types";
  import {
    formatCountdown,
    formatResetTime,
    formatSecondsAgo,
    getUsageColor,
  } from "$lib/utils";

  // Initialize composables
  const settings = useSettings();
  const analytics = useAnalytics();

  // Usage data needs callbacks to interact with settings
  const usageData = useUsageData({
    isAutoRefreshEnabled: () => settings.autoRefreshEnabled,
    setLoading: (value) => {
      settings.loading = value;
    },
    setError: (value) => {
      settings.error = value;
    },
    isConfigured: () => settings.isConfigured,
  });

  let initializing = $state(true);

  onMount(() => {
    initHistoryStorage();
    initApp();

    return () => {
      usageData.cleanup();
    };
  });

  async function initApp() {
    // Set up event listeners for backend events
    await usageData.setupEventListeners();

    // Initialize settings (loads from store and backend)
    await settings.init();

    initializing = false;

    // Start countdown timer
    usageData.startCountdown();
  }

  function openAnalytics() {
    analytics.open();
    settings.showSettings = false;
  }

  function openSettings() {
    settings.toggle();
    analytics.showAnalytics = false;
  }

  async function handleClearSettings() {
    await settings.clearAll();
    usageData.reset();
  }
</script>

{#snippet usageCard(title: string, period: UsagePeriod | null)}
  {#if period}
    {@const color = getUsageColor(period.utilization)}
    <div class="usage-card">
      <div class="card-header">
        <span class="card-title">{title}</span>
        <span class="reset-time"
          >{period.resets_at
            ? `Resets in ${formatResetTime(period.resets_at)}`
            : "Starts when a message is sent"}</span
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
      {#if settings.isConfigured}
        <div class="header-buttons">
          <button
            class="header-btn"
            class:active={analytics.showAnalytics}
            onclick={() => {
              if (analytics.showAnalytics) {
                analytics.close();
              } else {
                openAnalytics();
              }
            }}
          >
            {analytics.showAnalytics ? "Dashboard" : "Analytics"}
          </button>
          <button
            class="header-btn"
            class:active={settings.showSettings}
            onclick={openSettings}
          >
            {settings.showSettings ? "Dashboard" : "Settings"}
          </button>
        </div>
      {/if}
    </header>

    {#if !settings.isConfigured || settings.showSettings}
      <section class="setup">
        <h2>{settings.isConfigured ? "Settings" : "Setup"}</h2>

        {#if settings.isConfigured}
          <div class="tabs">
            <button
              class="tab"
              class:active={settings.settingsTab === "credentials"}
              onclick={() => (settings.settingsTab = "credentials")}
            >
              Credentials
            </button>
            <button
              class="tab"
              class:active={settings.settingsTab === "notifications"}
              onclick={() => (settings.settingsTab = "notifications")}
            >
              Notifications
            </button>
            <button
              class="tab"
              class:active={settings.settingsTab === "general"}
              onclick={() => (settings.settingsTab = "general")}
            >
              General
            </button>
          </div>
        {/if}

        {#if settings.settingsTab === "credentials" || !settings.isConfigured}
          <p class="hint">
            Enter your Claude organization ID and session token to view usage.
          </p>

          <form
            onsubmit={(e) => {
              e.preventDefault();
              settings.saveCredentials();
            }}
          >
            <label>
              Organization ID
              <input
                type="text"
                bind:value={settings.orgIdInput}
                placeholder="uuid-format-org-id"
                required
              />
            </label>

            <label>
              Session Token
              <input
                type="password"
                bind:value={settings.tokenInput}
                placeholder="Your session token"
                required
              />
            </label>

            <div class="actions">
              <button type="submit" disabled={settings.loading}>
                {settings.loading ? "Saving..." : "Save"}
              </button>
              {#if settings.isConfigured}
                <button
                  type="button"
                  class="danger"
                  onclick={handleClearSettings}
                >
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
        {:else if settings.settingsTab === "notifications"}
          <div class="notification-settings-wrapper">
            <NotificationSettingsComponent
              settings={settings.notificationSettings}
              onchange={settings.saveNotifications}
            />
          </div>
        {:else if settings.settingsTab === "general"}
          <div class="general-settings">
            <label class="toggle-row">
              <input
                type="checkbox"
                checked={settings.autoRefreshEnabled}
                onchange={(e) =>
                  settings.saveGeneral(
                    e.currentTarget.checked,
                    settings.refreshIntervalMinutes,
                  )}
              />
              <span>Enable auto-refresh</span>
            </label>

            {#if settings.autoRefreshEnabled}
              <label class="select-row">
                <span>Refresh interval</span>
                <select
                  value={settings.refreshIntervalMinutes}
                  onchange={(e) =>
                    settings.saveGeneral(
                      settings.autoRefreshEnabled,
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
                checked={settings.autostartEnabled}
                onchange={(e) =>
                  settings.toggleAutostart(e.currentTarget.checked)}
              />
              <span>Start at login</span>
            </label>

            <label class="select-row">
              <span>Data retention</span>
              <select
                value={settings.dataRetentionDays}
                onchange={(e) =>
                  settings.saveRetention(
                    Number.parseInt(e.currentTarget.value, 10),
                  )}
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
    {:else if analytics.showAnalytics}
      <section class="analytics">
        <h2>Usage Analytics</h2>

        <div class="analytics-controls">
          <div class="time-range-selector">
            <button
              class="range-btn"
              class:active={analytics.timeRange === "1h"}
              onclick={() => analytics.changeTimeRange("1h")}>1h</button
            >
            <button
              class="range-btn"
              class:active={analytics.timeRange === "6h"}
              onclick={() => analytics.changeTimeRange("6h")}>6h</button
            >
            <button
              class="range-btn"
              class:active={analytics.timeRange === "24h"}
              onclick={() => analytics.changeTimeRange("24h")}>24h</button
            >
            <button
              class="range-btn"
              class:active={analytics.timeRange === "7d"}
              onclick={() => analytics.changeTimeRange("7d")}>7d</button
            >
            <button
              class="range-btn"
              class:active={analytics.timeRange === "30d"}
              onclick={() => analytics.changeTimeRange("30d")}>30d</button
            >
          </div>

          <div class="usage-type-filter">
            <label class="filter-item" style="--color: #3b82f6">
              <input type="checkbox" bind:checked={analytics.showFiveHour} />
              <span>5h</span>
            </label>
            <label class="filter-item" style="--color: #8b5cf6">
              <input type="checkbox" bind:checked={analytics.showSevenDay} />
              <span>7d</span>
            </label>
            <label class="filter-item" style="--color: #22c55e">
              <input type="checkbox" bind:checked={analytics.showSonnet} />
              <span>Sonnet</span>
            </label>
            <label class="filter-item" style="--color: #f59e0b">
              <input type="checkbox" bind:checked={analytics.showOpus} />
              <span>Opus</span>
            </label>
          </div>
        </div>

        <div class="chart-section">
          <UsageLineChart
            data={analytics.history}
            height={220}
            showFiveHour={analytics.showFiveHour}
            showSevenDay={analytics.showSevenDay}
            showSonnet={analytics.showSonnet}
            showOpus={analytics.showOpus}
          />
        </div>
      </section>
    {:else}
      <section class="dashboard">
        <div class="refresh-row">
          <div class="update-info">
            <span class="last-update"
              >Updated: {formatSecondsAgo(
                usageData.secondsSinceLastUpdate,
              )}</span
            >
            {#if settings.autoRefreshEnabled}
              <span class="next-update"
                >Next: {formatCountdown(usageData.secondsUntilNextUpdate)}</span
              >
            {:else}
              <span class="next-update disabled">Auto-refresh off</span>
            {/if}
          </div>
          <button
            class="refresh-btn"
            onclick={() => usageData.refreshNow()}
            disabled={settings.loading}
          >
            {settings.loading ? "Loading..." : "Refresh"}
          </button>
        </div>

        {#if settings.loading && !usageData.usageData}
          <div class="loading">Loading usage data...</div>
        {:else if settings.error}
          <div class="error">
            <p>{settings.error}</p>
            <button onclick={() => usageData.refreshNow()}>Retry</button>
          </div>
        {:else if usageData.usageData}
          <div class="usage-grid">
            {@render usageCard("5 Hour", usageData.usageData.five_hour)}
            {@render usageCard("7 Day", usageData.usageData.seven_day)}
            {@render usageCard(
              "Sonnet (7 Day)",
              usageData.usageData.seven_day_sonnet,
            )}
            {@render usageCard(
              "Opus (7 Day)",
              usageData.usageData.seven_day_opus,
            )}
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

  .analytics-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-left: 4px;
  }

  .time-range-selector {
    display: flex;
    gap: 4px;
    background: #e5e5e5;
    border-radius: 8px;
    padding: 4px;
  }

  .usage-type-filter {
    display: flex;
    gap: 8px;
    margin-right: 12px;
  }

  .filter-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.7rem;
    color: #666;
    cursor: pointer;
  }

  .filter-item input[type="checkbox"] {
    width: 12px;
    height: 12px;
    accent-color: var(--color);
    cursor: pointer;
  }

  .filter-item span {
    color: var(--color);
    font-weight: 500;
  }

  @media (prefers-color-scheme: dark) {
    .time-range-selector {
      background: #2a2a2a;
    }

    .filter-item {
      color: #999;
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
