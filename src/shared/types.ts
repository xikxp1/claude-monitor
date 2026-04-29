import type { RPCSchema } from "electrobun/view";

export type ProviderKind = "claude" | "codex" | "ollama";

export type NotificationRule = {
  interval_enabled: boolean;
  interval_percent: number;
  threshold_enabled: boolean;
  thresholds: number[];
  time_remaining_enabled: boolean;
  time_remaining_minutes: number[];
};

export type NotificationSettings = {
  enabled: boolean;
  rules: Record<string, NotificationRule>;
};

export type NotificationState = {
  last_notified: Record<string, number>;
  fired_thresholds: string[];
  fired_time_remaining: string[];
};

export type ProviderStatus = {
  provider: ProviderKind;
  configured: boolean;
  source: string;
  message: string | null;
};

export type Settings = {
  active_provider: ProviderKind;
  refresh_interval_minutes: number;
};

export type UsageWindow = {
  key: string;
  label: string;
  utilization: number;
  resetsAt: string | null;
  windowDurationSeconds: number | null;
};

export type UsageSnapshot = {
  provider: ProviderKind;
  windows: UsageWindow[];
  accountEmail: string | null;
  planType: string | null;
};

export type UsageHistoryPoint = {
  id: number;
  provider: ProviderKind;
  timestamp: string;
  windowKey: string;
  label: string;
  utilization: number;
  resetsAt: string | null;
};

export type WindowStats = {
  key: string;
  label: string;
  current: number | null;
  change: number | null;
  velocity: number | null;
};

export type UsageStats = {
  windows: WindowStats[];
  recordCount: number;
  periodHours: number;
};

export type UpdateInfo = {
  version: string;
  hash: string;
  updateAvailable: boolean;
  updateReady: boolean;
  error: string;
};

export type UsageUpdateEvent = {
  usage: UsageSnapshot;
  nextRefreshAt: number | null;
};

export type UsageErrorEvent = {
  provider: ProviderKind;
  error: string;
};

export type CommandResult<T, E = string | null> =
  | { status: "ok"; data: T }
  | { status: "error"; error: E };

export type AppSettings = Record<string, unknown>;

export type AppRPC = {
  bun: RPCSchema<{
    requests: {
      getUsage: {
        params: {
          provider: ProviderKind;
          orgId: string | null;
          sessionToken: string | null;
          ollamaSessionToken: string | null;
        };
        response: CommandResult<UsageSnapshot, string>;
      };
      getDefaultSettings: {
        params: Record<string, never>;
        response: Settings;
      };
      saveCredentials: {
        params: { orgId: string; sessionToken: string };
        response: CommandResult<null, string>;
      };
      clearCredentials: {
        params: Record<string, never>;
        response: CommandResult<null, string>;
      };
      saveOllamaCredentials: {
        params: { sessionToken: string };
        response: CommandResult<null, string>;
      };
      clearOllamaCredentials: {
        params: Record<string, never>;
        response: CommandResult<null, string>;
      };
      getProviderStatuses: {
        params: Record<string, never>;
        response: CommandResult<ProviderStatus[], null>;
      };
      setActiveProvider: {
        params: { provider: ProviderKind };
        response: CommandResult<null, null>;
      };
      setAutoRefresh: {
        params: { enabled: boolean; intervalMinutes: number };
        response: CommandResult<null, null>;
      };
      setHourlyRefresh: {
        params: { enabled: boolean };
        response: CommandResult<null, null>;
      };
      refreshNow: {
        params: Record<string, never>;
        response: CommandResult<null, null>;
      };
      setNotificationSettings: {
        params: { settings: NotificationSettings };
        response: CommandResult<null, null>;
      };
      getUsageHistoryByRange: {
        params: { provider: ProviderKind; range: string };
        response: CommandResult<UsageHistoryPoint[], string>;
      };
      getUsageStats: {
        params: { provider: ProviderKind; range: string };
        response: CommandResult<UsageStats, string>;
      };
      cleanupHistory: {
        params: { retentionDays: number };
        response: CommandResult<number, string>;
      };
      storeGet: {
        params: { key: string };
        response: unknown;
      };
      storeSet: {
        params: { key: string; value: unknown };
        response: null;
      };
      storeClear: {
        params: Record<string, never>;
        response: null;
      };
      isAutostartEnabled: {
        params: Record<string, never>;
        response: boolean;
      };
      setAutostartEnabled: {
        params: { enabled: boolean };
        response: null;
      };
      checkForUpdates: {
        params: Record<string, never>;
        response: CommandResult<UpdateInfo, string>;
      };
      downloadAndInstallUpdate: {
        params: Record<string, never>;
        response: CommandResult<UpdateInfo, string>;
      };
      restartApp: {
        params: Record<string, never>;
        response: CommandResult<null, string>;
      };
    };
    messages: {
      browserReady: Record<string, never>;
    };
  }>;
  webview: RPCSchema<{
    requests: Record<string, never>;
    messages: {
      usageUpdated: { usage: UsageSnapshot; nextRefreshAt: number | null };
      usageError: { provider: ProviderKind; error: string };
      checkForUpdates: Record<string, never>;
    };
  }>;
};
