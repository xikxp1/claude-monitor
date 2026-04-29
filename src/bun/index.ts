import { BrowserView, BrowserWindow, Utils } from "electrobun/bun";
import type { AppRPC, UsageErrorEvent, UsageUpdateEvent } from "../shared/types";
import { AutoRefresh } from "./autoRefresh";
import { createCommandHandlers } from "./commands";
import { initDatabase } from "./history";
import { createPopoverController } from "./native/popover";
import { createInitialState } from "./state";
import { createTray } from "./tray";

Utils.setDockIconVisible(false);
initDatabase();

let mainWindow: BrowserWindow | null = null;
let commandHandlers: ReturnType<typeof createCommandHandlers> | null = null;

const rpc = BrowserView.defineRPC<AppRPC>({
  maxRequestTime: 30_000,
  handlers: {
    requests: {
      _: (method, params) => {
        if (!commandHandlers) {
          throw new Error("Command handlers are not ready");
        }
        const handler = commandHandlers[method as keyof typeof commandHandlers];
        if (typeof handler !== "function") {
          throw new Error(`Unknown command: ${String(method)}`);
        }
        return (handler as (params: unknown) => unknown)(params);
      },
    },
    messages: {},
  },
});

mainWindow = new BrowserWindow({
  title: "Claude Monitor",
  url: "views://mainview/index.html",
  frame: { x: -10_000, y: -10_000, width: 390, height: 304 },
  hidden: true,
  titleBarStyle: "hidden",
  transparent: true,
  styleMask: {
    Borderless: true,
    Titled: false,
    Resizable: false,
    NonactivatingPanel: true,
  },
  rpc,
});

const state = createInitialState();

const emitter = {
  usageUpdated(payload: UsageUpdateEvent) {
    (mainWindow?.webview.rpc as any)?.send.usageUpdated(payload);
  },
  usageError(payload: UsageErrorEvent) {
    (mainWindow?.webview.rpc as any)?.send.usageError(payload);
  },
};

const popover = createPopoverController(mainWindow);
const tray = createTray({
  window: mainWindow,
  popover,
  emitCheckForUpdates() {
    (mainWindow?.webview.rpc as any)?.send.checkForUpdates({});
  },
});
const autoRefresh = new AutoRefresh(state, emitter, tray);
commandHandlers = createCommandHandlers(state, autoRefresh);

mainWindow.on("blur", () => {
  if (process.platform !== "darwin") {
    mainWindow?.setPosition(-10_000, -10_000);
  }
});

autoRefresh.start();
