import { existsSync, readdirSync, rmSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

const DAY_MS = 24 * 60 * 60 * 1000;
const DEFAULT_KEEP_DAYS = 3;

const args = new Set(process.argv.slice(2));
const quiet = args.has("--quiet");

const keepDays = parseKeepDays(process.env.HANDY_PRUNE_DAYS);
const cutoffMs = Date.now() - keepDays * DAY_MS;
const targetRoot = resolve(process.cwd(), "src-tauri", "target");

if (!existsSync(targetRoot)) {
  if (!quiet) {
    console.log("[rust:prune] src-tauri/target does not exist. Nothing to prune.");
  }
  process.exit(0);
}

const candidates = [
  ...collectEntries(join(targetRoot, "debug", "incremental"), {
    includeName: () => true,
    includeDirectories: true,
  }),
  ...collectEntries(join(targetRoot, "release", "incremental"), {
    includeName: () => true,
    includeDirectories: true,
  }),
  ...collectEntries(join(targetRoot, "debug", "deps"), {
    includeName: isTempArchiveName,
    includeDirectories: true,
    includeFiles: true,
  }),
  ...collectEntries(join(targetRoot, "release", "deps"), {
    includeName: isTempArchiveName,
    includeDirectories: true,
    includeFiles: true,
  }),
];
const dedupedCandidates = [...new Set(candidates)];

let scanned = 0;
let removed = 0;
let failed = 0;

for (const candidate of dedupedCandidates) {
  scanned += 1;
  let stats;
  try {
    stats = statSync(candidate);
  } catch {
    continue;
  }

  if (stats.mtimeMs >= cutoffMs) {
    continue;
  }

  try {
    rmSync(candidate, { recursive: true, force: true });
    removed += 1;
    if (!quiet) {
      console.log(`[rust:prune] removed ${candidate}`);
    }
  } catch (error) {
    failed += 1;
    if (!quiet) {
      const message =
        error instanceof Error ? error.message : "unknown removal error";
      console.warn(`[rust:prune] failed to remove ${candidate}: ${message}`);
    }
  }
}

if (!quiet) {
  console.log(
    `[rust:prune] scanned=${scanned}, removed=${removed}, failed=${failed}, keep_days=${keepDays}`,
  );
}

process.exit(failed > 0 ? 1 : 0);

function parseKeepDays(raw) {
  const parsed = Number.parseInt(raw ?? "", 10);
  if (Number.isFinite(parsed) && parsed > 0) {
    return parsed;
  }
  return DEFAULT_KEEP_DAYS;
}

function collectEntries(
  root,
  {
    includeName,
    includeDirectories = false,
    includeFiles = false,
  },
) {
  if (!existsSync(root)) {
    return [];
  }

  const entries = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    if (!includeName(entry.name)) {
      continue;
    }
    if (entry.isDirectory() && !includeDirectories) {
      continue;
    }
    if (entry.isFile() && !includeFiles) {
      continue;
    }
    if (!entry.isDirectory() && !entry.isFile()) {
      continue;
    }
    entries.push(join(root, entry.name));
  }
  return entries;
}

function isTempArchiveName(name) {
  return name.startsWith(".tmp") && name.endsWith(".temp-archive");
}
