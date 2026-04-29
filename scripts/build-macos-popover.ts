import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";

if (process.platform !== "darwin") {
  process.exit(0);
}

mkdirSync("native/macos/build", { recursive: true });

const result = spawnSync(
  "clang",
  [
    "-dynamiclib",
    "-fobjc-arc",
    "-framework",
    "Cocoa",
    "native/macos/ClaudeMonitorPopover.m",
    "-o",
    "native/macos/build/libClaudeMonitorPopover.dylib",
  ],
  { stdio: "inherit" },
);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

