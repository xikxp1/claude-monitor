import type { ProviderStatus, UsageSnapshot, UsageWindow } from "../../shared/types";
import { AppError, INVALID_TOKEN, RATE_LIMITED, missingConfig } from "../errors";
import { validateOrgId, validateSessionToken } from "../validation";

type ClaudeUsageData = {
  five_hour?: ClaudeUsagePeriod | null;
  seven_day?: ClaudeUsagePeriod | null;
  seven_day_sonnet?: ClaudeUsagePeriod | null;
  seven_day_opus?: ClaudeUsagePeriod | null;
};

type ClaudeUsagePeriod = {
  utilization: number;
  resets_at?: string | null;
};

export async function fetchClaudeUsage(
  orgId: string | null | undefined,
  sessionToken: string | null | undefined,
): Promise<UsageSnapshot> {
  if (!orgId) {
    throw missingConfig("organization_id");
  }
  if (!sessionToken) {
    throw missingConfig("session_token");
  }

  validateOrgId(orgId);
  validateSessionToken(sessionToken);

  const response = await fetch(`https://claude.ai/api/organizations/${orgId}/usage`, {
    headers: {
      "User-Agent": "Claude-Monitor/0.1.0",
      Cookie: `sessionKey=${sessionToken}`,
    },
  });

  if (response.status === 401) {
    throw new AppError(INVALID_TOKEN);
  }
  if (response.status === 429) {
    throw new AppError(RATE_LIMITED);
  }
  if (response.status === 403) {
    throw new AppError("Access denied. Check your organization ID.");
  }
  if (response.status === 404) {
    throw new AppError("Organization not found. Check your organization ID.");
  }
  if (response.status >= 500) {
    throw new AppError("Claude is experiencing issues. Please try again later.");
  }
  if (!response.ok) {
    throw new AppError(`Unexpected error (HTTP ${response.status}). Please try again.`);
  }

  const usage = (await response.json()) as ClaudeUsageData;

  return {
    provider: "claude",
    windows: [
      mapWindow("five_hour", "5 Hour", usage.five_hour),
      mapWindow("seven_day", "7 Day", usage.seven_day),
      mapWindow("seven_day_sonnet", "Sonnet (7 Day)", usage.seven_day_sonnet),
      mapWindow("seven_day_opus", "Opus (7 Day)", usage.seven_day_opus),
    ].filter((window): window is UsageWindow => Boolean(window)),
    accountEmail: null,
    planType: null,
  };
}

export function getClaudeStatus(
  orgId: string | null | undefined,
  sessionToken: string | null | undefined,
): ProviderStatus {
  const configured = Boolean(orgId && sessionToken);
  return {
    provider: "claude",
    configured,
    source: "keychain",
    message: configured ? null : "Add your Claude organization ID and session token.",
  };
}

function mapWindow(
  key: string,
  label: string,
  period: ClaudeUsagePeriod | null | undefined,
): UsageWindow | null {
  if (!period) {
    return null;
  }

  return {
    key,
    label,
    utilization: period.utilization,
    resetsAt: period.resets_at ?? null,
    windowDurationSeconds: null,
  };
}
