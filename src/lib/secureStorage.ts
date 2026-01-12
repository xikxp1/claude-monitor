import { Client, Stronghold } from "@tauri-apps/plugin-stronghold";
import { appDataDir } from "@tauri-apps/api/path";

const VAULT_NAME = "credentials";
const STRONGHOLD_PASSWORD = "claude-monitor-secure-storage";

// Singleton promise to prevent race conditions during initialization
let initPromise: Promise<{ stronghold: Stronghold; client: Client }> | null = null;

async function initialize(): Promise<{ stronghold: Stronghold; client: Client }> {
  const appDir = await appDataDir();
  const strongholdPath = `${appDir}/credentials.stronghold`;

  const stronghold = await Stronghold.load(strongholdPath, STRONGHOLD_PASSWORD);

  let client: Client;
  try {
    client = await stronghold.loadClient(VAULT_NAME);
  } catch {
    // Client doesn't exist, create it
    client = await stronghold.createClient(VAULT_NAME);
  }

  return { stronghold, client };
}

function getInitialized(): Promise<{ stronghold: Stronghold; client: Client }> {
  if (!initPromise) {
    initPromise = initialize();
  }
  return initPromise;
}

// Call this early to start initialization in the background
export function initSecureStorage(): void {
  getInitialized().catch((e) => {
    console.error("Failed to initialize secure storage:", e);
    initPromise = null; // Allow retry
  });
}

export interface Credentials {
  organizationId: string | null;
  sessionToken: string | null;
}

export async function saveCredentials(
  organizationId: string,
  sessionToken: string,
): Promise<void> {
  const { stronghold, client } = await getInitialized();
  const store = client.getStore();

  await store.insert("organization_id", Array.from(new TextEncoder().encode(organizationId)));
  await store.insert("session_token", Array.from(new TextEncoder().encode(sessionToken)));

  await stronghold.save();
}

export async function getCredentials(): Promise<Credentials> {
  try {
    const { client } = await getInitialized();
    const store = client.getStore();

    const orgIdBytes = await store.get("organization_id");
    const tokenBytes = await store.get("session_token");

    return {
      organizationId: orgIdBytes ? new TextDecoder().decode(new Uint8Array(orgIdBytes)) : null,
      sessionToken: tokenBytes ? new TextDecoder().decode(new Uint8Array(tokenBytes)) : null,
    };
  } catch {
    return {
      organizationId: null,
      sessionToken: null,
    };
  }
}

export async function deleteCredentials(): Promise<void> {
  try {
    const { stronghold, client } = await getInitialized();
    const store = client.getStore();

    try {
      await store.remove("organization_id");
    } catch {
      // Ignore if not found
    }
    try {
      await store.remove("session_token");
    } catch {
      // Ignore if not found
    }

    await stronghold.save();
  } catch {
    // Ignore errors during deletion
  }
}

// Reset for clear settings - will reinitialize on next access
export function resetSecureStorage(): void {
  initPromise = null;
}
