import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type { ProviderStatus, UsageSnapshot, UsageWindow } from "../../shared/types";
import { AppError, INVALID_TOKEN, RATE_LIMITED } from "../errors";

type CodexAuthFile = {
  tokens?: {
    access_token?: string | null;
  };
};

type WhamUsageResponse = {
  email?: string | null;
  plan_type?: string | null;
  rate_limit?: {
    primary_window?: WhamRateLimitWindow | null;
    secondary_window?: WhamRateLimitWindow | null;
  } | null;
};

type WhamRateLimitWindow = {
  used_percent: number;
  reset_at?: string | number | null;
  limit_window_seconds?: number | null;
};

export async function fetchCodexUsage(): Promise<UsageSnapshot> {
  const accessToken = loadAccessToken();
  const response = await fetch("https://chatgpt.com/backend-api/wham/usage", {
    headers: {
      "User-Agent": "Claude-Monitor/0.1.0",
      Authorization: `Bearer ${accessToken}`,
    },
  });

  if (response.status === 401 || response.status === 403) {
    throw new AppError(INVALID_TOKEN);
  }
  if (response.status === 429) {
    throw new AppError(RATE_LIMITED);
  }
  if (response.status >= 500) {
    throw new AppError("OpenAI is experiencing issues. Please try again later.");
  }
  if (!response.ok) {
    throw new AppError(`Unexpected Codex error (HTTP ${response.status}). Please try again.`);
  }

  const usage = (await response.json()) as WhamUsageResponse;
  return {
    provider: "codex",
    windows: usage.rate_limit ? mapWindows(usage.rate_limit) : [],
    accountEmail: usage.email ?? null,
    planType: usage.plan_type ?? null,
  };
}

export function getCodexStatus(): ProviderStatus {
  try {
    loadAccessToken();
    return {
      provider: "codex",
      configured: true,
      source: "auth-json",
      message: null,
    };
  } catch {
    return {
      provider: "codex",
      configured: false,
      source: "auth-json",
      message: "Run `codex login` to enable Codex monitoring.",
    };
  }
}

export function loadAccessToken(): string {
  const authPath = getAuthPath();
  if (!existsSync(authPath)) {
    throw new AppError("Codex auth not found. Run `codex login` first.");
  }

  const auth = JSON.parse(readFileSync(authPath, "utf-8")) as CodexAuthFile;
  const token = auth.tokens?.access_token;
  if (!token) {
    throw new AppError("Codex auth is missing an access token. Run `codex login` again.");
  }

  return token;
}

export function getAuthPath(): string {
  const codexHome = process.env.CODEX_HOME;
  if (codexHome) {
    return join(codexHome, "auth.json");
  }

  return join(process.env.HOME ?? "", ".codex", "auth.json");
}

export function parseResetAt(value: string | number | null | undefined): string | null {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value === "string") {
    return value;
  }
  return new Date(value * 1000).toISOString();
}

export function labelForWindow(durationSeconds: number | null | undefined, fallbackKey: string) {
  if (durationSeconds === 18_000) {
    return "5 Hour";
  }
  if (durationSeconds === 604_800) {
    return "7 Day";
  }
  if (durationSeconds && durationSeconds > 0) {
    const hours = Math.floor(durationSeconds / 3600);
    if (hours >= 24) {
      return `${Math.floor(hours / 24)} Day`;
    }
    return `${hours} Hour`;
  }
  return fallbackKey === "primary" ? "Primary Window" : "Secondary Window";
}

function mapWindows(rateLimit: NonNullable<WhamUsageResponse["rate_limit"]>): UsageWindow[] {
  return [
    mapWindow("primary", rateLimit.primary_window),
    mapWindow("secondary", rateLimit.secondary_window),
  ].filter((window): window is UsageWindow => Boolean(window));
}

function mapWindow(
  key: string,
  window: WhamRateLimitWindow | null | undefined,
): UsageWindow | null {
  if (!window) {
    return null;
  }

  return {
    key,
    label: labelForWindow(window.limit_window_seconds, key),
    utilization: window.used_percent,
    resetsAt: parseResetAt(window.reset_at),
    windowDurationSeconds: window.limit_window_seconds ?? null,
  };
}
