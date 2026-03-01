import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { spawn, spawnSync } from "node:child_process";

const mode = process.argv[2] ?? "info";

const rustLogByMode = {
  info: "codictate_app_lib=info",
  debug: "codictate_app_lib=debug,ort=warn,transcribe_rs=warn,hyper_util=warn,reqwest=warn",
  verbose: "trace",
};

const rustLog = rustLogByMode[mode];
if (!rustLog) {
  console.error(`Unsupported tauri dev mode: ${mode}`);
  process.exit(1);
}

loadDotEnv(".env");
warnAndOptionallyPruneTargetDir();

const command = process.platform === "win32" ? "tauri.cmd" : "tauri";
const child = spawn(command, ["dev"], {
  stdio: "inherit",
  shell: process.platform === "win32",
  env: {
    ...process.env,
    RUST_LOG: rustLog,
  },
});

child.on("exit", (code) => {
  process.exit(code ?? 0);
});

child.on("error", (error) => {
  console.error(`Failed to start tauri dev: ${error}`);
  process.exit(1);
});

function loadDotEnv(filename) {
  const envPath = resolve(process.cwd(), filename);
  if (!existsSync(envPath)) {
    return;
  }

  const lines = readFileSync(envPath, "utf8").split(/\r?\n/);
  for (const rawLine of lines) {
    const trimmed = rawLine.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }

    const line = trimmed.startsWith("export ")
      ? trimmed.slice("export ".length)
      : trimmed;
    const eqIndex = line.indexOf("=");
    if (eqIndex <= 0) {
      continue;
    }

    const key = line.slice(0, eqIndex).trim();
    let value = line.slice(eqIndex + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    process.env[key] = value;
  }
}

function warnAndOptionallyPruneTargetDir() {
  const targetDir = resolve(process.cwd(), "src-tauri", "target");
  if (!existsSync(targetDir)) {
    return;
  }

  const autoPruneEnabled = process.env.HANDY_AUTO_PRUNE_TARGET === "1";
  if (autoPruneEnabled) {
    runStalePrune();
  }

  const thresholdGiB = parseFloatOrDefault(
    process.env.HANDY_TARGET_WARN_GIB,
    20,
  );
  const bytes = tryReadDirectorySizeBytes(targetDir);
  if (bytes === null) {
    return;
  }

  const gib = bytes / (1024 * 1024 * 1024);
  if (gib < thresholdGiB) {
    return;
  }

  console.warn(
    `[tauri:dev] src-tauri/target is ${gib.toFixed(1)} GiB (threshold ${thresholdGiB.toFixed(1)} GiB).`,
  );
  if (autoPruneEnabled) {
    console.warn(
      "[tauri:dev] src-tauri/target is still large after auto-prune; run `bun run rust:clean` for full cleanup.",
    );
    return;
  }

  console.warn(
    "[tauri:dev] Run `bun run rust:prune` for stale artifact cleanup or `bun run rust:clean` for full cleanup.",
  );
}

function runStalePrune() {
  const pruneScript = resolve(process.cwd(), "scripts", "rust-prune.mjs");
  console.warn(
    "[tauri:dev] HANDY_AUTO_PRUNE_TARGET=1 detected; pruning stale Rust artifacts before launch.",
  );
  const result = spawnSync(process.execPath, [pruneScript, "--quiet"], {
    stdio: "inherit",
  });
  if (result.status !== 0 || result.error) {
    console.warn(
      `[tauri:dev] stale artifact prune exited with status ${result.status ?? "unknown"}${result.error ? ` (${result.error.message})` : ""}.`,
    );
  }
}

function tryReadDirectorySizeBytes(targetDir) {
  const result = spawnSync("du", ["-sk", targetDir], {
    encoding: "utf8",
  });
  if (result.error) {
    if (result.error.code === "ENOENT") {
      console.warn(
        "[tauri:dev] Skipping target size check: `du` is unavailable on this platform.",
      );
    }
    return null;
  }
  if (result.status !== 0 || !result.stdout) {
    return null;
  }

  const kibibytes = Number.parseInt(result.stdout.trim().split(/\s+/)[0], 10);
  if (!Number.isFinite(kibibytes) || kibibytes < 0) {
    return null;
  }

  return kibibytes * 1024;
}

function parseFloatOrDefault(rawValue, fallback) {
  const parsed = Number.parseFloat(rawValue ?? "");
  if (Number.isFinite(parsed) && parsed > 0) {
    return parsed;
  }
  return fallback;
}
