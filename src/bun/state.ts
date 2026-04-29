import type { NotificationSettings, NotificationState, ProviderKind } from "../shared/types";
import { loadCredentials, loadOllamaCredentials } from "./credentials";
import { defaultNotificationSettings, defaultNotificationState } from "./notifications";
import { getSetting } from "./settings";

export type AutoRefreshConfig = {
  active_provider: ProviderKind;
  organization_id: string | null;
  session_token: string | null;
  ollama_session_token: string | null;
  enabled: boolean;
  interval_minutes: number;
  hourly_refresh_enabled: boolean;
};

export type AppState = {
  config: AutoRefreshConfig;
  notificationSettings: NotificationSettings;
  notificationState: NotificationState;
};

export function createInitialState(): AppState {
  const credentials = loadCredentials();
  const ollamaToken = loadOllamaCredentials();

  return {
    config: {
      active_provider: getSetting<ProviderKind>("active_provider") ?? "claude",
      organization_id: credentials?.[0] ?? null,
      session_token: credentials?.[1] ?? null,
      ollama_session_token: ollamaToken,
      enabled: getSetting<boolean>("auto_refresh_enabled") ?? true,
      interval_minutes: getSetting<number>("refresh_interval_minutes") ?? 5,
      hourly_refresh_enabled: getSetting<boolean>("hourly_refresh_enabled") ?? false,
    },
    notificationSettings:
      getSetting<NotificationSettings>("notification_settings") ?? defaultNotificationSettings(),
    notificationState:
      getSetting<NotificationState>("notification_state") ?? defaultNotificationState(),
  };
}
