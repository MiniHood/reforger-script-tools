#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const release = process.argv.includes("--release");
const cargo = resolveCargoCommand();
const platformArch = `${process.platform}-${process.arch}`;
const executableName = process.platform === "win32" ? "reforger_language_server.exe" : "reforger_language_server";
const profile = release ? "release" : "debug";
const buildTargetDir = resolve(repoRoot, "server", "target", "build-language-server");
const sourceBinary = resolve(buildTargetDir, profile, executableName);
const devBinary = resolve(repoRoot, "server", "target", profile, executableName);
const targetFolder = resolve(repoRoot, "dist", "server", platformArch);
const targetBinary = resolve(targetFolder, executableName);

const cargoArgs = [
  "build",
  ...(release ? ["--release"] : []),
  "--manifest-path",
  "server/Cargo.toml",
  "--bin",
  "reforger_language_server",
];

if (!release) {
  stopRepoLanguageServers("before build");
}

const result = spawnSync(cargo, cargoArgs, {
  cwd: repoRoot,
  stdio: "inherit",
  env: {
    ...process.env,
    CARGO_TARGET_DIR: buildTargetDir,
  },
});

if (result.error) {
  console.error(`Failed to build language server: ${result.error.message}`);
  process.exit(1);
}

if ((result.status ?? 1) !== 0) {
  process.exit(result.status ?? 1);
}

if (!release) {
  mkdirSync(dirname(devBinary), { recursive: true });
  copyBinaryWithStoppedServer(sourceBinary, devBinary, "development binary");
  if (process.platform !== "win32") {
    chmodSync(devBinary, 0o755);
  }
}

mkdirSync(targetFolder, { recursive: true });
if (release) {
  copyFileSync(sourceBinary, targetBinary);
} else {
  copyBinaryWithStoppedServer(sourceBinary, targetBinary, "packaged binary");
}
if (process.platform !== "win32") {
  chmodSync(targetBinary, 0o755);
}

if (!release) {
  console.log(`Copied language server binary: ${devBinary}`);
}
console.log(`Copied language server binary: ${targetBinary}`);

function resolveCargoCommand() {
  if (process.platform === "win32" && process.env.USERPROFILE) {
    const userCargo = resolve(process.env.USERPROFILE, ".cargo", "bin", "cargo.exe");
    if (existsSync(userCargo)) {
      return userCargo;
    }
  }

  return "cargo";
}

function copyBinaryWithStoppedServer(source, destination, label) {
  const attempts = process.platform === "win32" ? 12 : 3;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    stopRepoLanguageServers(`before replacing ${label}`);
    sleepSync(150);
    try {
      copyFileSync(source, destination);
      return;
    } catch (error) {
      const retryable = error && (error.code === "EBUSY" || error.code === "EPERM" || error.code === "EACCES");
      if (!retryable || attempt === attempts) {
        throw error;
      }
      console.warn(`Retrying ${label} replacement after locked file (${attempt}/${attempts}).`);
      sleepSync(250);
    }
  }
}

function stopRepoLanguageServers(phase) {
  if (process.platform === "win32") {
    stopRepoLanguageServersWindows(phase);
    return;
  }

  stopRepoLanguageServersPosix(phase);
}

function stopRepoLanguageServersWindows(phase) {
  const script = `
$repo = ${powerShellString(repoRoot)}
$procs = Get-CimInstance Win32_Process -Filter "Name = '${executableName}'" | Where-Object {
  $_.ExecutablePath -and $_.ExecutablePath.StartsWith($repo, [System.StringComparison]::OrdinalIgnoreCase)
}
foreach ($proc in $procs) {
  Write-Output ("Stopping language server during ${phase}: PID {0} {1}" -f $proc.ProcessId, $proc.ExecutablePath)
  Stop-Process -Id $proc.ProcessId -Force
}
`;
  const result = spawnSync("powershell.exe", ["-NoProfile", "-Command", script], {
    cwd: repoRoot,
    encoding: "utf8",
  });

  if (result.stdout.trim()) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr.trim()) {
    process.stderr.write(result.stderr);
  }
  if ((result.status ?? 0) !== 0) {
    console.warn(`Could not stop existing language server during ${phase}; continuing build.`);
  }
}

function stopRepoLanguageServersPosix(phase) {
  const result = spawnSync("pgrep", ["-f", executableName], {
    encoding: "utf8",
  });
  if ((result.status ?? 1) !== 0 || !result.stdout.trim()) {
    return;
  }

  for (const line of result.stdout.split(/\r?\n/)) {
    const pid = Number(line.trim());
    if (!Number.isInteger(pid) || pid <= 0 || pid === process.pid) {
      continue;
    }

    const command = spawnSync("ps", ["-p", String(pid), "-o", "command="], {
      encoding: "utf8",
    }).stdout;
    if (!command.includes(repoRoot)) {
      continue;
    }

    try {
      process.kill(pid, "SIGKILL");
      console.log(`Stopping language server during ${phase}: PID ${pid}`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.warn(`Could not stop language server PID ${pid} during ${phase}: ${message}`);
    }
  }
}

function powerShellString(value) {
  return `'${value.replace(/'/g, "''")}'`;
}

function sleepSync(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}
