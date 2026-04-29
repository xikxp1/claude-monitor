import { AppError, INVALID_TOKEN, missingConfig } from "./errors";

export function validateSessionToken(token: string): void {
  if (!token || token.length > 4096) {
    throw new AppError(INVALID_TOKEN);
  }

  for (const c of token) {
    if (!/[a-zA-Z0-9._+\-/=]/.test(c)) {
      throw new AppError(INVALID_TOKEN);
    }
  }
}

export function validateOrgId(orgId: string): void {
  if (!orgId) {
    throw missingConfig("organization_id");
  }

  if (orgId.length > 128) {
    throw missingConfig("organization_id too long");
  }

  for (const c of orgId) {
    if (!/[a-zA-Z0-9_-]/.test(c)) {
      throw missingConfig("invalid organization_id format");
    }
  }
}
