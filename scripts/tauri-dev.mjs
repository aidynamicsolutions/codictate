import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { spawn } from "node:child_process";

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
