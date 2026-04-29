import { Entry } from "@napi-rs/keyring";
import { AppError } from "./errors";

const SERVICE_NAME = "dev.xikxp1.claude-monitor";
const CREDENTIALS_KEY = "credentials";
const OLLAMA_CREDENTIALS_KEY = "ollama_credentials";

type StoredCredentials = {
  organization_id: string;
  session_token: string;
};

export function loadCredentials(): [string, string] | null {
  try {
    const raw = getPassword(CREDENTIALS_KEY);
    if (!raw) {
      return null;
    }
    const creds = JSON.parse(raw) as StoredCredentials;
    return [creds.organization_id, creds.session_token];
  } catch {
    return null;
  }
}

export function saveCredentials(orgId: string, sessionToken: string): void {
  try {
    setPassword(CREDENTIALS_KEY, JSON.stringify({ organization_id: orgId, session_token: sessionToken }));
  } catch (error) {
    throw new AppError(`Storage error: Failed to store credentials: ${String(error)}`);
  }
}

export function deleteCredentials(): void {
  deletePassword(CREDENTIALS_KEY);
}

export function loadOllamaCredentials(): string | null {
  try {
    return getPassword(OLLAMA_CREDENTIALS_KEY);
  } catch {
    return null;
  }
}

export function saveOllamaCredentials(sessionToken: string): void {
  try {
    setPassword(OLLAMA_CREDENTIALS_KEY, sessionToken);
  } catch (error) {
    throw new AppError(`Storage error: Failed to store Ollama credentials: ${String(error)}`);
  }
}

export function deleteOllamaCredentials(): void {
  deletePassword(OLLAMA_CREDENTIALS_KEY);
}

function getPassword(account: string): string | null {
  return new Entry(SERVICE_NAME, account).getPassword();
}

function setPassword(account: string, password: string): void {
  new Entry(SERVICE_NAME, account).setPassword(password);
}

function deletePassword(account: string): void {
  try {
    new Entry(SERVICE_NAME, account).deletePassword();
  } catch {
    // Clearing credentials should be idempotent from the app's perspective.
  }
}
