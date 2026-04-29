import type { ProviderKind, UsageSnapshot } from "../shared/types";
import { AppError, RATE_LIMITED, normalizeError } from "./errors";
import { saveUsageSnapshot } from "./history";
import { processNotifications, resetNotificationStateIfNeeded } from "./notifications";
import { fetchUsageForProvider } from "./providers";
import { setSetting } from "./settings";
import type { AppState } from "./state";
import type { TrayController } from "./tray";

export type FetchResult = "success" | "rate-limited" | "other-error" | "no-credentials";

export const INITIAL_BACKOFF_SECS = 30;
export const MAX_BACKOFF_SECS = 300;
export const BACKOFF_MULTIPLIER = 2;
export const HOURLY_REFRESH_INITIAL_GAP_SECS = 5;
export const HOURLY_REFRESH_JITTER_MAX_SECS = 55;

export type UsageEmitter = {
  usageUpdated: (payload: { usage: UsageSnapshot; nextRefreshAt: number | null }) => void;
  usageError: (payload: { provider: ProviderKind; error: string }) => void;
};

export class AutoRefresh {
  private timeout: Timer | null = null;
  private backoffSecs = 0;

  constructor(
    private readonly state: AppState,
    private readonly emitter: UsageEmitter,
    private readonly tray: TrayController,
  ) {}

  start() {
    this.restart();
  }

  restart() {
    this.stop();
    this.backoffSecs = 0;
    void this.loop();
  }

  stop() {
    if (this.timeout) {
      clearTimeout(this.timeout);
      this.timeout = null;
    }
  }

  async refreshNow() {
    const interval = this.state.config.interval_minutes;
    await this.doFetchAndEmit(interval);
    this.restart();
  }

  private async loop() {
    const { enabled, interval_minutes: intervalMinutes } = this.state.config;
    const hasCredentials = this.hasProviderConfig();

    if (!shouldRefresh(enabled, hasCredentials)) {
      return;
    }

    const output = await this.doFetchAndEmit(intervalMinutes);
    this.backoffSecs = calculateNextBackoff(this.backoffSecs, output.result);

    const waitMs =
      this.backoffSecs > 0
        ? this.backoffSecs * 1000
        : output.nextRefreshAt
          ? Math.max(0, output.nextRefreshAt - Date.now())
          : intervalMinutes * 60_000;

    this.timeout = setTimeout(() => {
      void this.loop();
    }, waitMs);
  }

  private async doFetchAndEmit(intervalMinutes: number) {
    const config = { ...this.state.config };
    if (!this.hasProviderConfig()) {
      return { result: "no-credentials" as FetchResult, nextRefreshAt: null };
    }

    try {
      const usage = await fetchUsageForProvider(
        config.active_provider,
        config.organization_id,
        config.session_token,
        config.ollama_session_token,
      );
      this.tray.updateTooltip(usage);
      saveUsageSnapshot(usage);

      const previousNotificationState = this.state.notificationState;
      const nextNotificationState = processNotifications(
        usage,
        this.state.notificationSettings,
        resetNotificationStateIfNeeded(
          usage,
          previousNotificationState,
        ),
      );

      this.state.notificationState = nextNotificationState;
      if (notificationStateChanged(previousNotificationState, nextNotificationState)) {
        setSetting("notification_state", nextNotificationState);
      }

      const hourlyDelay = calculateHourlyRefreshDelay(config.hourly_refresh_enabled);
      const nextRefreshAt = calculateNextRefreshAt(
        config.enabled,
        intervalMinutes,
        Date.now(),
        hourlyDelay,
      );
      this.emitter.usageUpdated({ usage, nextRefreshAt });
      return { result: "success" as FetchResult, nextRefreshAt };
    } catch (error) {
      const message = normalizeError(error);
      const isRateLimited =
        message === RATE_LIMITED || (error instanceof AppError && error.message === RATE_LIMITED);
      const nextRefreshAt = calculateNextRefreshAt(
        config.enabled,
        intervalMinutes,
        Date.now(),
        calculateHourlyRefreshDelay(config.hourly_refresh_enabled),
      );
      this.emitter.usageError({ provider: config.active_provider, error: message });
      return {
        result: (isRateLimited ? "rate-limited" : "other-error") as FetchResult,
        nextRefreshAt,
      };
    }
  }

  private hasProviderConfig() {
    const config = this.state.config;
    switch (config.active_provider) {
      case "claude":
        return Boolean(config.organization_id && config.session_token);
      case "codex":
        return true;
      case "ollama":
        return Boolean(config.ollama_session_token);
    }
  }
}

export function notificationStateChanged(
  previous: AppState["notificationState"],
  next: AppState["notificationState"],
): boolean {
  return JSON.stringify(previous) !== JSON.stringify(next);
}

export function shouldRefresh(enabled: boolean, hasCredentials: boolean): boolean {
  return enabled && hasCredentials;
}

export function calculateNextBackoff(currentBackoff: number, result: FetchResult): number {
  if (result === "success") {
    return 0;
  }
  if (result === "rate-limited") {
    return currentBackoff === 0
      ? INITIAL_BACKOFF_SECS
      : Math.min(currentBackoff * BACKOFF_MULTIPLIER, MAX_BACKOFF_SECS);
  }
  return currentBackoff;
}

export function calculateHourlyRefreshDelay(
  hourlyRefreshEnabled: boolean,
  now = new Date(),
  jitter = Math.floor(Math.random() * (HOURLY_REFRESH_JITTER_MAX_SECS + 1)),
): number | null {
  if (!hourlyRefreshEnabled) {
    return null;
  }
  const secondsIntoHour = now.getUTCMinutes() * 60 + now.getUTCSeconds();
  return 3600 - secondsIntoHour + HOURLY_REFRESH_INITIAL_GAP_SECS + jitter;
}

export function calculateNextRefreshAt(
  enabled: boolean,
  intervalMinutes: number,
  now: number,
  hourlyDelaySecs: number | null,
): number | null {
  if (!enabled) {
    return null;
  }
  const regularNext = now + intervalMinutes * 60_000;
  if (hourlyDelaySecs === null) {
    return regularNext;
  }
  return Math.min(regularNext, now + hourlyDelaySecs * 1000);
}
