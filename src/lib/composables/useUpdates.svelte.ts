/**
 * Auto-update composable for checking and installing app updates
 */

import {
  checkForUpdates as checkForElectrobunUpdates,
  downloadAndInstallUpdate,
  restartApp as restartElectrobunApp,
  type Update,
} from "$lib/electrobunClient";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error"
  | "up-to-date";

export interface UpdateProgress {
  contentLength: number | null;
  downloaded: number;
}

export function useUpdates() {
  let status: UpdateStatus = $state("idle");
  let error: string | null = $state(null);
  let availableUpdate: Update | null = $state(null);
  let progress: UpdateProgress = $state({ contentLength: null, downloaded: 0 });
  let lastChecked: Date | null = $state(null);

  /**
   * Check for available updates
   */
  async function checkForUpdates(): Promise<boolean> {
    if (status === "checking" || status === "downloading") {
      return false;
    }

    status = "checking";
    error = null;

    try {
      const result = await checkForElectrobunUpdates();
      lastChecked = new Date();

      if (result.status === "error") {
        throw new Error(result.error);
      }

      if (result.data.updateAvailable || result.data.updateReady) {
        availableUpdate = result.data;
        status = "available";
        return true;
      } else {
        availableUpdate = null;
        status = "up-to-date";
        return false;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      status = "error";
      return false;
    }
  }

  /**
   * Download and install the available update
   */
  async function downloadAndInstall(): Promise<void> {
    if (!availableUpdate || status === "downloading") {
      return;
    }

    status = "downloading";
    error = null;
    progress = { contentLength: null, downloaded: 0 };

    try {
      const result = await downloadAndInstallUpdate();
      if (result.status === "error") {
        throw new Error(result.error);
      }
      availableUpdate = result.data;

      status = "ready";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      status = "error";
    }
  }

  /**
   * Restart the application to apply the update
   */
  async function restartApp(): Promise<void> {
    try {
      const result = await restartElectrobunApp();
      if (result.status === "error") {
        throw new Error(result.error);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      status = "error";
    }
  }

  /**
   * Get progress percentage (0-100)
   */
  function getProgressPercent(): number {
    if (!progress.contentLength || progress.contentLength === 0) {
      return 0;
    }
    return Math.round((progress.downloaded / progress.contentLength) * 100);
  }

  /**
   * Format bytes to human readable string
   */
  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  }

  /**
   * Reset update state
   */
  function reset(): void {
    status = "idle";
    error = null;
    availableUpdate = null;
    progress = { contentLength: null, downloaded: 0 };
  }

  return {
    // State (read-only getters)
    get status() {
      return status;
    },
    get error() {
      return error;
    },
    get availableUpdate() {
      return availableUpdate;
    },
    get progress() {
      return progress;
    },
    get lastChecked() {
      return lastChecked;
    },

    // Computed
    get isUpdateAvailable() {
      return status === "available" && availableUpdate !== null;
    },
    get isChecking() {
      return status === "checking";
    },
    get isDownloading() {
      return status === "downloading";
    },
    get isReady() {
      return status === "ready";
    },
    get progressPercent() {
      return getProgressPercent();
    },

    // Actions
    checkForUpdates,
    downloadAndInstall,
    restartApp,
    formatBytes,
    reset,
  };
}
