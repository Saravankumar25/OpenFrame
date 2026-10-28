// Runs the WindowsHTML capture tour in parallel parts and merges them into WindowsHTML/.
//
//   node scripts/windows-html/parallel.mjs
//
// The sections of tests/e2e/capture/sections are split into parts that run side by side, each in
// its own data root and on its own driver ports (only the first part records the seed screens).
// The parts are merged in section order and renumbered, WindowsHTML/ is generated once, every
// page is checked again against its screenshot in shards, and WindowsHTML/README.md is written.
// Uses the last debug build (--no-build); build it first with `node tests/e2e/run.mjs --only=zz-capture`.

import { spawn, execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync, createWriteStream } from "node:fs";
import { join, resolve } from "node:path";
import { RAW, build } from "./build-windows-html.mjs";
import { writeReadme } from "./readme.mjs";

const REPO = resolve(import.meta.dirname, "..", "..");
const CACHE = join(REPO, ".build-cache");
const MODES = ["light", "dark"];
const PARTS = [
  ["10-shell", "95-app-home"],
  ["20-vault"],
  ["30-story"],
  ["40-screenplay"],
  ["50-breakdown", "85-ai-packages"],
  ["60-production-plan"],
  ["70-production-schedule"],
  ["80-pages"],
];
const SHARDS = 4;

function run(label, env, logFile) {
  return new Promise((done) => {
    const log = createWriteStream(logFile);
    const p = spawn(process.execPath, ["tests/e2e/run.mjs", "--only=zz-capture", "--no-build"], {
      cwd: REPO,
      env: { ...process.env, E2E_CAPTURE: "1", ...env },
      stdio: ["ignore", "pipe", "pipe"],
    });
    p.stdout.pipe(log);
    p.stderr.pipe(log);
    p.on("exit", (code) => {
      console.log(`${label}: ${code === 0 ? "passed" : `exit ${code}`} (${logFile})`);
      done(code ?? 1);
    });
  });
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// 1. Parts, started a few seconds apart.
const partDirs = PARTS.map((_, i) => join(CACHE, `wh-part-${i + 1}`));
const codes = await Promise.all(
  PARTS.map(async (sections, i) => {
    await sleep(i * 15_000);
    return run(
      `part ${i + 1} (${sections.join(", ")})`,
      {
        E2E_CAPTURE_ONLY: sections.join(","),
        E2E_PORT_BASE: String(4600 + i * 10),
        E2E_CAPTURE_OUT: partDirs[i],
        E2E_CAPTURE_NO_SEED_SHOTS: i === 0 ? "0" : "1",
      },
      join(CACHE, `wh-part-${i + 1}.log`),
    );
  }),
);

// 2. Merge in section order, renumbered.
rmSync(RAW, { recursive: true, force: true });
for (const m of MODES) mkdirSync(join(RAW, m), { recursive: true });
const merged = { screens: [], links: [], unreachable: [], failed: [] };
for (const dir of partDirs) {
  if (!existsSync(join(dir, "manifest.json"))) {
    merged.failed.push({ name: dir, section: "", error: "the part wrote no manifest" });
    continue;
  }
  const m = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));
  const ids = new Map();
  for (const s of m.screens) {
    const id = `${String(merged.screens.length + 1).padStart(3, "0")}-${s.name}`;
    ids.set(s.id, id);
    for (const mode of MODES) for (const ext of ["png", "json"]) copyFileSync(join(dir, mode, `${s.id}.${ext}`), join(RAW, mode, `${id}.${ext}`));
    merged.screens.push({ ...s, id });
  }
  for (const l of m.links) if (ids.has(l.from) && ids.has(l.to)) merged.links.push({ ...l, from: ids.get(l.from), to: ids.get(l.to) });
  merged.unreachable.push(...m.unreachable);
  merged.failed.push(...m.failed);
}
writeFileSync(join(RAW, "manifest.json"), JSON.stringify(merged, null, 1));
const commit = execFileSync("git", ["rev-parse", "--short", "HEAD"], { cwd: REPO, encoding: "utf8" }).trim();
build({ commit });
console.log(`merged: ${merged.screens.length} screens, ${merged.failed.length} failed, ${merged.unreachable.length} unreachable`);

// 3. Check the merged pages in shards.
const shardCodes = await Promise.all(
  Array.from({ length: SHARDS }, async (_, i) => {
    await sleep(i * 10_000);
    return run(`check ${i + 1} of ${SHARDS}`, { E2E_CAPTURE_VERIFY_ONLY: "1", E2E_VERIFY_SHARD: `${i + 1}/${SHARDS}`, E2E_PORT_BASE: String(4700 + i * 10) }, join(CACHE, `wh-check-${i + 1}.log`));
  }),
);
const results = [];
for (let i = 1; i <= SHARDS; i++) {
  const f = join(RAW, `verify-${i}-of-${SHARDS}.json`);
  if (existsSync(f)) results.push(...JSON.parse(readFileSync(f, "utf8")));
}
results.sort((a, b) => a.id.localeCompare(b.id) || a.mode.localeCompare(b.mode));
writeFileSync(join(RAW, "verify.json"), JSON.stringify(results, null, 1));
writeReadme({ manifest: merged, results, commit, threshold: 24, maxRatio: 0.005, width: 1600, height: 1000 });

const bad = results.filter((r) => !r.ok);
const missing = merged.screens.length * MODES.length - results.length;
console.log(`WindowsHTML: ${merged.screens.length} screens, ${results.length - bad.length} of ${results.length} pages pass, ${bad.length} over the limit, ${missing} not checked; parts ${codes.join(",")}, checks ${shardCodes.join(",")}`);
process.exit(bad.length || missing || merged.failed.length || merged.screens.length < 400 ? 1 : 0);
