import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPendingSaves, registerPendingSave } from "./pendingSaves";

const offs: Array<() => void> = [];
const register = (f: Parameters<typeof registerPendingSave>[0]) => offs.push(registerPendingSave(f));

afterEach(() => {
  while (offs.length) offs.pop()!();
  vi.useRealTimers();
});

describe("pending saves", () => {
  it("is done at once when nothing is waiting", async () => {
    expect(await flushPendingSaves()).toBe(true);
  });

  it("runs every registered flush and waits for all of them", async () => {
    const order: string[] = [];
    register(async () => {
      await new Promise((r) => setTimeout(r, 20));
      order.push("slow");
    });
    register(() => {
      order.push("fast");
    });
    expect(await flushPendingSaves()).toBe(true);
    expect(order.sort()).toEqual(["fast", "slow"]);
  });

  it("reports text that could not be stored, and a flush that throws", async () => {
    register(() => true);
    register(() => false);
    expect(await flushPendingSaves()).toBe(false);
    offs.pop()!();
    register(() => Promise.reject(new Error("disk full")));
    expect(await flushPendingSaves()).toBe(false);
  });

  it("never waits longer than the time limit", async () => {
    vi.useFakeTimers();
    register(() => new Promise(() => undefined));
    const done = flushPendingSaves(1000);
    await vi.advanceTimersByTimeAsync(1000);
    expect(await done).toBe(false);
  });

  it("forgets a flush once it is unregistered", async () => {
    const f = vi.fn(() => false);
    register(f);
    offs.pop()!();
    expect(await flushPendingSaves()).toBe(true);
    expect(f).not.toHaveBeenCalled();
  });
});
