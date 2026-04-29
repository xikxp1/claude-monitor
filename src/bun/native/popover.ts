import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { dlopen, FFIType, ptr, suffix, type CString, type Pointer } from "bun:ffi";
import type { BrowserWindow } from "electrobun/bun";

export type PopoverController = {
  show: (trayPtr: Pointer) => boolean;
  hide: () => void;
  isShown: () => boolean;
  setTrayLength: (trayPtr: Pointer, length: number) => void;
  setTooltip: (trayPtr: Pointer, tooltip: string) => void;
};

export function createPopoverController(window: BrowserWindow): PopoverController {
  let nativePopover: NativePopover | null | undefined =
    process.platform === "darwin" ? undefined : null;
  let shown = false;

  return {
    show(trayPtr: Pointer) {
      if (nativePopover === undefined) {
        nativePopover = loadNativePopover();
      }
      if (nativePopover) {
        const didShow = nativePopover.show(window.ptr, trayPtr, 390, 304);
        if (didShow) {
          shown = true;
          return true;
        }
        console.error("Native NSPopover could not attach to the Electrobun window.");
      }

      return false;
    },
    hide() {
      nativePopover ??= null;
      if (nativePopover) {
        nativePopover.hide();
      }
      shown = false;
      window.setPosition(-10_000, -10_000);
    },
    isShown() {
      nativePopover ??= null;
      return nativePopover ? nativePopover.isShown() : shown;
    },
    setTrayLength(trayPtr: Pointer, length: number) {
      if (nativePopover === undefined) {
        nativePopover = loadNativePopover();
      }
      nativePopover?.setTrayLength(trayPtr, length);
    },
    setTooltip(trayPtr: Pointer, tooltip: string) {
      if (nativePopover === undefined) {
        nativePopover = loadNativePopover();
      }
      nativePopover?.setTooltip(trayPtr, tooltip);
    },
  };
}

type NativePopover = {
  show: (windowPtr: Pointer, trayPtr: Pointer, width: number, height: number) => boolean;
  hide: () => void;
  isShown: () => boolean;
  setTrayLength: (trayPtr: Pointer, length: number) => void;
  setTooltip: (trayPtr: Pointer, tooltip: string) => void;
};

function loadNativePopover(): NativePopover | null {
  const dylibPath = resolveNativePopoverPath();
  if (!dylibPath) {
    console.error("Native NSPopover bridge was not found.");
    return null;
  }

  const lib = dlopen(dylibPath, {
    cm_tray_set_length: {
      args: [FFIType.ptr, FFIType.f64],
      returns: FFIType.i32,
    },
    cm_tray_set_tooltip: {
      args: [FFIType.ptr, FFIType.cstring],
      returns: FFIType.i32,
    },
    cm_popover_show: {
      args: [FFIType.ptr, FFIType.ptr, FFIType.f64, FFIType.f64],
      returns: FFIType.i32,
    },
    cm_popover_hide: {
      args: [],
      returns: FFIType.void,
    },
    cm_popover_is_shown: {
      args: [],
      returns: FFIType.i32,
    },
  });

  return {
    show(windowPtr: Pointer, trayPtr: Pointer, width: number, height: number) {
      return lib.symbols.cm_popover_show(windowPtr, trayPtr, width, height) === 1;
    },
    hide() {
      lib.symbols.cm_popover_hide();
    },
    isShown() {
      return lib.symbols.cm_popover_is_shown() === 1;
    },
    setTrayLength(trayPtr: Pointer, length: number) {
      lib.symbols.cm_tray_set_length(trayPtr, length);
    },
    setTooltip(trayPtr: Pointer, tooltip: string) {
      lib.symbols.cm_tray_set_tooltip(trayPtr, toCString(tooltip));
    },
  };
}

function resolveNativePopoverPath(): string | null {
  const candidates = [
    join(process.cwd(), "..", "Resources", "app", "native", "libClaudeMonitorPopover.dylib"),
    resolve("native/macos/build/libClaudeMonitorPopover.dylib"),
    resolve(`native/macos/build/libClaudeMonitorPopover.${suffix}`),
  ];

  return candidates.find((candidate) => existsSync(candidate)) ?? null;
}

function toCString(value: string): CString {
  return ptr(new TextEncoder().encode(`${value}\0`)) as unknown as CString;
}
