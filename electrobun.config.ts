import type { ElectrobunConfig } from "electrobun";

export default {
  app: {
    name: "Claude Monitor",
    identifier: "dev.xikxp1.claude-monitor",
    version: "0.2.0",
  },
  runtime: {
    exitOnLastWindowClosed: false,
  },
  build: {
    bun: {
      entrypoint: "src/bun/index.ts",
    },
    watch: [
      "src",
      "native/macos/ClaudeMonitorPopover.m",
      "scripts",
    ],
    watchIgnore: ["web-build/**", ".svelte-kit/**"],
    copy: {
      "web-build": "views/mainview",
      "assets/icons/32x32.png": "views/assets/icon-template.png",
      "assets/icons/icon.icns": "views/assets/icon.icns",
      "assets/icons/icon.ico": "views/assets/icon.ico",
      "assets/icons/icon.png": "views/assets/icon.png",
      "native/macos/build/libClaudeMonitorPopover.dylib": "native/libClaudeMonitorPopover.dylib",
    },
    mac: {
      icons: "icon.iconset",
    },
    win: {
      icon: "assets/icons/icon.ico",
    },
    linux: {
      icon: "assets/icons/icon.png",
    },
  },
  release: {
    baseUrl: "https://github.com/xikxp1/claude-monitor/releases/latest/download",
  },
  scripts: {
    preBuild: "scripts/prebuild-electrobun.ts",
  },
} satisfies ElectrobunConfig;
