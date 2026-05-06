#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const https = require("node:https");
const { spawnSync } = require("node:child_process");

const pkg = require("./package.json");
const repoSlug = process.env.CTX_REPO_SLUG || "Alegau03/CTX";
const version = process.env.CTX_VERSION || pkg.version;
const installRoot = __dirname;
const vendorDir = path.join(installRoot, "vendor");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "alegau-ctx-bin-"));

function fail(message) {
  console.error(`@alegau/ctx-bin install error: ${message}`);
  process.exit(1);
}

function targetFor(platform, arch) {
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
  if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";
  return null;
}

function assetName(versionValue, target) {
  if (target.includes("windows")) return `ctx-${versionValue}-${target}.zip`;
  return `ctx-${versionValue}-${target}.tar.gz`;
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destination);
    https.get(url, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        file.close();
        return resolve(download(response.headers.location, destination));
      }
      if (response.statusCode !== 200) {
        file.close();
        return reject(new Error(`unexpected status ${response.statusCode} for ${url}`));
      }
      response.pipe(file);
      file.on("finish", () => {
        file.close(resolve);
      });
    }).on("error", (error) => {
      file.close();
      reject(error);
    });
  });
}

function verifyChecksum(archivePath, sumsPath) {
  const archiveName = path.basename(archivePath);
  const sums = fs.readFileSync(sumsPath, "utf8");
  const line = sums.split(/\r?\n/).find((entry) => entry.includes(archiveName));
  if (!line) fail(`missing checksum entry for ${archiveName}`);
  const expected = line.trim().split(/\s+/)[0];
  const actual = spawnSync("shasum", ["-a", "256", archivePath], { encoding: "utf8" });
  if (actual.status !== 0) fail("failed to compute checksum with shasum");
  const actualHash = actual.stdout.trim().split(/\s+/)[0];
  if (actualHash !== expected) fail("checksum verification failed");
}

function extractArchive(archivePath, target) {
  if (target.includes("windows")) {
    const result = spawnSync("powershell", [
      "-NoProfile",
      "-Command",
      `Expand-Archive -LiteralPath '${archivePath}' -DestinationPath '${tmpDir}/extract' -Force`,
    ], { stdio: "inherit" });
    if (result.status !== 0) fail("failed to extract zip archive");
    return;
  }

  const result = spawnSync("tar", ["-xzf", archivePath, "-C", path.join(tmpDir, "extract")], {
    stdio: "inherit",
  });
  if (result.status !== 0) fail("failed to extract tar archive");
}

async function main() {
  const target = targetFor(process.platform, process.arch);
  if (!target) fail(`unsupported platform ${process.platform}/${process.arch}`);

  fs.mkdirSync(path.join(tmpDir, "extract"), { recursive: true });

  const archive = assetName(version, target);
  const baseUrl = process.env.CTX_RELEASE_BASE_URL || `https://github.com/${repoSlug}/releases/download/v${version}`;
  const archivePath = path.join(tmpDir, archive);
  const sumsPath = path.join(tmpDir, "SHA256SUMS");

  console.log(`@alegau/ctx-bin: downloading ${archive}`);
  await download(`${baseUrl}/${archive}`, archivePath);
  await download(`${baseUrl}/SHA256SUMS`, sumsPath);
  verifyChecksum(archivePath, sumsPath);
  extractArchive(archivePath, target);

  const binaryName = process.platform === "win32" ? "ctx.exe" : "ctx";
  const extractedBinary = findBinary(path.join(tmpDir, "extract"), binaryName);
  if (!extractedBinary) fail(`could not find ${binaryName} in extracted archive`);

  fs.mkdirSync(vendorDir, { recursive: true });
  const targetBinary = path.join(vendorDir, binaryName);
  fs.copyFileSync(extractedBinary, targetBinary);
  if (process.platform !== "win32") fs.chmodSync(targetBinary, 0o755);

  console.log(`@alegau/ctx-bin: installed ${binaryName}`);
}

function findBinary(root, binaryName) {
  const entries = fs.readdirSync(root, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      const nested = findBinary(fullPath, binaryName);
      if (nested) return nested;
    } else if (entry.isFile() && entry.name === binaryName) {
      return fullPath;
    }
  }
  return null;
}

main().catch((error) => fail(error.message));
