import { Client, Stronghold } from "@tauri-apps/plugin-stronghold";
import { appDataDir } from "@tauri-apps/api/path";

const VAULT_NAME = "credentials";
const STRONGHOLD_PASSWORD = "claude-monitor-secure-storage";

let stronghold: Stronghold | null = null;
let client: Client | null = null;

async function getStronghold(): Promise<Stronghold> {
  if (stronghold) return stronghold;

  const appDir = await appDataDir();
  const strongholdPath = `${appDir}/credentials.stronghold`;

  stronghold = await Stronghold.load(strongholdPath, STRONGHOLD_PASSWORD);
  return stronghold;
}

async function getClient(): Promise<Client> {
  if (client) return client;

  const sh = await getStronghold();

  try {
    client = await sh.loadClient(VAULT_NAME);
  } catch {
    // Client doesn't exist, create it
    client = await sh.createClient(VAULT_NAME);
  }

  return client;
}

export interface Credentials {
  organizationId: string | null;
  sessionToken: string | null;
}

export async function saveCredentials(
  organizationId: string,
  sessionToken: string,
): Promise<void> {
  const cl = await getClient();
  const store = cl.getStore();

  await store.insert("organization_id", Array.from(new TextEncoder().encode(organizationId)));
  await store.insert("session_token", Array.from(new TextEncoder().encode(sessionToken)));

  const sh = await getStronghold();
  await sh.save();
}

export async function getCredentials(): Promise<Credentials> {
  try {
    const cl = await getClient();
    const store = cl.getStore();

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
    const cl = await getClient();
    const store = cl.getStore();

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

    const sh = await getStronghold();
    await sh.save();
  } catch {
    // Ignore errors during deletion
  }
}
