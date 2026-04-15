<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import UsageLineChart from "$lib/components/charts/UsageLineChart.svelte";
  import NotificationSettingsComponent from "$lib/components/NotificationSettings.svelte";
  import ToastContainer from "$lib/components/ToastContainer.svelte";
  import { useAnalytics, useSettings, useToast, useUpdates, useUsageData } from "$lib/composables";
  import { initHistoryStorage } from "$lib/historyStorage";
  import type { ProviderKind, UsageWindow } from "$lib/types";
  import { PROVIDER_LABELS, getProviderWindows } from "$lib/types";
  import {
    formatCountdown,
    formatResetTime,
    formatSecondsAgo,
    getUsageColor,
  } from "$lib/utils";

  const toast = useToast();
  const settings = useSettings({
    onSuccess: (msg) => toast.success(msg),
    onError: (msg) => toast.error(msg),
  });
  const analytics = useAnalytics({
    getActiveProvider: () => settings.activeProvider,
  });
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
  const updates = useUpdates();

  const chartPalette = ["#3b82f6", "#8b5cf6", "#22c55e", "#f59e0b", "#ef4444", "#14b8a6"];

  let initializing = $state(true);
  let showResetConfirm = $state(false);
  let unlistenCheckUpdates: UnlistenFn | null = null;

  let providerWindows = $derived(
    getProviderWindows(settings.activeProvider, usageData.usageData),
  );

  onMount(() => {
    initHistoryStorage();
    void initApp();

    return () => {
      usageData.cleanup();
      unlistenCheckUpdates?.();
    };
  });

  async function initApp() {
    await usageData.setupEventListeners();

    unlistenCheckUpdates = await listen("check-for-updates", async () => {
      analytics.showAnalytics = false;
      settings.showSettings = true;
      settings.settingsTab = "updates";
      await updates.checkForUpdates();
    });

    await settings.init();
    initializing = false;
    usageData.startCountdown();

    if (settings.isConfigured) {
      settings.loading = true;
      await usageData.refreshNow();
    }

    setTimeout(() => {
      void updates.checkForUpdates();
    }, 3000);
  }

  async function handleProviderChange(provider: ProviderKind) {
    if (provider === settings.activeProvider) {
      return;
    }

    await settings.setActiveProvider(provider);
    settings.error = null;
    usageData.reset();
    analytics.resetForProviderSwitch();

    if (settings.isConfigured) {
      settings.loading = true;
      await usageData.refreshNow();
    }
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

  async function handleResetAll() {
    await settings.resetAll();
    usageData.reset();
    analytics.resetForProviderSwitch();
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

  function getChartColor(index: number): string {
    return chartPalette[index % chartPalette.length];
  }

  function providerInstruction(provider: ProviderKind): string {
    if (provider === "claude") {
      return "Enter your Claude organization ID and session token to view usage.";
    }

    if (provider === "ollama") {
      return "Enter your Ollama session cookie to monitor your cloud usage. Find it in your browser cookies on ollama.com.";
    }

    return "Codex monitoring reads your local `~/.codex/auth.json` after you log in with the Codex CLI.";
  }
</script>

{#snippet usageCard(window: UsageWindow)}
  {@const color = getUsageColor(window.utilization)}
  <div class="card bg-base-200 shadow-sm">
    <div class="card-body p-3.5">
      <div class="flex justify-between items-center mb-1.5 gap-3">
        <span class="font-semibold text-[0.9rem]">{window.label}</span>
        <span class="text-xs text-base-content/60 text-right">
          {window.resetsAt
            ? `Resets in ${formatResetTime(window.resetsAt)}`
            : "Starts when activity is detected"}
        </span>
      </div>
      <progress
        class="progress {getProgressClass(color)} h-2.5"
        value={Math.min(window.utilization, 100)}
        max="100"
      ></progress>
      <div class="text-center text-lg font-semibold mt-1.5">
        {window.utilization.toFixed(0)}%
      </div>
    </div>
  </div>
{/snippet}

<main class="h-screen p-3.5 bg-base-100 rounded-xl border border-base-300 shadow-lg flex flex-col overflow-hidden">
  {#if initializing}
    <div class="flex flex-col items-center justify-center flex-1 gap-3">
      <span class="loading loading-spinner loading-md text-primary"></span>
      <span class="text-sm text-base-content/60">Loading...</span>
    </div>
  {:else}
    <header class="flex justify-between items-center mb-2.5 py-1 border-b border-base-300 gap-2">
      <div class="flex items-center gap-3">
        <h1 class="m-0 ml-2 text-[1.15rem] font-semibold tracking-tight">
          <span class="text-secondary">{PROVIDER_LABELS[settings.activeProvider]}</span>
          <span class="text-neutral font-normal"> Monitor</span>
        </h1>
      </div>

      {#if settings.isConfigured}
        <div class="flex gap-1.5 mr-2">
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
              class="join-item btn btn-xs flex-1 {settings.settingsTab === 'account' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "account")}
            >
              Account
            </button>
            <button
              class="join-item btn btn-xs flex-1 {settings.settingsTab === 'notifications' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "notifications")}
            >
              Alerts
            </button>
            <button
              class="join-item btn btn-xs flex-1 {settings.settingsTab === 'general' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "general")}
            >
              General
            </button>
            <button
              class="join-item btn btn-xs flex-1 {settings.settingsTab === 'updates' ? 'btn-primary' : 'btn-ghost'}"
              onclick={() => (settings.settingsTab = "updates")}
            >
              Updates
            </button>
          </div>
        {/if}

        {#if settings.settingsTab === "account" || !settings.isConfigured}
          <div class="join w-full mb-4">
            {#each ["claude", "codex", "ollama"] as provider (provider)}
              <button
                class="join-item btn btn-sm flex-1 {settings.activeProvider === provider ? 'btn-primary' : 'btn-ghost'}"
                onclick={() => handleProviderChange(provider as ProviderKind)}
              >
                {PROVIDER_LABELS[provider as ProviderKind]}
              </button>
            {/each}
          </div>

          <p class="text-sm text-base-content/60 mb-4">
            {providerInstruction(settings.activeProvider)}
          </p>

          {#if settings.activeProvider === "claude"}
            <form
              class="flex flex-col gap-3"
              onsubmit={(event) => {
                event.preventDefault();
                void settings.saveCredentials();
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
                {#if settings.providerStatuses.claude.configured}
                  <button type="button" class="btn btn-ghost" onclick={handleLogout}>
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
          {:else if settings.activeProvider === "ollama"}
            <form
              class="flex flex-col gap-3"
              onsubmit={(event) => {
                event.preventDefault();
                void settings.saveOllamaCredentials();
              }}
            >
              <label class="form-control w-full">
                <div class="label">
                  <span class="label-text font-medium">Session Cookie</span>
                </div>
                <input
                  type="password"
                  class="input input-bordered w-full"
                  bind:value={settings.ollamaTokenInput}
                  placeholder="Your Ollama session cookie value"
                  required
                />
              </label>

              <div class="flex gap-2 mt-2">
                <button type="submit" class="btn btn-primary" disabled={settings.loading}>
                  {settings.loading ? "Saving..." : "Save"}
                </button>
                {#if settings.providerStatuses.ollama.configured}
                  <button type="button" class="btn btn-ghost" onclick={settings.logoutOllama}>
                    Log Out
                  </button>
                {/if}
              </div>
            </form>

            <div class="collapse collapse-arrow bg-base-200 mt-3 min-h-0">
              <input type="checkbox" />
              <div class="collapse-title text-xs font-medium py-2 min-h-0">
                How to get your session cookie
              </div>
              <div class="collapse-content text-xs text-base-content/70 !pb-2">
                <ol class="list-decimal pl-4 space-y-0.5">
                  <li>Go to <a href="https://ollama.com/settings" target="_blank" class="link link-primary">ollama.com</a> and log in</li>
                  <li>Open browser DevTools (F12)</li>
                  <li>Go to Application &gt; Cookies &gt; ollama.com</li>
                  <li>Find your '__Secure-session' cookie and copy its value</li>
                </ol>
              </div>
            </div>
          {:else}
            <div class="card bg-base-200 shadow-sm">
              <div class="card-body p-4 gap-3">
                <div class="flex items-center justify-between gap-2">
                  <div>
                    <h3 class="font-semibold">Codex CLI Status</h3>
                    <p class="text-xs text-base-content/60">
                      Source: {settings.activeProviderStatus.source}
                    </p>
                  </div>
                  <span class="badge {settings.activeProviderStatus.configured ? 'badge-success' : 'badge-warning'}">
                    {settings.activeProviderStatus.configured ? "Ready" : "Needs Login"}
                  </span>
                </div>
                <p class="text-sm text-base-content/70">
                  {settings.activeProviderStatus.message ?? "Local Codex auth detected. Refresh to load usage."}
                </p>
                <div class="flex gap-2">
                  <button
                    type="button"
                    class="btn btn-primary btn-sm"
                    onclick={() => settings.refreshProviderStatuses()}
                  >
                    Recheck Auth
                  </button>
                  {#if settings.activeProviderStatus.configured}
                    <button
                      type="button"
                      class="btn btn-ghost btn-sm"
                      onclick={() => usageData.refreshNow()}
                    >
                      Refresh Now
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          {/if}
        {:else if settings.settingsTab === "notifications"}
          <div class="mt-2">
            <NotificationSettingsComponent
              settings={settings.notificationSettings}
              provider={settings.activeProvider}
              windows={providerWindows}
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
                onchange={(event) =>
                  settings.saveGeneral(
                    event.currentTarget.checked,
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
                  onchange={(event) =>
                    settings.saveGeneral(
                      settings.autoRefreshEnabled,
                      Number.parseInt(event.currentTarget.value, 10),
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
                checked={settings.hourlyRefreshEnabled}
                onchange={(event) =>
                  settings.toggleHourlyRefresh(event.currentTarget.checked)}
              />
              <div class="flex flex-col">
                <span class="font-medium">Refresh when hour starts</span>
                <span class="text-xs text-base-content/60">Auto-refresh in the first minute of every hour</span>
              </div>
            </label>

            <label class="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                class="checkbox checkbox-primary checkbox-sm"
                checked={settings.autostartEnabled}
                onchange={(event) =>
                  settings.toggleAutostart(event.currentTarget.checked)}
              />
              <span class="font-medium">Start at login</span>
            </label>

            <label class="flex items-center justify-between gap-3">
              <span class="text-sm">Data retention</span>
              <select
                class="select select-bordered select-sm"
                value={settings.dataRetentionDays}
                onchange={(event) =>
                  settings.saveRetention(
                    Number.parseInt(event.currentTarget.value, 10),
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
                  This will clear your Claude credentials and reset all settings to defaults.
                </p>
                <div class="flex gap-2">
                  <button type="button" class="btn btn-error btn-sm" onclick={handleResetAll}>
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
                <button class="btn btn-primary btn-sm" onclick={() => updates.downloadAndInstall()}>
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
                <button class="btn btn-success btn-sm" onclick={() => updates.restartApp()}>
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
              <button class="btn btn-ghost btn-sm" onclick={() => updates.reset()}>
                Try Again
              </button>
            {/if}
          </div>
        {/if}
      </section>
    {:else if analytics.showAnalytics}
      <section class="flex-1 overflow-y-auto flex flex-col gap-4 px-2">
        <h2 class="text-lg font-semibold">{PROVIDER_LABELS[settings.activeProvider]} Analytics</h2>

        <div class="flex justify-between items-center gap-2">
          <div class="join">
            {#each ["1h", "6h", "24h", "7d", "30d"] as range (range)}
              <button
                class="join-item btn btn-sm {analytics.timeRange === range ? 'btn-primary' : 'btn-ghost'}"
                onclick={() => analytics.changeTimeRange(range as "1h" | "6h" | "24h" | "7d" | "30d")}
              >
                {range}
              </button>
            {/each}
          </div>

          <div class="flex flex-wrap gap-2 justify-end">
            {#each analytics.availableWindows as window, index (window.key)}
              <label class="flex items-center gap-1 cursor-pointer text-xs">
                <input
                  type="checkbox"
                  class="checkbox checkbox-xs"
                  style="--chkbg: {getChartColor(index)}; --chkfg: white;"
                  checked={analytics.filters[window.key] !== false}
                  onchange={(event) =>
                    analytics.setWindowFilter(window.key, event.currentTarget.checked)}
                />
                <span style="color: {getChartColor(index)}" class="font-medium">{window.label}</span>
              </label>
            {/each}
          </div>
        </div>

        <div class="card bg-base-200 shadow-sm">
          <div class="card-body p-3">
            <UsageLineChart data={analytics.history} filters={analytics.filters} height={220} />
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
              <span>Next in: {formatCountdown(usageData.secondsUntilNextUpdate)}</span>
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
        {:else if settings.isAuthExpired}
          <div class="card bg-base-200 shadow-sm">
            <div class="card-body p-4 items-center text-center">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 text-warning mb-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              <h3 class="font-semibold text-base">Authentication Needed</h3>
              <p class="text-sm text-base-content/70 mb-2">
                {#if settings.activeProvider === "claude"}
                  Your Claude session has expired. Update your session token to continue.
                {:else if settings.activeProvider === "ollama"}
                  Your Ollama session has expired. Update your session cookie to continue.
                {:else}
                  Your Codex login appears to be expired. Run `codex login` and then recheck auth.
                {/if}
              </p>
              {#if settings.activeProvider === "claude" || settings.activeProvider === "ollama"}
                <button class="btn btn-primary btn-sm" onclick={() => settings.openCredentials()}>
                  Update {settings.activeProvider === "claude" ? "Token" : "Cookie"}
                </button>
              {:else}
                <button class="btn btn-primary btn-sm" onclick={() => settings.refreshProviderStatuses()}>
                  Recheck Codex Auth
                </button>
              {/if}
            </div>
          </div>
        {:else}
          {#if settings.error}
            <div class="alert alert-error text-sm py-2">
              <span>{settings.error}</span>
              <button class="btn btn-sm btn-ghost" onclick={() => usageData.refreshNow()}>Retry</button>
            </div>
          {/if}

          {#if usageData.usageData && usageData.usageData.windows.length > 0}
            <div class="flex flex-col gap-2.5">
              {#each usageData.usageData.windows as window (window.key)}
                {@render usageCard(window)}
              {/each}
            </div>
          {:else}
            <div class="text-center text-sm text-base-content/60 py-10">
              No usage data available for {PROVIDER_LABELS[settings.activeProvider]}.
            </div>
          {/if}
        {/if}
      </section>
    {/if}
  {/if}

  <ToastContainer toasts={toast.toasts} onDismiss={toast.dismiss} />
</main>
