import { describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useDebouncedSave } from "./useDebouncedSave";

vi.mock("../../ipc/query", () => ({ reportError: vi.fn() }));

describe("debounced save (W2 review)", () => {
  it("keeps a value whose save failed, so the next flush stores it", async () => {
    const save = vi.fn<(v: string) => Promise<void>>().mockRejectedValueOnce(new Error("disk full")).mockResolvedValue(undefined);
    const { result } = renderHook(() => useDebouncedSave(save, 10_000));
    result.current.schedule("last words");
    expect(await result.current.flush()).toBe(false);
    expect(await result.current.flush()).toBe(true);
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith("last words");
  });

  it("waits for a save already running before it reports the text stored", async () => {
    let finish: () => void = () => undefined;
    const save = vi.fn(() => new Promise<void>((r) => (finish = r)));
    const { result } = renderHook(() => useDebouncedSave(save, 10_000));
    result.current.schedule("typed");
    const first = result.current.flush();
    let done = false;
    const second = result.current.flush().then((v) => ((done = true), v));
    await Promise.resolve();
    expect(done).toBe(false);
    finish();
    expect(await first).toBe(true);
    expect(await second).toBe(true);
    expect(save).toHaveBeenCalledTimes(1);
  });
});
