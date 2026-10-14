// Small, dependency-free source scanners shared by the governance checks
// (scripts/architecture-lint.mjs). They blank comments and string literals while
// keeping every newline, so line numbers stay exact and a pattern that appears only
// in a comment or a message string is not reported as code.

import fs from "node:fs";
import path from "node:path";

/** Recursively list files under `dir` whose name matches `test`, skipping `skipDirs`. */
export function walk(dir, test, skipDirs = new Set(["node_modules", "target", "dist", ".git"])) {
  const out = [];
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!skipDirs.has(entry.name)) out.push(...walk(path.join(dir, entry.name), test, skipDirs));
    } else if (test(entry.name)) {
      out.push(path.join(dir, entry.name));
    }
  }
  return out;
}

const blank = (s) => s.replace(/[^\n]/g, " ");

/**
 * Rust views of a file: `noComments` (comments blanked, strings kept) and `code`
 * (comments and string/char literals blanked). Handles nested block comments, raw
 * strings (r"..", r#".."#, br#".."#), byte strings, char literals and lifetimes.
 */
export function rustViews(src) {
  let noComments = "";
  let code = "";
  let i = 0;
  const n = src.length;
  while (i < n) {
    const c = src[i];
    const next = src[i + 1];
    if (c === "/" && next === "/") {
      const end = src.indexOf("\n", i);
      const stop = end === -1 ? n : end;
      const seg = blank(src.slice(i, stop));
      noComments += seg;
      code += seg;
      i = stop;
      continue;
    }
    if (c === "/" && next === "*") {
      let depth = 1;
      let j = i + 2;
      while (j < n && depth > 0) {
        if (src[j] === "/" && src[j + 1] === "*") { depth++; j += 2; continue; }
        if (src[j] === "*" && src[j + 1] === "/") { depth--; j += 2; continue; }
        j++;
      }
      const seg = blank(src.slice(i, j));
      noComments += seg;
      code += seg;
      i = j;
      continue;
    }
    const raw = /^(b?r)(#*)"/.exec(src.slice(i, i + 40));
    const prevIdent = i > 0 && /[A-Za-z0-9_]/.test(src[i - 1]);
    if (raw && !prevIdent) {
      const close = '"' + raw[2];
      const start = i + raw[0].length;
      const end = src.indexOf(close, start);
      const stop = end === -1 ? n : end + close.length;
      const lit = src.slice(i, stop);
      noComments += lit;
      code += blank(lit);
      i = stop;
      continue;
    }
    if (c === '"' || (c === "b" && next === '"' && !prevIdent)) {
      let j = c === "b" ? i + 2 : i + 1;
      while (j < n && src[j] !== '"') j += src[j] === "\\" ? 2 : 1;
      const lit = src.slice(i, j + 1);
      noComments += lit;
      code += blank(lit);
      i = j + 1;
      continue;
    }
    if (c === "'") {
      const m = /^'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'/.exec(src.slice(i, i + 14));
      if (m) {
        noComments += m[0];
        code += blank(m[0]);
        i += m[0].length;
        continue;
      }
    }
    noComments += c;
    code += c;
    i++;
  }
  return { noComments, code };
}

/**
 * Character ranges of `#[cfg(test)]` items (test modules and test-only functions)
 * found in the `code` view. Text inside these ranges is test code.
 */
export function rustTestRanges(code) {
  const ranges = [];
  const re = /#\[cfg\(test\)\]/g;
  let m;
  while ((m = re.exec(code))) {
    const open = code.indexOf("{", m.index);
    const semi = code.indexOf(";", m.index);
    if (open === -1 || (semi !== -1 && semi < open)) continue;
    let depth = 0;
    let j = open;
    for (; j < code.length; j++) {
      if (code[j] === "{") depth++;
      else if (code[j] === "}" && --depth === 0) break;
    }
    ranges.push([m.index, j + 1]);
    re.lastIndex = j + 1;
  }
  return ranges;
}

/** TypeScript/JavaScript views: `noComments` and `code` (comments and string/template literals blanked). */
export function tsViews(src) {
  let noComments = "";
  let code = "";
  let i = 0;
  const n = src.length;
  while (i < n) {
    const c = src[i];
    const next = src[i + 1];
    if (c === "/" && next === "/") {
      const end = src.indexOf("\n", i);
      const stop = end === -1 ? n : end;
      const seg = blank(src.slice(i, stop));
      noComments += seg; code += seg; i = stop; continue;
    }
    if (c === "/" && next === "*") {
      const end = src.indexOf("*/", i + 2);
      const stop = end === -1 ? n : end + 2;
      const seg = blank(src.slice(i, stop));
      noComments += seg; code += seg; i = stop; continue;
    }
    if (c === '"' || c === "'" || c === "`") {
      let j = i + 1;
      while (j < n && src[j] !== c) {
        if (src[j] === "\n" && c !== "`") break;
        j += src[j] === "\\" ? 2 : 1;
      }
      const lit = src.slice(i, j + 1);
      noComments += lit; code += blank(lit); i = j + 1; continue;
    }
    noComments += c; code += c; i++;
  }
  return { noComments, code };
}

/** 1-based line number of character offset `idx` in `text`. */
export function lineOf(text, idx) {
  let line = 1;
  for (let i = 0; i < idx && i < text.length; i++) if (text[i] === "\n") line++;
  return line;
}

export const inRanges = (ranges, idx) => ranges.some(([a, b]) => idx >= a && idx < b);
