// Files on disk reach the interface only through the app's `ofasset` protocol. Tauri's own asset
// protocol is switched off, so a URL built for it never loads: the Idea Vault's voice notes,
// images and video, character pictures and call sheet attachment thumbnails once used it and
// showed "can't be previewed" or nothing at all.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const convertFileSrc = vi.fn((path: string, scheme = "asset") => `http://${scheme}.localhost/${encodeURIComponent(path)}`);
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (p: string, s?: string) => convertFileSrc(p, s) }));

const { ASSET_SCHEME, assetSrc, fileSrc } = await import("./assetSrc");

const asset = (over: Record<string, unknown> = {}) =>
  ({ id: "a1", available: true, path: "C:\\Films\\Letter Town.openframe\\assets\\voice.webm", ...over }) as never;

describe("assetSrc", () => {
  beforeEach(() => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    convertFileSrc.mockClear();
  });
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it("builds the URL on the app's ofasset protocol", () => {
    expect(ASSET_SCHEME).toBe("ofasset");
    expect(assetSrc(asset())).toBe(`http://ofasset.localhost/${encodeURIComponent("C:\\Films\\Letter Town.openframe\\assets\\voice.webm")}`);
    expect(convertFileSrc).toHaveBeenCalledWith("C:\\Films\\Letter Town.openframe\\assets\\voice.webm", "ofasset");
  });

  it("gives nothing for a missing file, no path, or outside the desktop app", () => {
    expect(assetSrc(asset({ available: false }))).toBeNull();
    expect(assetSrc(asset({ path: null }))).toBeNull();
    expect(assetSrc(null)).toBeNull();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    expect(fileSrc("C:\\x.png")).toBeNull();
  });

  it("gives nothing when the path cannot be converted", () => {
    convertFileSrc.mockImplementationOnce(() => {
      throw new Error("bad path");
    });
    expect(fileSrc("C:\\x.png")).toBeNull();
  });
});

describe("no screen builds a file URL by itself", () => {
  const src = join(__dirname, "..");
  const walk = (dir: string, out: string[] = []): string[] => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) {
        if (name !== "generated" && name !== "node_modules") walk(p, out);
      } else if (/\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name)) out.push(p);
    }
    return out;
  };

  it("only ipc/assetSrc.ts calls convertFileSrc", () => {
    const callers = walk(src)
      .filter((f) => /\bconvertFileSrc\b/.test(readFileSync(f, "utf8")))
      .map((f) => relative(src, f).replace(/\\/g, "/"));
    expect(callers).toEqual(["ipc/assetSrc.ts"]);
  });
});
