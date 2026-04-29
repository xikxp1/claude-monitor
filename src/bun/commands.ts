import type {
  CommandResult,
  NotificationSettings,
  ProviderKind,
  Settings,
  UsageSnapshot,
} from "../shared/types";
import { AutoRefresh } from "./autoRefresh";
import {
  deleteCredentials,
  deleteOllamaCredentials,
  saveCredentials as saveKeychainCredentials,
  saveOllamaCredentials as saveKeychainOllamaCredentials,
} from "./credentials";
import { normalizeError } from "./errors";
import { cleanupOldData, getUsageHistoryByRange, getUsageStats } from "./history";
import { setAutostartEnabled, isAutostartEnabled } from "./platform/autostart";
import { fetchUsageForProvider, getProviderStatuses as collectProviderStatuses } from "./providers";
import { clearSettings, getSetting, setSetting } from "./settings";
import type { AppState } from "./state";
import { validateOrgId, validateSessionToken } from "./validation";
import {
  checkForUpdates,
  downloadAndInstallUpdate,
  restartApp,
} from "./updates";

export function createCommandHandlers(state: AppState, autoRefresh: AutoRefresh) {
  const ok = <T>(data: T): CommandResult<T, never> => ({ status: "ok", data });
  const nullableOk = ok(null);

  function fail(error: unknown): CommandResult<never, string> {
    return { status: "error", error: normalizeError(error) };
  }

  return {
    async getUsage({
      provider,
      orgId,
      sessionToken,
      ollamaSessionToken,
    }: {
      provider: ProviderKind;
      orgId: string | null;
      sessionToken: string | null;
      ollamaSessionToken: string | null;
    }): Promise<CommandResult<UsageSnapshot, string>> {
      try {
        return ok(await fetchUsageForProvider(provider, orgId, sessionToken, ollamaSessionToken));
      } catch (error) {
        return fail(error);
      }
    },

    getDefaultSettings(): Settings {
      return {
        active_provider: "claude",
        refresh_interval_minutes: 5,
      };
    },

    async saveCredentials({ orgId, sessionToken }: { orgId: string; sessionToken: string }) {
      try {
        validateOrgId(orgId);
        validateSessionToken(sessionToken);
        saveKeychainCredentials(orgId, sessionToken);
        state.config.organization_id = orgId;
        state.config.session_token = sessionToken;
        autoRefresh.restart();
        return nullableOk;
      } catch (error) {
        return fail(error);
      }
    },

    async clearCredentials() {
      try {
        deleteCredentials();
        state.config.organization_id = null;
        state.config.session_token = null;
        autoRefresh.restart();
        return nullableOk;
      } catch (error) {
        return fail(error);
      }
    },

    async saveOllamaCredentials({ sessionToken }: { sessionToken: string }) {
      try {
        validateSessionToken(sessionToken);
        saveKeychainOllamaCredentials(sessionToken);
        state.config.ollama_session_token = sessionToken;
        autoRefresh.restart();
        return nullableOk;
      } catch (error) {
        return fail(error);
      }
    },

    async clearOllamaCredentials() {
      try {
        deleteOllamaCredentials();
        state.config.ollama_session_token = null;
        autoRefresh.restart();
        return nullableOk;
      } catch (error) {
        return fail(error);
      }
    },

    async getProviderStatuses() {
      return ok(
        collectProviderStatuses(
          state.config.organization_id,
          state.config.session_token,
          state.config.ollama_session_token,
        ),
      );
    },

    async setActiveProvider({ provider }: { provider: ProviderKind }) {
      state.config.active_provider = provider;
      autoRefresh.restart();
      return nullableOk;
    },

    async setAutoRefresh({
      enabled,
      intervalMinutes,
    }: {
      enabled: boolean;
      intervalMinutes: number;
    }) {
      state.config.enabled = enabled;
      state.config.interval_minutes = intervalMinutes;
      autoRefresh.restart();
      return nullableOk;
    },

    async setHourlyRefresh({ enabled }: { enabled: boolean }) {
      state.config.hourly_refresh_enabled = enabled;
      autoRefresh.restart();
      return nullableOk;
    },

    async refreshNow() {
      await autoRefresh.refreshNow();
      return nullableOk;
    },

    async setNotificationSettings({ settings }: { settings: NotificationSettings }) {
      state.notificationSettings = settings;
      return nullableOk;
    },

    async getUsageHistoryByRange({ provider, range }: { provider: ProviderKind; range: string }) {
      try {
        return ok(getUsageHistoryByRange(provider, range));
      } catch (error) {
        return fail(error);
      }
    },

    async getUsageStats({ provider, range }: { provider: ProviderKind; range: string }) {
      try {
        return ok(getUsageStats(provider, range));
      } catch (error) {
        return fail(error);
      }
    },

    async cleanupHistory({ retentionDays }: { retentionDays: number }) {
      try {
        return ok(cleanupOldData(retentionDays));
      } catch (error) {
        return fail(error);
      }
    },

    storeGet({ key }: { key: string }) {
      return getSetting(key) ?? null;
    },

    storeSet({ key, value }: { key: string; value: unknown }) {
      setSetting(key, value);
      return null;
    },

    storeClear() {
      clearSettings();
      return null;
    },

    isAutostartEnabled() {
      return isAutostartEnabled();
    },

    setAutostartEnabled({ enabled }: { enabled: boolean }) {
      setAutostartEnabled(enabled);
      return null;
    },

    checkForUpdates,
    downloadAndInstallUpdate,
    restartApp,
  };
}
