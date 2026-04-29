import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const APP_ID = "dev.xikxp1.claude-monitor";
const APP_NAME = "Claude Monitor";

export function isAutostartEnabled(): boolean {
  if (process.platform === "darwin") {
    return existsSync(getMacLaunchAgentPath());
  }
  if (process.platform === "linux") {
    return existsSync(getLinuxDesktopPath());
  }
  if (process.platform === "win32") {
    const result = spawnSync("reg", [
      "query",
      "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
      "/v",
      APP_NAME,
    ]);
    return result.status === 0;
  }
  return false;
}

export function setAutostartEnabled(enabled: boolean): void {
  if (process.platform === "darwin") {
    setMacAutostart(enabled);
    return;
  }
  if (process.platform === "linux") {
    setLinuxAutostart(enabled);
    return;
  }
  if (process.platform === "win32") {
    setWindowsAutostart(enabled);
  }
}

function setMacAutostart(enabled: boolean): void {
  const path = getMacLaunchAgentPath();
  if (!enabled) {
    rmSync(path, { force: true });
    return;
  }

  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(
    path,
    `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${APP_ID}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${process.execPath}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
`,
  );
}

function setLinuxAutostart(enabled: boolean): void {
  const path = getLinuxDesktopPath();
  if (!enabled) {
    rmSync(path, { force: true });
    return;
  }

  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(
    path,
    `[Desktop Entry]
Type=Application
Name=${APP_NAME}
Exec=${process.execPath}
Terminal=false
X-GNOME-Autostart-enabled=true
`,
  );
}

function setWindowsAutostart(enabled: boolean): void {
  const key = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
  if (enabled) {
    spawnSync("reg", ["add", key, "/v", APP_NAME, "/t", "REG_SZ", "/d", process.execPath, "/f"]);
  } else {
    spawnSync("reg", ["delete", key, "/v", APP_NAME, "/f"]);
  }
}

function getMacLaunchAgentPath(): string {
  return join(homedir(), "Library", "LaunchAgents", `${APP_ID}.plist`);
}

function getLinuxDesktopPath(): string {
  return join(homedir(), ".config", "autostart", `${APP_ID}.desktop`);
}
