// Reads the Process Monitor CSV from the clean-machine run and writes the summary the release
// evidence keeps: where the OpenFrame processes talked to the network, which files and registry
// keys they wrote, and everything outside the places the app is allowed to use.
//   node procmon-report.mjs [session.csv] [--out file.md]      --self-test runs it on a tiny CSV

import { createReadStream, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";

const REPO = join(import.meta.dirname, "..", "..", "..");
const EVIDENCE = join(REPO, "docs", "release", "evidence", "w9", "vm");

const OPENFRAME = /^(openframe-desktop|llama-server|llama-[\w-]+)\.exe$/i;
const WEBVIEW = /^msedgewebview2\.exe$/i;
const HF_HOST = /(^|\.)(huggingface\.co|hf\.co)$|^cdn-lfs/i;
const LOOPBACK = /^(localhost|127\.\d+\.\d+\.\d+|::1|\[::1\]|0\.0\.0\.0)$/i;

const ALLOWED_FILES = [
  [/^%USERPROFILE%\\AppData\\Local\\OpenFrame\\/i, "app data"],
  [/^%USERPROFILE%\\AppData\\Local\\Programs\\OpenFrame Studio\\/i, "install folder"],
  [/^%USERPROFILE%\\AppData\\Local\\studio\.openframe\.desktop\\/i, "WebView2 data"],
  [/^%USERPROFILE%\\Documents\\OpenFrame\\/i, "projects"],
  [/^%USERPROFILE%\\Documents\\w9out\\/i, "test output folder"],
  [/^%USERPROFILE%\\AppData\\Local\\Temp\\/i, "temp"],
];
const ALLOWED_REG = /OpenFrame|studio\.openframe|WebView2|EdgeWebView|Edge\\(BLBeacon|PreferenceMACs)|Microsoft\\Windows\\CurrentVersion\\(Explorer|Internet Settings)/i;

/** One CSV line with quoted fields ("" is an escaped quote). Procmon writes one record per line. */
export function parseCsvLine(line) {
  const out = [];
  let cur = "";
  let q = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (q) {
      if (c === '"' && line[i + 1] === '"') {
        cur += '"';
        i++;
      } else if (c === '"') q = false;
      else cur += c;
    } else if (c === '"') q = true;
    else if (c === ",") {
      out.push(cur);
      cur = "";
    } else cur += c;
  }
  out.push(cur);
  return out;
}

const norm = (p) => p.replace(/^[A-Z]:\\Users\\[^\\]+\\/i, "%USERPROFILE%\\");
const topFolder = (p) => p.split("\\").slice(0, 5).join("\\");
const bump = (m, k) => m.set(k, (m.get(k) ?? 0) + 1);

/** Splits "local:port -> remote:port" and returns the remote host and port. */
function remote(path) {
  const right = path.includes(" -> ") ? path.split(" -> ")[1] : path;
  const m = /^(.*):([^:]+)$/.exec(right.trim());
  return m ? { host: m[1].replace(/^\[|\]$/g, ""), port: m[2] } : { host: right.trim(), port: "" };
}
const localName = (path) => path.split(" -> ")[0].replace(/:[^:]+$/, "").toLowerCase();

export async function summarize(lines) {
  const procs = new Map(); // process name -> { net, files, reg }
  let header = null;
  let rows = 0;
  for await (const line of lines) {
    if (!line.trim()) continue;
    const f = parseCsvLine(line);
    if (!header) {
      header = f.map((h) => h.replace(/^\uFEFF/, "").trim());
      continue;
    }
    rows++;
    const [, name, , op, path, result, detail] = f.length >= 7 ? f : [...f, "", "", "", "", "", ""];
    const own = OPENFRAME.test(name);
    if (!own && !WEBVIEW.test(name)) continue;
    const p = procs.get(name) ?? { own, net: new Map(), files: new Map(), reg: new Map() };
    procs.set(name, p);
    if (/^(TCP|UDP) (Connect|Send)$/.test(op)) {
      const r = remote(path);
      const loop = LOOPBACK.test(r.host) || r.host.toLowerCase() === localName(path);
      bump(p.net, `${r.host}:${r.port}${loop ? "|loopback" : HF_HOST.test(r.host) ? "|huggingface" : "|OTHER"}`);
    } else if (result === "SUCCESS" && (/^(WriteFile|SetRenameInformationFile|SetDispositionInformationFile)$/.test(op) || (op === "CreateFile" && /Desired Access:[^,]*(Generic Write|Write Data|Append)/i.test(detail ?? "")))) {
      bump(p.files, norm(path));
    } else if (result === "SUCCESS" && /^(RegSetValue|RegCreateKey|RegDeleteKey|RegDeleteValue)$/.test(op)) {
      bump(p.reg, path);
    }
  }
  return { procs, rows, headerSeen: !!header };
}

export function render(s, defender = "") {
  const out = [`# Clean-machine network, file and registry activity`, "", `Source: Process Monitor session from the clean Windows 11 guest (${s.rows} events read). OpenFrame processes are exact; \`msedgewebview2.exe\` cannot be tied to the app by name alone (it is the shared WebView2 runtime) and is listed separately.`, ""];
  const flagged = [];
  for (const [name, p] of [...s.procs].sort((a, b) => Number(b[1].own) - Number(a[1].own) || a[0].localeCompare(b[0]))) {
    out.push(`## ${name}${p.own ? "" : " (not attributable to the app by name)"}`, "");
    out.push("### Network (TCP/UDP connect and send)", "");
    if (!p.net.size) out.push("None.", "");
    else {
      out.push("| Remote | Class | Count |", "|---|---|---|");
      for (const [k, n] of [...p.net].sort((a, b) => b[1] - a[1])) {
        const [dest, cls] = k.split("|");
        out.push(`| ${dest} | ${cls} | ${n} |`);
        if (cls === "OTHER") flagged.push(`${name}: network to ${dest} (${n}x)`);
      }
      out.push("");
    }
    const groups = new Map();
    const stray = [];
    for (const [path, n] of p.files) {
      const allowed = ALLOWED_FILES.find(([re]) => re.test(path));
      const top = allowed ? allowed[1] : topFolder(path);
      groups.set(top, (groups.get(top) ?? 0) + n);
      if (!allowed) stray.push([path, n]);
    }
    out.push("### Files written, by top folder", "");
    if (!groups.size) out.push("None.", "");
    else {
      out.push("| Folder | Writes | Allowed |", "|---|---|---|");
      for (const [g, n] of [...groups].sort((a, b) => b[1] - a[1])) out.push(`| ${g} | ${n} | ${ALLOWED_FILES.some(([, l]) => l === g) ? "yes" : "FLAGGED"} |`);
      out.push("");
    }
    for (const [path, n] of stray.slice(0, 200)) flagged.push(`${name}: file written outside the allowed folders: ${path} (${n}x)`);
    out.push("### Registry keys written", "");
    if (!p.reg.size) out.push("None.", "");
    else {
      out.push("| Key | Count | Expected |", "|---|---|---|");
      for (const [k, n] of [...p.reg].sort((a, b) => b[1] - a[1]).slice(0, 150)) {
        const ok = ALLOWED_REG.test(k);
        out.push(`| ${k} | ${n} | ${ok ? "yes" : "FLAGGED"} |`);
        if (!ok) flagged.push(`${name}: registry write ${k} (${n}x)`);
      }
      out.push("");
    }
  }
  out.push("## Flagged (outside the allowed places)", "");
  out.push(flagged.length ? flagged.map((f) => `- ${f}`).join("\n") : "Nothing flagged.", "");
  out.push("Allowed: `%LOCALAPPDATA%\\OpenFrame`, `%LOCALAPPDATA%\\Programs\\OpenFrame Studio`, `%LOCALAPPDATA%\\studio.openframe.desktop`, `Documents\\OpenFrame`, the w9out folder, Temp; network only to loopback and Hugging Face (huggingface.co, *.hf.co, cdn-lfs*).", "");
  if (defender) out.push("## Microsoft Defender scans of the installers", "", defender, "");
  return { text: out.join("\n"), flagged };
}

function defenderSection() {
  if (!existsSync(EVIDENCE)) return "";
  return readdirSync(EVIDENCE)
    .filter((f) => /^defender-.*\.txt$/.test(f))
    .map((f) => {
      const t = readFileSync(join(EVIDENCE, f), "utf8").trim().split(/\r?\n/).filter(Boolean).slice(-4).join("\n");
      return `**${f}**\n\n\`\`\`\n${t}\n\`\`\``;
    })
    .join("\n\n");
}

async function selfTest() {
  const csv = [
    '"Time of Day","Process Name","PID","Operation","Path","Result","Detail"',
    '"1:00:00 PM","openframe-desktop.exe","10","TCP Connect","vm.local:50000 -> localhost:4445","SUCCESS",""',
    '"1:00:01 PM","llama-server.exe","11","TCP Send","vm.local:50001 -> cdn-lfs.hf.co:https","SUCCESS",""',
    '"1:00:02 PM","openframe-desktop.exe","10","TCP Connect","vm.local:50002 -> evil.example.com:443","SUCCESS",""',
    '"1:00:03 PM","openframe-desktop.exe","10","WriteFile","C:\\Users\\tester\\AppData\\Local\\OpenFrame\\settings.json","SUCCESS","Offset: 0"',
    '"1:00:04 PM","openframe-desktop.exe","10","WriteFile","C:\\Users\\tester\\Desktop\\oops.txt","SUCCESS","Offset: 0"',
    '"1:00:05 PM","openframe-desktop.exe","10","CreateFile","C:\\Users\\tester\\Documents\\OpenFrame\\Night, Bus\\db.sqlite","SUCCESS","Desired Access: Generic Write, Read Attributes, Disposition: OpenIf"',
    '"1:00:06 PM","openframe-desktop.exe","10","RegSetValue","HKCU\\Software\\studio.openframe.desktop\\x","SUCCESS","Type: REG_DWORD"',
    '"1:00:07 PM","openframe-desktop.exe","10","RegSetValue","HKCU\\Software\\Other\\y","SUCCESS","Type: REG_DWORD"',
    '"1:00:08 PM","notepad.exe","12","TCP Connect","vm.local:1 -> bad.example.org:80","SUCCESS",""',
    '"1:00:09 PM","msedgewebview2.exe","13","WriteFile","C:\\Users\\tester\\AppData\\Local\\studio.openframe.desktop\\EBWebView\\Cache","SUCCESS",""',
  ];
  const s = await summarize(csv);
  const { text, flagged } = render(s);
  const must = (c, m) => {
    if (!c) throw new Error(`self-test failed: ${m}`);
  };
  must(s.rows === 10, `rows ${s.rows}`);
  must(flagged.some((f) => f.includes("evil.example.com")), "evil host not flagged");
  must(!flagged.some((f) => f.includes("hf.co")), "hugging face flagged");
  must(!flagged.some((f) => f.includes("localhost")), "loopback flagged");
  must(!flagged.some((f) => f.includes("bad.example.org")), "a non-OpenFrame process was reported");
  must(flagged.some((f) => f.includes("Desktop\\oops.txt")), "stray file not flagged");
  must(flagged.some((f) => f.includes("Software\\Other")), "stray registry key not flagged");
  must(!flagged.some((f) => f.includes("db.sqlite") || f.includes("settings.json") || f.includes("EBWebView")), "allowed path flagged");
  must(text.includes("## msedgewebview2.exe"), "webview section missing");
  must(!text.includes("\u2014"), "em dash in output");
  console.log(`procmon-report self-test passed (${flagged.length} flagged as expected)`);
}

async function main() {
  const args = process.argv.slice(2);
  if (args.includes("--self-test")) return selfTest();
  const csv = args.find((a) => a.toLowerCase().endsWith(".csv")) ?? String.raw`C:\w9-vm\procmon\session.csv`;
  const out = args.includes("--out") ? args[args.indexOf("--out") + 1] : join(EVIDENCE, "procmon-summary.md");
  if (!existsSync(csv)) throw new Error(`CSV not found: ${csv}`);
  const lines = createInterface({ input: createReadStream(csv, { encoding: "utf8" }), crlfDelay: Infinity });
  const s = await summarize(lines);
  if (!s.headerSeen) throw new Error("The CSV is empty.");
  const { text, flagged } = render(s, defenderSection());
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, text);
  console.log(`Wrote ${out}: ${s.rows} events, ${s.procs.size} relevant processes, ${flagged.length} flagged.`);
}

main().catch((e) => {
  console.error(e.message);
  process.exitCode = 1;
});
