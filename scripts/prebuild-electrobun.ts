import { spawnSync } from "node:child_process";

run("node", ["node_modules/vite/bin/vite.js", "build"]);
run("bun", ["scripts/fix-electrobun-assets.ts"]);
run("bun", ["scripts/build-macos-popover.ts"]);

function run(command: string, args: string[]): void {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
