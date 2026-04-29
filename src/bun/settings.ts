import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { Utils } from "electrobun/bun";

export type SettingsMap = Record<string, unknown>;

const settingsPath = join(Utils.paths.appData, "settings.json");

let cache: SettingsMap | null = null;

export function getSetting<T>(key: string): T | undefined {
  return loadSettings()[key] as T | undefined;
}

export function setSetting(key: string, value: unknown): void {
  const settings = loadSettings();
  settings[key] = value;
  saveSettings(settings);
}

export function clearSettings(): void {
  saveSettings({});
}

export function loadSettings(): SettingsMap {
  if (cache) {
    return cache;
  }

  try {
    cache = existsSync(settingsPath)
      ? (JSON.parse(readFileSync(settingsPath, "utf-8")) as SettingsMap)
      : {};
  } catch {
    cache = {};
  }
  return cache;
}

function saveSettings(settings: SettingsMap): void {
  mkdirSync(dirname(settingsPath), { recursive: true });
  cache = settings;
  writeFileSync(settingsPath, `${JSON.stringify(settings, null, 2)}\n`);
}
