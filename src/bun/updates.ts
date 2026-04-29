import { Updater } from "electrobun/bun";
import type { CommandResult, UpdateInfo } from "../shared/types";
import { normalizeError } from "./errors";

export async function checkForUpdates(): Promise<CommandResult<UpdateInfo, string>> {
  try {
    return { status: "ok", data: await Updater.checkForUpdate() };
  } catch (error) {
    return { status: "error", error: normalizeError(error) };
  }
}

export async function downloadAndInstallUpdate(): Promise<CommandResult<UpdateInfo, string>> {
  try {
    await Updater.downloadUpdate();
    return { status: "ok", data: Updater.updateInfo() };
  } catch (error) {
    return { status: "error", error: normalizeError(error) };
  }
}

export async function restartApp(): Promise<CommandResult<null, string>> {
  try {
    await Updater.applyUpdate();
    return { status: "ok", data: null };
  } catch (error) {
    return { status: "error", error: normalizeError(error) };
  }
}
