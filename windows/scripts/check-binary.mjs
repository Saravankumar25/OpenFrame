#!/usr/bin/env node
// Hardening check of a built Windows executable (security standard, "binary hardening"; Windows phase W4).
//
//   node scripts/check-binary.mjs <file.exe | file.dll>...     exit 1 when a check fails
//
// Reads the PE headers and requires: 64-bit image, address space layout randomisation with high entropy
// (DYNAMIC_BASE, HIGH_ENTROPY_VA), data execution prevention (NX_COMPAT), control-flow guard (GUARD_CF in
// the header and the CF-instrumented flag in the load config), CET shadow-stack compatibility (CETCOMPAT in the
// extended DLL characteristics debug entry), and no COFF symbol table (symbols stripped).
// It prints the PDB path the debug directory names, if any, so a personal folder in it is seen; the string
// scan in scripts/scan-installer.mjs refuses personal paths.

import fs from "node:fs";
import url from "node:url";

const DLL = { HIGH_ENTROPY_VA: 0x20, DYNAMIC_BASE: 0x40, NX_COMPAT: 0x100, GUARD_CF: 0x4000 };
const IMAGE_GUARD_CF_INSTRUMENTED = 0x100;

/** Facts about one PE image, from its bytes. Throws on a file that is not a PE image. */
export function peFacts(buf) {
  if (buf.readUInt16LE(0) !== 0x5a4d) throw new Error("not an MZ file");
  const pe = buf.readUInt32LE(0x3c);
  if (buf.readUInt32LE(pe) !== 0x4550) throw new Error("no PE signature");
  const coff = pe + 4;
  const sections = buf.readUInt16LE(coff + 2);
  const symbolTable = buf.readUInt32LE(coff + 8);
  const symbols = buf.readUInt32LE(coff + 12);
  const optSize = buf.readUInt16LE(coff + 16);
  const opt = coff + 20;
  if (buf.readUInt16LE(opt) !== 0x20b) return { pe64: false };
  const dllCharacteristics = buf.readUInt16LE(opt + 70);
  const dirs = opt + 112;
  const dir = (i) => ({ rva: buf.readUInt32LE(dirs + i * 8), size: buf.readUInt32LE(dirs + i * 8 + 4) });
  const sectionTable = opt + optSize;
  const toOffset = (rva) => {
    for (let i = 0; i < sections; i++) {
      const s = sectionTable + i * 40;
      const va = buf.readUInt32LE(s + 12);
      const size = Math.max(buf.readUInt32LE(s + 8), buf.readUInt32LE(s + 16));
      if (rva >= va && rva < va + size) return buf.readUInt32LE(s + 20) + (rva - va);
    }
    return -1;
  };
  let guardFlags = 0;
  const lc = dir(10);
  if (lc.rva && lc.size >= 0x94) {
    const off = toOffset(lc.rva);
    // IMAGE_LOAD_CONFIG_DIRECTORY64.GuardFlags sits at offset 0x90.
    if (off >= 0 && buf.readUInt32LE(off) >= 0x94) guardFlags = buf.readUInt32LE(off + 0x90);
  }
  let pdb = null;
  let cetCompat = false;
  const dbg = dir(6);
  const dbgOff = dbg.rva ? toOffset(dbg.rva) : -1;
  for (let i = 0; dbgOff >= 0 && i < dbg.size / 28; i++) {
    const e = dbgOff + i * 28;
    const type = buf.readUInt32LE(e + 12);
    // IMAGE_DEBUG_TYPE_EX_DLLCHARACTERISTICS (20): a DWORD of extended flags; bit 0 is CET_COMPAT.
    if (type === 20 && buf.readUInt32LE(e + 16) >= 4) cetCompat = (buf.readUInt32LE(buf.readUInt32LE(e + 24)) & 1) !== 0;
    if (type !== 2) continue; // IMAGE_DEBUG_TYPE_CODEVIEW
    const raw = buf.readUInt32LE(e + 24);
    if (buf.toString("latin1", raw, raw + 4) === "RSDS") pdb = buf.toString("utf8", raw + 24, buf.indexOf(0, raw + 24));
  }
  return { pe64: true, dllCharacteristics, guardFlags, cetCompat, symbolTable, symbols, pdb };
}

/** The failed checks for these facts, as plain sentences; empty when the image passes. */
export function failures(f) {
  if (!f.pe64) return ["not a 64-bit image"];
  const out = [];
  for (const [name, bit] of Object.entries(DLL)) if (!(f.dllCharacteristics & bit)) out.push(`${name} is not set`);
  if (!(f.guardFlags & IMAGE_GUARD_CF_INSTRUMENTED)) out.push("the load config does not mark the code as control-flow guard instrumented");
  if (!f.cetCompat) out.push("CETCOMPAT is not set (the image is not marked for the hardware shadow stack)");
  if (f.symbolTable !== 0 || f.symbols !== 0) out.push(`a COFF symbol table is present (${f.symbols} symbols)`);
  return out;
}

if (import.meta.url === url.pathToFileURL(process.argv[1] ?? "").href) {
  const files = process.argv.slice(2);
  if (files.length === 0) {
    console.error("usage: node scripts/check-binary.mjs <file.exe>...");
    process.exit(2);
  }
  let bad = 0;
  for (const file of files) {
    const f = peFacts(fs.readFileSync(file));
    const fails = failures(f);
    console.log(`${file}: ${fails.length ? "FAIL" : "ok"} (pdb: ${f.pdb ?? "none"})`);
    for (const m of fails) console.log(`  - ${m}`);
    if (fails.length) bad++;
  }
  process.exit(bad ? 1 : 0);
}
