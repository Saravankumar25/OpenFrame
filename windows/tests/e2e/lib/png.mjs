// Minimal PNG decode/encode and pixel comparison for the visual-regression harness
// (build plan item screen visual comparison). Node built-ins only (zlib): no image dependency.
//
// Supported input: 8-bit, non-interlaced PNGs of colour type 0 (grey), 2 (RGB), 4 (grey + alpha)
// and 6 (RGBA), which covers everything WebView2 / WebDriver screenshots produce. Anything else is
// rejected with a clear error rather than compared wrongly.

import { deflateSync, inflateSync } from "node:zlib";

const SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const CHANNELS = { 0: 1, 2: 3, 4: 2, 6: 4 };

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  return pb <= pc ? b : c;
}

/**
 * Decode a PNG buffer to `{ width, height, data }` where `data` is RGBA (4 bytes per pixel).
 * @param {Buffer} buf
 */
export function decodePng(buf) {
  if (buf.length < 8 || !buf.subarray(0, 8).equals(SIGNATURE)) throw new Error("not a PNG file");
  let pos = 8;
  let width = 0;
  let height = 0;
  let depth = 0;
  let ctype = -1;
  let interlace = 0;
  const idat = [];
  while (pos + 8 <= buf.length) {
    const len = buf.readUInt32BE(pos);
    const type = buf.toString("latin1", pos + 4, pos + 8);
    if (pos + 12 + len > buf.length) throw new Error(`truncated PNG chunk ${type}`);
    const body = buf.subarray(pos + 8, pos + 8 + len);
    if (crc32(buf.subarray(pos + 4, pos + 8 + len)) !== buf.readUInt32BE(pos + 8 + len)) throw new Error(`bad CRC in PNG chunk ${type}`);
    if (type === "IHDR") {
      width = body.readUInt32BE(0);
      height = body.readUInt32BE(4);
      depth = body[8];
      ctype = body[9];
      interlace = body[12];
    } else if (type === "IDAT") {
      idat.push(body);
    } else if (type === "IEND") {
      break;
    }
    pos += 12 + len;
  }
  const channels = CHANNELS[ctype];
  if (!channels || depth !== 8 || interlace !== 0) {
    throw new Error(`unsupported PNG (bit depth ${depth}, colour type ${ctype}, interlace ${interlace}); expected 8-bit non-interlaced`);
  }
  if (width === 0 || height === 0 || width * height > 64_000_000) throw new Error(`implausible PNG size ${width}x${height}`);
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  if (raw.length < height * (stride + 1)) throw new Error("PNG image data is shorter than its header says");
  const px = Buffer.alloc(height * stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const src = y * (stride + 1) + 1;
    const out = y * stride;
    const prev = out - stride;
    for (let x = 0; x < stride; x++) {
      const v = raw[src + x];
      const a = x >= channels ? px[out + x - channels] : 0;
      const b = y > 0 ? px[prev + x] : 0;
      const c = x >= channels && y > 0 ? px[prev + x - channels] : 0;
      let r;
      switch (filter) {
        case 0: r = v; break;
        case 1: r = v + a; break;
        case 2: r = v + b; break;
        case 3: r = v + ((a + b) >> 1); break;
        case 4: r = v + paeth(a, b, c); break;
        default: throw new Error(`bad PNG filter type ${filter} on row ${y}`);
      }
      px[out + x] = r & 0xff;
    }
  }
  const data = Buffer.alloc(width * height * 4);
  for (let i = 0, j = 0; i < width * height; i++, j += channels) {
    const o = i * 4;
    if (channels >= 3) {
      data[o] = px[j];
      data[o + 1] = px[j + 1];
      data[o + 2] = px[j + 2];
      data[o + 3] = channels === 4 ? px[j + 3] : 255;
    } else {
      data[o] = data[o + 1] = data[o + 2] = px[j];
      data[o + 3] = channels === 2 ? px[j + 1] : 255;
    }
  }
  return { width, height, data };
}

function chunk(type, body) {
  const head = Buffer.alloc(8);
  head.writeUInt32BE(body.length, 0);
  head.write(type, 4, "latin1");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), body])), 0);
  return Buffer.concat([head, body, crc]);
}

/**
 * Encode RGBA pixels as an 8-bit RGBA PNG (filter "Up" on every row, which compresses UI
 * screenshots well and keeps the encoder trivial).
 * @param {{ width: number, height: number, data: Buffer }} image
 */
export function encodePng({ width, height, data }) {
  const stride = width * 4;
  const raw = Buffer.alloc(height * (stride + 1));
  for (let y = 0; y < height; y++) {
    const o = y * (stride + 1);
    raw[o] = y === 0 ? 0 : 2;
    for (let x = 0; x < stride; x++) {
      const v = data[y * stride + x];
      raw[o + 1 + x] = y === 0 ? v : (v - data[(y - 1) * stride + x]) & 0xff;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([SIGNATURE, chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw, { level: 6 })), chunk("IEND", Buffer.alloc(0))]);
}

/**
 * Compare two decoded images.
 *
 * A pixel "differs" when any of its R, G or B channels differs by more than `threshold`
 * (0..255). Pixels inside `ignore` rectangles (device pixels: { x, y, w, h }) are skipped in both
 * images. Returns the counts and a diff image: the baseline in faded grey, differing pixels in
 * red, ignored regions tinted blue.
 */
export function comparePixels(baseline, actual, { threshold, ignore = [] }) {
  if (baseline.width !== actual.width || baseline.height !== actual.height) {
    return { sizeMismatch: true, differing: 0, compared: 0, ratio: 1, diff: null };
  }
  const { width, height } = baseline;
  const mask = new Uint8Array(width * height);
  for (const r of ignore) {
    const x0 = Math.max(0, Math.floor(r.x));
    const y0 = Math.max(0, Math.floor(r.y));
    const x1 = Math.min(width, Math.ceil(r.x + r.w));
    const y1 = Math.min(height, Math.ceil(r.y + r.h));
    for (let y = y0; y < y1; y++) mask.fill(1, y * width + x0, y * width + Math.max(x0, x1));
  }
  const a = baseline.data;
  const b = actual.data;
  const out = Buffer.alloc(width * height * 4);
  let differing = 0;
  let compared = 0;
  for (let i = 0; i < width * height; i++) {
    const o = i * 4;
    const grey = Math.round(0.3 * a[o] + 0.59 * a[o + 1] + 0.11 * a[o + 2]);
    const faded = 255 - Math.round((255 - grey) * 0.25);
    if (mask[i]) {
      out[o] = faded >> 1;
      out[o + 1] = faded >> 1;
      out[o + 2] = 255;
      out[o + 3] = 255;
      continue;
    }
    compared++;
    const d = Math.max(Math.abs(a[o] - b[o]), Math.abs(a[o + 1] - b[o + 1]), Math.abs(a[o + 2] - b[o + 2]));
    if (d > threshold) {
      differing++;
      out[o] = 230;
      out[o + 1] = 0;
      out[o + 2] = 0;
    } else {
      out[o] = out[o + 1] = out[o + 2] = faded;
    }
    out[o + 3] = 255;
  }
  return { sizeMismatch: false, differing, compared, ratio: compared ? differing / compared : 0, diff: { width, height, data: out } };
}
