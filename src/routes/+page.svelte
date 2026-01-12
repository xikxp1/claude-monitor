<script lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { LazyStore } from "@tauri-apps/plugin-store";
import { onMount } from "svelte";
import type {
  Settings,
  UsageData,
  UsagePeriod,
  NotificationSettings,
  NotificationState,
} from "$lib/types";
import {
  getDefaultNotificationSettings,
  getDefaultNotificationState,
} from "$lib/types";
import NotificationSettingsComponent from "$lib/components/NotificationSettings.svelte";
import {
  processNotifications,
  resetNotificationStateIfNeeded,
} from "$lib/notifications";

let settings: Settings = $state({
  organization_id: null,
  session_token: null,
  refresh_interval_minutes: 5,
});
let usageData: UsageData | null = $state(null);
let loading = $state(false);
let error: string | null = $state(null);
let showSettings = $state(false);
let settingsTab: "credentials" | "notifications" = $state("credentials");

// Notification state
let notificationSettings: NotificationSettings = $state(getDefaultNotificationSettings());
let notificationState: NotificationState = $state(getDefaultNotificationState());

// Form inputs
let orgIdInput = $state("");
let tokenInput = $state("");

const isConfigured = $derived(settings.organization_id !== null && settings.session_token !== null);

const store = new LazyStore("settings.json", { autoSave: true, defaults: {} });

let unlistenFn: UnlistenFn | null = null;

function getUsageColor(utilization: number): string {
  if (utilization >= 80) return "red";
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

onMount(() => {
  initApp();

  return () => {
    if (unlistenFn) {
      unlistenFn();
    }
  };
});

async function initApp() {
  // Load credentials
  const savedOrgId = await store.get<string>("organization_id");
  const savedToken = await store.get<string>("session_token");
  const savedInterval = await store.get<number>("refresh_interval_minutes");

  settings = {
    organization_id: savedOrgId ?? null,
    session_token: savedToken ?? null,
    refresh_interval_minutes: savedInterval ?? 5,
  };

  orgIdInput = settings.organization_id ?? "";
  tokenInput = settings.session_token ?? "";

  // Load notification settings
  const savedNotificationSettings = await store.get<NotificationSettings>("notification_settings");
  if (savedNotificationSettings) {
    notificationSettings = savedNotificationSettings;
  }

  // Load notification state
  const savedNotificationState = await store.get<NotificationState>("notification_state");
  if (savedNotificationState) {
    notificationState = savedNotificationState;
  }

  if (settings.organization_id && settings.session_token) {
    await fetchUsage();
  }

  unlistenFn = await listen("refresh-usage", () => {
    fetchUsage();
  });
}

async function saveSettings() {
  loading = true;
  error = null;

  try {
    await store.set("organization_id", orgIdInput);
    await store.set("session_token", tokenInput);
    await store.set("refresh_interval_minutes", settings.refresh_interval_minutes);

    settings = {
      ...settings,
      organization_id: orgIdInput,
      session_token: tokenInput,
    };

    showSettings = false;
    await fetchUsage();
  } catch (e) {
    error = e instanceof Error ? e.message : "Failed to save settings";
  } finally {
    loading = false;
  }
}

async function saveNotificationSettings(newSettings: NotificationSettings) {
  notificationSettings = newSettings;
  await store.set("notification_settings", newSettings);
}

async function fetchUsage() {
  if (!settings.organization_id || !settings.session_token) {
    return;
  }

  loading = true;
  error = null;

  try {
    const newUsageData = await invoke<UsageData>("get_usage", {
      orgId: settings.organization_id,
      sessionToken: settings.session_token,
    });

    usageData = newUsageData;

    // Check for usage resets and clear notification state if needed
    notificationState = resetNotificationStateIfNeeded(newUsageData, notificationState);

    // Process notifications
    const newNotificationState = await processNotifications(
      newUsageData,
      notificationSettings,
      notificationState,
    );

    if (newNotificationState !== notificationState) {
      notificationState = newNotificationState;
      await store.set("notification_state", notificationState);
    }
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    usageData = null;
  } finally {
    loading = false;
  }
}

async function clearSettings() {
  await store.clear();
  settings = {
    organization_id: null,
    session_token: null,
    refresh_interval_minutes: 5,
  };
  orgIdInput = "";
  tokenInput = "";
  usageData = null;
  notificationSettings = getDefaultNotificationSettings();
  notificationState = getDefaultNotificationState();
  showSettings = false;
}
</script>

{#snippet usageCard(title: string, period: UsagePeriod | null)}
  {#if period}
    {@const color = getUsageColor(period.utilization)}
    <div class="usage-card">
      <div class="card-header">
        <span class="card-title">{title}</span>
        <span class="reset-time">Resets in {formatResetTime(period.resets_at)}</span>
      </div>
      <div class="usage-bar-container">
        <div
          class="usage-bar"
          class:green={color === "green"}
          class:yellow={color === "yellow"}
          class:red={color === "red"}
          style="width: {Math.min(period.utilization, 100)}%"
        ></div>
      </div>
      <div class="usage-percent">{period.utilization.toFixed(0)}%</div>
    </div>
  {/if}
{/snippet}

<main class="container">
  <header>
    <h1>Claude Monitor</h1>
    {#if isConfigured}
      <button class="icon-btn" onclick={() => (showSettings = !showSettings)}>
        {showSettings ? "Dashboard" : "Settings"}
      </button>
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
        </div>
      {/if}

      {#if settingsTab === "credentials" || !isConfigured}
        <p class="hint">
          Enter your Claude organization ID and session token to view usage.
        </p>

        <form onsubmit={(e) => { e.preventDefault(); saveSettings(); }}>
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
            <li>Go to <a href="https://claude.ai" target="_blank">claude.ai</a> and log in</li>
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
      {/if}
    </section>
  {:else}
    <section class="dashboard">
      <div class="refresh-row">
        <button class="refresh-btn" onclick={fetchUsage} disabled={loading}>
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {#if loading && !usageData}
        <div class="loading">Loading usage data...</div>
      {:else if error}
        <div class="error">
          <p>{error}</p>
          <button onclick={fetchUsage}>Retry</button>
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
    font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
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
    margin-bottom: 16px;
    padding-bottom: 12px;
    border-bottom: 1px solid #e0e0e0;
  }

  @media (prefers-color-scheme: dark) {
    header {
      border-bottom-color: #333;
    }
  }

  h1 {
    margin: 0;
    font-size: 1.3rem;
    font-weight: 600;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .icon-btn {
    padding: 6px 12px;
    font-size: 0.85rem;
    background: transparent;
    border: 1px solid #ddd;
    border-radius: 6px;
    cursor: pointer;
  }

  @media (prefers-color-scheme: dark) {
    .icon-btn {
      border-color: #444;
      color: #f0f0f0;
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
  }

  @media (prefers-color-scheme: dark) {
    .tab {
      color: #999;
    }
  }

  .tab.active {
    background: #fff;
    color: #1a1a1a;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  @media (prefers-color-scheme: dark) {
    .tab.active {
      background: #3a3a3a;
      color: #f0f0f0;
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

  .dashboard {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1;
    overflow-y: auto;
  }

  .refresh-row {
    display: flex;
    justify-content: flex-end;
  }

  .refresh-btn {
    padding: 6px 14px;
    font-size: 0.85rem;
    background: #f0f0f0;
    color: #333;
  }

  @media (prefers-color-scheme: dark) {
    .refresh-btn {
      background: #3a3a3a;
      color: #f0f0f0;
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

  .usage-bar.red {
    background: #ef4444;
  }

  .usage-percent {
    margin-top: 8px;
    font-size: 1.2rem;
    font-weight: 600;
    text-align: center;
  }
</style>
