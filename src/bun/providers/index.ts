import type { ProviderKind, ProviderStatus, UsageSnapshot } from "../../shared/types";
import { missingConfig } from "../errors";
import { fetchClaudeUsage, getClaudeStatus } from "./claude";
import { fetchCodexUsage, getCodexStatus } from "./codex";
import { fetchOllamaUsage, getOllamaStatus } from "./ollama";

export async function fetchUsageForProvider(
  provider: ProviderKind,
  orgId: string | null | undefined,
  sessionToken: string | null | undefined,
  ollamaSessionToken: string | null | undefined,
): Promise<UsageSnapshot> {
  switch (provider) {
    case "claude":
      return fetchClaudeUsage(orgId, sessionToken);
    case "codex":
      return fetchCodexUsage();
    case "ollama":
      if (!ollamaSessionToken) {
        throw missingConfig("ollama_session_token");
      }
      return fetchOllamaUsage(ollamaSessionToken);
  }
}

export function getProviderStatuses(
  claudeOrgId: string | null | undefined,
  claudeSessionToken: string | null | undefined,
  ollamaSessionToken: string | null | undefined,
): ProviderStatus[] {
  return [
    getClaudeStatus(claudeOrgId, claudeSessionToken),
    getCodexStatus(),
    getOllamaStatus(ollamaSessionToken),
  ];
}
