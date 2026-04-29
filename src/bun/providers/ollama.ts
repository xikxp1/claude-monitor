import type { ProviderStatus, UsageSnapshot, UsageWindow } from "../../shared/types";
import { AppError, INVALID_TOKEN, RATE_LIMITED } from "../errors";
import { validateSessionToken } from "../validation";

const OLLAMA_COOKIE_NAME = "__Secure-session";

type OllamaSettingsData = {
  planType: string | null;
  sessionUsage: number | null;
  sessionResetsAt: string | null;
  weeklyUsage: number | null;
  weeklyResetsAt: string | null;
  accountEmail: string | null;
};

export async function fetchOllamaUsage(sessionToken: string): Promise<UsageSnapshot> {
  validateSessionToken(sessionToken);

  const response = await fetch("https://ollama.com/settings", {
    headers: {
      "User-Agent": "Claude-Monitor/0.1.0",
      Cookie: `${OLLAMA_COOKIE_NAME}=${sessionToken}`,
    },
  });

  if (response.status === 401 || response.status === 403) {
    throw new AppError(INVALID_TOKEN);
  }
  if (response.status === 429) {
    throw new AppError(RATE_LIMITED);
  }
  if (response.status >= 500) {
    throw new AppError("Ollama is experiencing issues. Please try again later.");
  }
  if (!response.ok) {
    throw new AppError(`Unexpected error (HTTP ${response.status}). Please try again.`);
  }

  const data = parseOllamaSettings(await response.text());
  return {
    provider: "ollama",
    windows: buildWindows(data),
    accountEmail: data.accountEmail,
    planType: data.planType,
  };
}

export function getOllamaStatus(sessionToken: string | null | undefined): ProviderStatus {
  const configured = Boolean(sessionToken);
  return {
    provider: "ollama",
    configured,
    source: "keychain",
    message: configured ? null : "Add your Ollama session cookie to enable monitoring.",
  };
}

export function parseOllamaSettings(html: string): OllamaSettingsData {
  return {
    planType: matchText(html, /<span[^>]*class="[^"]*\bcapitalize\b[^"]*"[^>]*>([^<]+)<\/span>/i),
    sessionUsage: parseUsageAfterLabel(html, "Session usage"),
    sessionResetsAt: parseResetAt(html, 0),
    weeklyUsage: parseUsageAfterLabel(html, "Weekly usage"),
    weeklyResetsAt: parseResetAt(html, 1),
    accountEmail: matchText(html, /<h2[^>]*id="header-email"[^>]*>([^<]+)<\/h2>/i),
  };
}

export function parsePercentage(text: string): number | null {
  const match = text.match(/(\d+(?:\.\d+)?)\s*%/);
  return match ? Number(match[1]) : null;
}

function parseUsageAfterLabel(html: string, label: string): number | null {
  const labelIndex = html.toLowerCase().indexOf(label.toLowerCase());
  if (labelIndex < 0) {
    return null;
  }

  const nextChunk = html.slice(labelIndex, labelIndex + 800);
  return parsePercentage(stripTags(nextChunk));
}

function parseResetAt(html: string, index: number): string | null {
  const matches = [...html.matchAll(/class="[^"]*\blocal-time\b[^"]*"[^>]*data-time="([^"]+)"/gi)];
  const value = matches[index]?.[1]?.trim();
  return value || null;
}

function matchText(html: string, regex: RegExp): string | null {
  const value = html.match(regex)?.[1]?.trim();
  return value ? decodeHtml(value) : null;
}

function stripTags(html: string): string {
  return html.replace(/<[^>]*>/g, " ");
}

function decodeHtml(value: string): string {
  return value
    .replaceAll("&amp;", "&")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'");
}

function buildWindows(data: OllamaSettingsData): UsageWindow[] {
  const windows: UsageWindow[] = [];
  if (data.sessionUsage !== null) {
    windows.push({
      key: "session",
      label: "Session",
      utilization: data.sessionUsage,
      resetsAt: data.sessionResetsAt,
      windowDurationSeconds: null,
    });
  }
  if (data.weeklyUsage !== null) {
    windows.push({
      key: "weekly",
      label: "Weekly",
      utilization: data.weeklyUsage,
      resetsAt: data.weeklyResetsAt,
      windowDurationSeconds: null,
    });
  }
  return windows;
}
