import { Electroview } from "electrobun/view";
import type {
  AppRPC,
  CommandResult,
  NotificationSettings,
  ProviderKind,
  UsageErrorEvent,
  UsageSnapshot,
  UsageUpdateEvent,
} from "../shared/types";

const rpc = Electroview.defineRPC<AppRPC>({
  handlers: {
    requests: {},
    messages: {
      usageUpdated: (payload) => emit("usage-updated", payload),
      usageError: (payload) => emit("usage-error", payload),
      checkForUpdates: () => emit("check-for-updates", {}),
    },
  },
});

const hasElectrobunBridge =
  typeof window !== "undefined" &&
  ("__electrobunBunBridge" in window || "electrobun" in window);

if (hasElectrobunBridge) {
  new Electroview({ rpc });
}

type EventPayloads = {
  "usage-updated": UsageUpdateEvent;
  "usage-error": UsageErrorEvent;
  "check-for-updates": Record<string, never>;
};

type EventCallback<K extends keyof EventPayloads> = (event: { payload: EventPayloads[K] }) => void;

const listeners = new Map<keyof EventPayloads, Set<(event: { payload: never }) => void>>();

export type UnlistenFn = () => void;

export function listen<K extends keyof EventPayloads>(
  eventName: K,
  callback: EventCallback<K>,
): Promise<UnlistenFn> {
  const set = listeners.get(eventName) ?? new Set();
  set.add(callback as (event: { payload: never }) => void);
  listeners.set(eventName, set);

  return Promise.resolve(() => {
    set.delete(callback as (event: { payload: never }) => void);
  });
}

function emit<K extends keyof EventPayloads>(eventName: K, payload: EventPayloads[K]) {
  const set = listeners.get(eventName);
  if (!set) {
    return;
  }
  for (const callback of set) {
    callback({ payload: payload as never });
  }
}

export const commands = {
  getUsage: (
    provider: ProviderKind,
    orgId: string | null,
    sessionToken: string | null,
    ollamaSessionToken: string | null,
  ) => rpc.request.getUsage({ provider, orgId, sessionToken, ollamaSessionToken }),
  getDefaultSettings: () => rpc.request.getDefaultSettings({}),
  saveCredentials: (orgId: string, sessionToken: string) =>
    rpc.request.saveCredentials({ orgId, sessionToken }),
  clearCredentials: () => rpc.request.clearCredentials({}),
  saveOllamaCredentials: (sessionToken: string) =>
    rpc.request.saveOllamaCredentials({ sessionToken }),
  clearOllamaCredentials: () => rpc.request.clearOllamaCredentials({}),
  getProviderStatuses: () => rpc.request.getProviderStatuses({}),
  setActiveProvider: (provider: ProviderKind) => rpc.request.setActiveProvider({ provider }),
  setAutoRefresh: (enabled: boolean, intervalMinutes: number) =>
    rpc.request.setAutoRefresh({ enabled, intervalMinutes }),
  setHourlyRefresh: (enabled: boolean) => rpc.request.setHourlyRefresh({ enabled }),
  refreshNow: () => rpc.request.refreshNow({}),
  setNotificationSettings: (settings: NotificationSettings) =>
    rpc.request.setNotificationSettings({ settings }),
  getUsageHistoryByRange: (provider: ProviderKind, range: string) =>
    rpc.request.getUsageHistoryByRange({ provider, range }),
  getUsageStats: (provider: ProviderKind, range: string) =>
    rpc.request.getUsageStats({ provider, range }),
  cleanupHistory: (retentionDays: number) => rpc.request.cleanupHistory({ retentionDays }),
};

export const appStore = {
  get: <T>(key: string) => rpc.request.storeGet({ key }) as Promise<T | null>,
  set: (key: string, value: unknown) => rpc.request.storeSet({ key, value }),
  clear: () => rpc.request.storeClear({}),
};

export const autostart = {
  isEnabled: () => rpc.request.isAutostartEnabled({}),
  enable: () => rpc.request.setAutostartEnabled({ enabled: true }),
  disable: () => rpc.request.setAutostartEnabled({ enabled: false }),
};

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error"
  | "up-to-date";

export type Update = Awaited<ReturnType<typeof checkForUpdates>> extends CommandResult<infer T>
  ? T
  : never;

export function checkForUpdates() {
  return rpc.request.checkForUpdates({});
}

export function downloadAndInstallUpdate() {
  return rpc.request.downloadAndInstallUpdate({});
}

export function restartApp() {
  return rpc.request.restartApp({});
}

export type { UsageSnapshot };
