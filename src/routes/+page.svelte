<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import UsageLineChart from "$lib/components/charts/UsageLineChart.svelte";
  import NotificationSettingsComponent from "$lib/components/NotificationSettings.svelte";
  import ToastContainer from "$lib/components/ToastContainer.svelte";
  import { useAnalytics, useSettings, useToast, useUpdates, useUsageData } from "$lib/composables";
  import { initHistoryStorage } from "$lib/historyStorage";
  import type { UsagePeriod } from "$lib/types";
  import {
    formatCountdown,
    formatResetTime,
    formatSecondsAgo,
    getUsageColor,
  } from "$lib/utils";

  // Initialize composables
  const toast = useToast();
  const settings = useSettings({
    onSuccess: (msg) => toast.success(msg),
    onError: (msg) => toast.error(msg),
  });
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

  // Updates composable for auto-update functionality
  const updates = useUpdates();

  let initializing = $state(true);
  let unlistenCheckUpdates: UnlistenFn | null = null;

  onMount(() => {
    initHistoryStorage();
    initApp();

    return () => {
      usageData.cleanup();
      unlistenCheckUpdates?.();
    };
  });

  async function initApp() {
    // Set up event listeners for backend events
    await usageData.setupEventListeners();

    // Listen for check-for-updates event from tray menu
    unlistenCheckUpdates = await listen("check-for-updates", async () => {
      // Open settings to Updates tab and trigger check
      analytics.showAnalytics = false;
      settings.showSettings = true;
      settings.settingsTab = "updates";
      await updates.checkForUpdates();
    });

    // Initialize settings (loads from store and backend)
    await settings.init();

    initializing = false;

    // Start countdown timer
    usageData.startCountdown();

    // Check for updates in background after a short delay
    setTimeout(() => {
      updates.checkForUpdates();
    }, 3000);
  }

  function openAnalytics() {
    analytics.open();
    settings.showSettings = false;
  }

  function openSettings() {
    settings.toggle();
    analytics.showAnalytics = false;
  }

  async function handleLogout() {
    await settings.logout();
    usageData.reset();
  }

  let showResetConfirm = $state(false);

  async function handleResetAll() {
    await settings.resetAll();
    usageData.reset();
    showResetConfirm = false;
  }

  function getProgressClass(color: string): string {
    switch (color) {
      case "green":
        return "progress-success";
      case "yellow":
        return "progress-warning";
      case "orange":
        return "progress-orange";
      case "red":
        return "progress-error";
      default:
        return "progress-primary";
    }
  }
</script>

{#snippet usageCard(title: string, period: UsagePeriod | null)}
  {#if period}
    {@const color = getUsageColor(period.utilization)}
    <div class="card bg-base-200 shadow-sm">
      <div class="card-body p-3.5">
        <div class="flex justify-between items-center mb-1.5">
          <span class="font-semibold text-[0.9rem]">{title}</span>
          <span class="text-xs text-base-content/60">
            {period.resets_at
              ? `Resets in ${formatResetTime(period.resets_at)}`
              : "Starts when a message is sent"}
          </span>
        </div>
        <progress
          class="progress {getProgressClass(color)} h-2.5"
          value={Math.min(period.utilization, 100)}
          max="100"
        ></progress>
        <div class="text-center text-lg font-semibold mt-1.5">
          {period.utilization.toFixed(0)}%
        </div>
      </div>
    </div>
  {/if}
{/snippet}

<main class="h-screen p-3.5 bg-base-100 rounded-xl border border-base-300 shadow-lg flex flex-col overflow-hidden">
  {#if initializing}
    <div class="flex flex-col items-center justify-center flex-1 gap-3">
      <span class="loading loading-spinner loading-md text-primary"></span>
      <span class="text-sm text-base-content/60">Loading...</span>
    </div>
  {:else}
    <header class="flex justify-between items-center mb-2.5 py-1 border-b border-base-300">
      <h1 class="m-0 ml-2 mr-3 text-[1.15rem] font-semibold tracking-tight">
        <span class="text-secondary">Claude</span>
        <span class="text-neutral font-normal">Monitor</span>
      </h1>
      {#if settings.isConfigured}
        <div class="flex gap-1.5 mr-1">
          <button
            class="btn btn-sm {analytics.showAnalytics ? 'btn-primary' : 'btn-soft'}"
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
            class="btn btn-sm {settings.showSettings ? 'btn-primary' : 'btn-soft'} relative"
            onclick={openSettings}
          >
            {settings.showSettings ? "Dashboard" : "Settings"}
            {#if updates.isUpdateAvailable && !settings.showSettings}
              <span class="absolute -top-1 -right-1 w-2.5 h-2.5 bg-info rounded-full"></span>
            {/if}
          </button>
        </div>
      {/if}
    </header>

    {#if !settings.isConfigured || settings.showSettings}
      <section class="w-full max-w-xs mx-auto flex-1 overflow-y-auto">
        <h2 class="text-lg font-semibold mb-2">{settings.isConfigured ? "Settings" : "Setup"}</h2>

        {#if settings.isConfigured}
          <div class="join w-full mb-4">
            <button
              class="join-item btn btn-sm flex-1 {settings.settingsTab === 'credentials' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "credentials")}
            >
              Credentials
            </button>
            <button
              class="join-item btn btn-sm flex-1 {settings.settingsTab === 'notifications' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "notifications")}
            >
              Notifications
            </button>
            <button
              class="join-item btn btn-sm flex-1 {settings.settingsTab === 'general' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "general")}
            >
              General
            </button>
            <button
              class="join-item btn btn-sm flex-1 {settings.settingsTab === 'updates' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "updates")}
            >
              Updates
            </button>
          </div>
        {/if}

        {#if settings.settingsTab === "credentials" || !settings.isConfigured}
          <p class="text-sm text-base-content/60 mb-4">
            Enter your Claude organization ID and session token to view usage.
          </p>

          <form
            class="flex flex-col gap-3"
            onsubmit={(e) => {
              e.preventDefault();
              settings.saveCredentials();
            }}
          >
            <label class="form-control w-full">
              <div class="label">
                <span class="label-text font-medium">Organization ID</span>
              </div>
              <input
                type="text"
                class="input input-bordered w-full"
                bind:value={settings.orgIdInput}
                placeholder="uuid-format-org-id"
                required
              />
            </label>

            <label class="form-control w-full">
              <div class="label">
                <span class="label-text font-medium">Session Token</span>
              </div>
              <input
                type="password"
                class="input input-bordered w-full"
                bind:value={settings.tokenInput}
                placeholder="Your session token"
                required
              />
            </label>

            <div class="flex gap-2 mt-2">
              <button type="submit" class="btn btn-primary" disabled={settings.loading}>
                {settings.loading ? "Saving..." : "Save"}
              </button>
              {#if settings.isConfigured}
                <button
                  type="button"
                  class="btn btn-ghost"
                  onclick={handleLogout}
                >
                  Log Out
                </button>
              {/if}
            </div>
          </form>

          <div class="collapse collapse-arrow bg-base-200 mt-3 min-h-0">
            <input type="checkbox" />
            <div class="collapse-title text-xs font-medium py-2 min-h-0">
              How to get your session token
            </div>
            <div class="collapse-content text-xs text-base-content/70 !pb-2">
              <ol class="list-decimal pl-4 space-y-0.5">
                <li>Go to <a href="https://claude.ai" target="_blank" class="link link-primary">claude.ai</a> and log in</li>
                <li>Open browser DevTools (F12)</li>
                <li>Go to Application &gt; Cookies &gt; claude.ai</li>
                <li>Find "sessionKey" cookie and copy its value</li>
              </ol>
            </div>
          </div>
        {:else if settings.settingsTab === "notifications"}
          <div class="mt-2">
            <NotificationSettingsComponent
              settings={settings.notificationSettings}
              onchange={settings.saveNotifications}
            />
          </div>
        {:else if settings.settingsTab === "general"}
          <div class="flex flex-col gap-4 mt-2">
            <label class="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                class="checkbox checkbox-primary checkbox-sm"
                checked={settings.autoRefreshEnabled}
                onchange={(e) =>
                  settings.saveGeneral(
                    e.currentTarget.checked,
                    settings.refreshIntervalMinutes,
                  )}
              />
              <span class="font-medium">Enable auto-refresh</span>
            </label>

            {#if settings.autoRefreshEnabled}
              <label class="flex items-center justify-between gap-3">
                <span class="text-sm">Refresh interval</span>
                <select
                  class="select select-bordered select-sm"
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

            <label class="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                class="checkbox checkbox-primary checkbox-sm"
                checked={settings.autostartEnabled}
                onchange={(e) =>
                  settings.toggleAutostart(e.currentTarget.checked)}
              />
              <span class="font-medium">Start at login</span>
            </label>

            <label class="flex items-center justify-between gap-3">
              <span class="text-sm">Data retention</span>
              <select
                class="select select-bordered select-sm"
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

            <div class="divider my-1"></div>

            {#if !showResetConfirm}
              <button
                type="button"
                class="btn btn-error btn-sm"
                onclick={() => (showResetConfirm = true)}
              >
                Reset All Settings
              </button>
            {:else}
              <div class="bg-error/10 rounded-lg p-3 flex flex-col gap-2">
                <p class="text-sm">
                  This will clear your credentials and reset all settings to defaults.
                </p>
                <div class="flex gap-2">
                  <button
                    type="button"
                    class="btn btn-error btn-sm"
                    onclick={handleResetAll}
                  >
                    Confirm Reset
                  </button>
                  <button
                    type="button"
                    class="btn btn-ghost btn-sm"
                    onclick={() => (showResetConfirm = false)}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            {/if}
          </div>
        {:else if settings.settingsTab === "updates"}
          <div class="flex flex-col gap-4 mt-2">
            {#if updates.status === "idle" || updates.status === "up-to-date"}
              <div class="text-sm text-base-content/60">
                {#if updates.lastChecked}
                  Last checked: {updates.lastChecked.toLocaleString()}
                {:else}
                  Updates have not been checked yet.
                {/if}
              </div>
              {#if updates.status === "up-to-date"}
                <div class="flex items-center gap-2 text-success">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                  </svg>
                  <span class="font-medium">You're up to date!</span>
                </div>
              {/if}
              <button
                class="btn btn-primary btn-sm"
                onclick={() => updates.checkForUpdates()}
                disabled={updates.isChecking}
              >
                Check for Updates
              </button>
            {:else if updates.status === "checking"}
              <div class="flex items-center gap-3">
                <span class="loading loading-spinner loading-sm"></span>
                <span>Checking for updates...</span>
              </div>
            {:else if updates.status === "available"}
              <div class="card bg-base-200 p-4">
                <div class="flex items-center gap-2 text-info mb-2">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd" />
                  </svg>
                  <span class="font-medium">Update Available</span>
                </div>
                {#if updates.availableUpdate}
                  <p class="text-sm mb-3">Version {updates.availableUpdate.version} is available.</p>
                {/if}
                <button
                  class="btn btn-primary btn-sm"
                  onclick={() => updates.downloadAndInstall()}
                >
                  Download and Install
                </button>
              </div>
            {:else if updates.status === "downloading"}
              <div class="flex flex-col gap-2">
                <div class="flex items-center gap-2">
                  <span class="loading loading-spinner loading-sm"></span>
                  <span>Downloading update...</span>
                </div>
                <progress
                  class="progress progress-primary w-full"
                  value={updates.progressPercent}
                  max="100"
                ></progress>
                <div class="text-xs text-base-content/60 text-center">
                  {updates.formatBytes(updates.progress.downloaded)}
                  {#if updates.progress.contentLength}
                    / {updates.formatBytes(updates.progress.contentLength)}
                  {/if}
                  ({updates.progressPercent}%)
                </div>
              </div>
            {:else if updates.status === "ready"}
              <div class="card bg-success/10 p-4">
                <div class="flex items-center gap-2 text-success mb-2">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd" />
                  </svg>
                  <span class="font-medium">Update Ready</span>
                </div>
                <p class="text-sm mb-3">The update has been downloaded and is ready to install.</p>
                <button
                  class="btn btn-success btn-sm"
                  onclick={() => updates.restartApp()}
                >
                  Restart Now
                </button>
              </div>
            {:else if updates.status === "error"}
              <div class="alert alert-error text-sm">
                <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current h-5 w-5" fill="none" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span>{updates.error}</span>
              </div>
              <button
                class="btn btn-ghost btn-sm"
                onclick={() => updates.reset()}
              >
                Try Again
              </button>
            {/if}
          </div>
        {/if}
      </section>
    {:else if analytics.showAnalytics}
      <section class="flex-1 overflow-y-auto flex flex-col gap-4 px-2">
        <h2 class="text-lg font-semibold">Usage Analytics</h2>

        <div class="flex justify-between items-center gap-3">
          <div class="join">
            <button
              class="join-item btn btn-sm {analytics.timeRange === '1h' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => analytics.changeTimeRange("1h")}>1h</button
            >
            <button
              class="join-item btn btn-sm {analytics.timeRange === '6h' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => analytics.changeTimeRange("6h")}>6h</button
            >
            <button
              class="join-item btn btn-sm {analytics.timeRange === '24h' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => analytics.changeTimeRange("24h")}>24h</button
            >
            <button
              class="join-item btn btn-sm {analytics.timeRange === '7d' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => analytics.changeTimeRange("7d")}>7d</button
            >
            <button
              class="join-item btn btn-sm {analytics.timeRange === '30d' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => analytics.changeTimeRange("30d")}>30d</button
            >
          </div>

          <div class="flex gap-3 mr-3">
            <label class="flex items-center gap-1 cursor-pointer text-xs">
              <input type="checkbox" class="checkbox checkbox-xs" style="--chkbg: #3b82f6; --chkfg: white;" bind:checked={analytics.showFiveHour} />
              <span class="text-info font-medium">5h</span>
            </label>
            <label class="flex items-center gap-1 cursor-pointer text-xs">
              <input type="checkbox" class="checkbox checkbox-xs" style="--chkbg: #8b5cf6; --chkfg: white;" bind:checked={analytics.showSevenDay} />
              <span class="text-[#8b5cf6] font-medium">7d</span>
            </label>
            <label class="flex items-center gap-1 cursor-pointer text-xs">
              <input type="checkbox" class="checkbox checkbox-xs" style="--chkbg: #22c55e; --chkfg: white;" bind:checked={analytics.showSonnet} />
              <span class="text-success font-medium">Sonnet</span>
            </label>
            <label class="flex items-center gap-1 cursor-pointer text-xs">
              <input type="checkbox" class="checkbox checkbox-xs" style="--chkbg: #f59e0b; --chkfg: white;" bind:checked={analytics.showOpus} />
              <span class="text-[#f59e0b] font-medium">Opus</span>
            </label>
          </div>
        </div>

        <div class="card bg-base-200 shadow-sm">
          <div class="card-body p-3">
            <UsageLineChart
              data={analytics.history}
              height={220}
              showFiveHour={analytics.showFiveHour}
              showSevenDay={analytics.showSevenDay}
              showSonnet={analytics.showSonnet}
              showOpus={analytics.showOpus}
            />
          </div>
        </div>
      </section>
    {:else}
      <section class="flex flex-col gap-2.5 flex-1 overflow-y-auto px-2">
        {#if updates.isUpdateAvailable}
          <button
            class="alert alert-info text-sm py-2 cursor-pointer hover:brightness-95 transition-all"
            onclick={() => {
              settings.showSettings = true;
              settings.settingsTab = "updates";
            }}
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-11a1 1 0 10-2 0v3.586L7.707 9.293a1 1 0 00-1.414 1.414l3 3a1 1 0 001.414 0l3-3a1 1 0 00-1.414-1.414L11 10.586V7z" clip-rule="evenodd" />
            </svg>
            <span>
              Update available{#if updates.availableUpdate}: v{updates.availableUpdate.version}{/if}
            </span>
            <span class="text-xs opacity-70">Click to update</span>
          </button>
        {/if}

        <div class="flex justify-between items-center">
          <div class="flex flex-col text-xs text-base-content/60">
            <span>Updated: {formatSecondsAgo(usageData.secondsSinceLastUpdate)}</span>
            {#if settings.autoRefreshEnabled}
              <span>Next: {formatCountdown(usageData.secondsUntilNextUpdate)}</span>
            {:else}
              <span class="italic opacity-70">Auto-refresh off</span>
            {/if}
          </div>
          <button
            class="btn btn-sm btn-soft"
            onclick={() => usageData.refreshNow()}
            disabled={settings.loading}
          >
            {settings.loading ? "Loading..." : "Refresh"}
          </button>
        </div>

        {#if settings.loading && !usageData.usageData}
          <div class="text-center text-sm text-base-content/60 py-10">Loading usage data...</div>
        {:else if settings.isSessionExpired}
          <div class="card bg-base-200 shadow-sm">
            <div class="card-body p-4 items-center text-center">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 text-warning mb-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              <h3 class="font-semibold text-base">Session Expired</h3>
              <p class="text-sm text-base-content/70 mb-2">Your Claude session has expired. Please update your session token to continue.</p>
              <button class="btn btn-primary btn-sm" onclick={() => settings.openCredentials()}>
                Update Token
              </button>
            </div>
          </div>
        {:else if settings.error}
          <div class="alert alert-error text-sm py-2">
            <span>{settings.error}</span>
            <button class="btn btn-sm btn-ghost" onclick={() => usageData.refreshNow()}>Retry</button>
          </div>
        {:else if usageData.usageData}
          <div class="flex flex-col gap-2.5">
            {@render usageCard("5 Hour", usageData.usageData.five_hour)}
            {@render usageCard("7 Day", usageData.usageData.seven_day)}
            {@render usageCard("Sonnet (7 Day)", usageData.usageData.seven_day_sonnet)}
            {@render usageCard("Opus (7 Day)", usageData.usageData.seven_day_opus)}
          </div>
        {:else}
          <div class="text-center text-sm text-base-content/60 py-10">No usage data available</div>
        {/if}
      </section>
    {/if}
  {/if}

  <ToastContainer toasts={toast.toasts} onDismiss={toast.dismiss} />
</main>
