// Writes apps/macos/Tests/OpenFrameUITests/theme-reference.json: the sRGB hex of a few key tokens for
// every theme in both modes, computed here with an independent implementation of the CSS maths
// (the CSS Color 4 XYZ matrices, hand-written token rules). The Swift tests compare the Mac theme
// engine against this file with a tolerance of 1/255 per channel.
//
// Run: node scripts/gen-theme-reference.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const file = JSON.parse(
  readFileSync(join(root, "crates/openframe-application/src/modules/appearance/themes.json"), "utf8"),
);
const E = file.engine;
const TOKENS = ["bg", "panel", "ink", "accent", "accent-soft", "side", "red", "yellow-t", "lg1"];

const mul = (m, v) => m.map((row) => row[0] * v[0] + row[1] * v[1] + row[2] * v[2]);
const LMS_TO_XYZ = [
  [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
  [-0.0405757452148008, 1.112286803280317, -0.0717110580655164],
  [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
];
const XYZ_TO_RGB = [
  [3.2409699419045226, -1.537383177570094, -0.4986107602930034],
  [-0.9692436362808796, 1.8759675015077202, 0.0415550574071756],
  [0.0556300796969936, -0.2039769588889765, 1.0569715142428786],
];
const RGB_TO_XYZ = [
  [0.4123907992659595, 0.357584339383878, 0.1804807884018343],
  [0.2126390058715104, 0.7151686787677559, 0.0721923153607337],
  [0.0193308187155918, 0.1191947797946259, 0.9505321522496607],
];
const XYZ_TO_LMS = [
  [0.819022437996703, 0.3619062600528904, -0.1288737815209879],
  [0.0329836539323885, 0.9292868615863434, 0.0361446663506424],
  [0.0481771893596242, 0.2642395317527308, 0.6335478284084480],
];
const LMS_TO_LAB = [
  [0.210454268309314, 0.7936177747023054, -0.0040720430116193],
  [1.9779985324311684, -2.4285922420485799, 0.450593709617411],
  [0.0259040424655478, 0.7827717124575296, -0.8086757549230774],
];
const LAB_TO_LMS = [
  [1, 0.3963377773761749, 0.2158037573099136],
  [1, -0.1055613458156586, -0.0638541728258133],
  [1, -0.0894841775298119, -1.2914855480194092],
];

const lin = (v) => (Math.abs(v) <= 0.04045 ? v / 12.92 : Math.sign(v) * ((Math.abs(v) + 0.055) / 1.055) ** 2.4);
const gam = (v) => (Math.abs(v) <= 0.0031308 ? v * 12.92 : Math.sign(v) * (1.055 * Math.abs(v) ** (1 / 2.4) - 0.055));

const lch = (L, C, H) => {
  const h = (H * Math.PI) / 180;
  return { lab: [L, C * Math.cos(h), C * Math.sin(h)], alpha: 1 };
};
const fromSrgb = (r, g, b) => {
  const lms = mul(XYZ_TO_LMS, mul(RGB_TO_XYZ, [lin(r), lin(g), lin(b)])).map(Math.cbrt);
  return { lab: mul(LMS_TO_LAB, lms), alpha: 1 };
};
const hex = (s) => {
  let d = s.slice(1);
  if (d.length === 3) d = [...d].map((c) => c + c).join("");
  return fromSrgb(...[0, 2, 4].map((i) => parseInt(d.slice(i, i + 2), 16) / 255));
};
const toHex = (c) => {
  const lms = mul(LAB_TO_LMS, c.lab).map((x) => x ** 3);
  const rgb = mul(XYZ_TO_RGB, mul(LMS_TO_XYZ, lms));
  return (
    "#" +
    rgb
      .map((v) => Math.round(Math.min(1, Math.max(0, gam(Math.min(1, Math.max(0, v))))) * 255))
      .map((n) => n.toString(16).padStart(2, "0"))
      .join("")
  );
};
const mix = (a, pa, b) => {
  const w = pa / 100;
  return { lab: a.lab.map((x, i) => x * w + b.lab[i] * (1 - w)), alpha: 1 };
};
const colour = (s) => {
  if (s.startsWith("#")) return hex(s);
  const m = /^oklch\(([\d.]+)% ([\d.]+) ([\d.]+)\)$/.exec(s);
  if (!m) throw new Error(`unsupported override ${s}`);
  return lch(+m[1] / 100, +m[2], +m[3]);
};

function resolve(theme, mode) {
  const s = theme.seeds;
  const over = theme.overrides?.[mode] ?? {};
  const cache = {};
  const rules = {
    accent: () => lch((E.accentLightness[mode] + s.accentLightnessNudge) / 100, s.accentChroma, s.accentHue),
    "accent-soft": () => mix(tok("accent"), E.softMix[mode], tok("panel")),
    "yellow-t": () => mix(tok("yellow"), E.warningTextMix, tok("ink")),
    lg1: () => {
      const a = tok("accent");
      const C = Math.hypot(a.lab[1], a.lab[2]);
      const H = (Math.atan2(a.lab[2], a.lab[1]) * 180) / Math.PI - 30;
      return lch(a.lab[0], C, H);
    },
  };
  for (const [name, [l, k]] of Object.entries(E.ladder[mode])) {
    rules[name] = () => lch(l / 100, s.neutralTint * k, s.neutralHue);
  }
  for (const [name, [dl, c, h]] of Object.entries(E.status)) {
    rules[name] = () => lch((E.statusLightness[mode] + dl) / 100, c, h);
  }
  function tok(name) {
    if (cache[name]) return cache[name];
    const v = over[name] !== undefined ? colour(over[name]) : rules[name]();
    return (cache[name] = v);
  }
  return Object.fromEntries(TOKENS.map((t) => [t, toHex(tok(t))]));
}

const out = { generatedBy: "scripts/gen-theme-reference.mjs", tokens: TOKENS, themes: {} };
for (const t of file.themes) out.themes[t.id] = { light: resolve(t, "light"), dark: resolve(t, "dark") };
const dest = join(root, "apps/macos/Tests/OpenFrameUITests/theme-reference.json");
writeFileSync(dest, JSON.stringify(out, null, 1) + "\n");
console.log(`wrote ${dest}`);
