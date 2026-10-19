// A voice note recorded in the app has no length in its header; the drawer's player learns it
// once so it shows the total time and can seek, and starts again from the beginning.

import { describe, expect, it } from "vitest";
import { settleUnknownDuration } from "./media";

function player(duration: number) {
  const el = document.createElement("audio");
  let d = duration;
  let t = 0;
  Object.defineProperty(el, "duration", { get: () => d });
  Object.defineProperty(el, "currentTime", { get: () => t, set: (v: number) => (t = v) });
  return { el, learn: (v: number) => { d = v; el.dispatchEvent(new Event("durationchange")); }, time: () => t };
}

describe("settleUnknownDuration", () => {
  it("reads to the end once when the length is unknown, then goes back to the start", () => {
    const p = player(Infinity);
    settleUnknownDuration(p.el);
    expect(p.time()).toBe(Number.MAX_SAFE_INTEGER);
    p.learn(1.81);
    expect(p.time()).toBe(0);
    // Later changes (the writer seeks) are not touched.
    p.el.currentTime = 1.2;
    p.learn(1.82);
    expect(p.time()).toBe(1.2);
  });

  it("leaves a file that knows its length alone", () => {
    const p = player(12.5);
    settleUnknownDuration(p.el);
    expect(p.time()).toBe(0);
  });
});
