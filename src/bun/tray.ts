import { Screen, Tray, Utils, type BrowserWindow } from "electrobun/bun";
import type { UsageSnapshot } from "../shared/types";
import type { PopoverController } from "./native/popover";

export type TrayController = {
  updateTooltip: (usage?: UsageSnapshot | null) => void;
};

type TrayOptions = {
  window: BrowserWindow;
  popover: PopoverController;
  emitCheckForUpdates: () => void;
};

export function createTray({ window, popover, emitCheckForUpdates }: TrayOptions): TrayController {
  if (process.platform === "darwin") {
    const tray = new Tray({
      image: "views://assets/icon-template.png",
      template: true,
      width: 18,
      height: 18,
    });

    if (tray.ptr) {
      popover.setTrayLength(tray.ptr, 18);
      popover.setTooltip(tray.ptr, "Claude Monitor");
    }

    tray.on("tray-clicked", () => {
      if (!tray.ptr) {
        return;
      }
      if (popover.isShown()) {
        popover.hide();
        return;
      }
      popover.show(tray.ptr);
    });

    return {
      updateTooltip(usage?: UsageSnapshot | null) {
        if (tray.ptr) {
          popover.setTooltip(tray.ptr, buildTooltip(usage));
        }
      },
    };
  }

  const tray = new Tray({
    title: "Claude Monitor",
    image: "views://assets/icon-template.png",
    template: true,
  });

  tray.setMenu([
    { type: "normal", label: "Claude Monitor", action: "app_info", enabled: false },
    { type: "normal", label: "Check for Updates", action: "check_updates" },
    { type: "separator" },
    { type: "normal", label: "Quit", action: "quit" },
  ]);

  tray.on("tray-clicked", (event) => {
    const action = (event as { data?: { action?: string } }).data?.action ?? "";
    if (action === "check_updates") {
      emitCheckForUpdates();
      showWindowNearTray(window, tray);
      return;
    }
    if (action === "quit") {
      Utils.quit();
      return;
    }
    if (action === "" || action === "app_info") {
      showWindowNearTray(window, tray);
    }
  });

  return {
    updateTooltip(usage?: UsageSnapshot | null) {
      tray.setTitle(buildTooltip(usage));
    },
  };
}

function showWindowNearTray(window: BrowserWindow, tray: Tray): void {
  const frame = getTrayPopoverFrame(tray);

  window.setFrame(frame.x, frame.y, frame.width, frame.height);
  window.setAlwaysOnTop(true);
  window.show();
  window.focus();
}

function getTrayPopoverFrame(tray: Tray): { x: number; y: number; width: number; height: number } {
  const bounds = tray.getBounds();
  const display = displayForPoint(bounds.x, bounds.y);
  const width = 380;
  const height = 304;
  const x = Math.min(
    Math.max(display.workArea.x, Math.round(bounds.x + bounds.width / 2 - width / 2)),
    display.workArea.x + display.workArea.width - width,
  );
  const y =
    bounds.y < display.workArea.y + display.workArea.height / 2
      ? bounds.y + bounds.height + 8
      : bounds.y - height - 8;

  return { x, y, width, height };
}

function displayForPoint(x: number, y: number) {
  return (
    Screen.getAllDisplays().find((display) => {
      const { bounds } = display;
      return x >= bounds.x && x < bounds.x + bounds.width && y >= bounds.y && y < bounds.y + bounds.height;
    }) ?? Screen.getPrimaryDisplay()
  );
}

function buildTooltip(usage?: UsageSnapshot | null): string {
  if (!usage || usage.windows.length === 0) {
    return "Claude Monitor";
  }

  const providerName =
    usage.provider === "claude"
      ? "Claude Monitor"
      : usage.provider === "codex"
        ? "Codex Monitor"
        : "Ollama Monitor";
  const parts = usage.windows.map((window) => `${window.label}: ${window.utilization.toFixed(0)}%`);
  return `${providerName}\n${parts.join(" | ")}`;
}
