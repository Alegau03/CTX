#!/usr/bin/env node
const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const path = require("node:path");

const isWindows = process.platform === "win32";
const binaryName = isWindows ? "ctx.exe" : "ctx";
const binaryPath = path.join(__dirname, "..", "vendor", binaryName);

if (!existsSync(binaryPath)) {
  console.error("@alegau/ctx-bin: bundled CTX binary is missing. Try reinstalling the package.");
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
});

if (result.error) {
  console.error(`@alegau/ctx-bin: failed to launch CTX: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 0);
