// Editor ⇄ Rust synchronisation. Rust stays the source of truth:
//
// * local edits mark the editor dirty; after `delay` ms of quiet (or on
//   flush()) the diff between the baseline (what Rust has) and the editor is
//   sent as element-level ops; on success the baseline advances;
// * documents read from Rust carry an edit sequence number; a read older than
//   the editor's own last save is ignored, a newer/different one replaces the
//   editor content — but never while local edits are pending.

import type { ScreenplayEditOp } from "../../../ipc/generated/ScreenplayEditOp";
import { diffSnapshots, snapshotsEqual, type Snapshot } from "./model";

export type SyncStatus = "saved" | "pending" | "saving" | "error";

export interface SyncDeps {
  /** Send ops to Rust; resolves with the new edit sequence. */
  send: (ops: ScreenplayEditOp[]) => Promise<{ seq: number }>;
  /** Current editor content. */
  read: () => Snapshot;
  /** Replace editor content with a Rust document. */
  apply: (snap: Snapshot) => void;
  /** A save failed. Return "retry" to keep the local edits and try again later. */
  onError: (e: unknown) => "retry" | "revert";
  onStatus?: (s: SyncStatus) => void;
}

export class SyncController {
  private baseline: Snapshot;
  private ackSeq: number;
  private dirty = false;
  private inflight: Promise<void> | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private latest: { seq: number; snap: Snapshot } | null = null;
  private deps: SyncDeps;
  private delay: number;
  private status: SyncStatus = "saved";
  private disposed = false;

  constructor(initial: Snapshot, seq: number, deps: SyncDeps, delay = 400) {
    this.baseline = initial;
    this.ackSeq = seq;
    this.deps = deps;
    this.delay = delay;
  }

  get isDirty(): boolean {
    return this.dirty || this.inflight !== null;
  }

  private setStatus(s: SyncStatus) {
    if (s !== this.status) {
      this.status = s;
      this.deps.onStatus?.(s);
    }
  }

  /** The editor content changed locally. */
  markDirty(): void {
    this.dirty = true;
    this.setStatus("pending");
    this.schedule(this.delay);
  }

  private schedule(ms: number) {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.flush();
    }, ms);
  }

  /** Send pending edits now and wait until Rust has them. */
  async flush(): Promise<void> {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    while (this.inflight) await this.inflight;
    if (!this.dirty || this.disposed) return;
    const snap = this.deps.read();
    const ops = diffSnapshots(this.baseline, snap);
    this.dirty = false;
    if (ops.length === 0) {
      this.setStatus("saved");
      this.tryApplyServer();
      return;
    }
    this.setStatus("saving");
    const run = (async () => {
      try {
        const r = await this.deps.send(ops);
        this.baseline = snap;
        this.ackSeq = Math.max(this.ackSeq, r.seq);
        this.setStatus(this.dirty ? "pending" : "saved");
      } catch (e) {
        if (this.deps.onError(e) === "retry") {
          // Keep the local text; the next diff against the unchanged baseline resends it.
          this.dirty = true;
          this.setStatus("error");
          this.schedule(3000);
        } else {
          this.dirty = false;
          this.setStatus("saved");
          const back = this.latest && this.latest.seq >= this.ackSeq ? this.latest.snap : this.baseline;
          this.baseline = back;
          this.deps.apply(back);
        }
      }
    })();
    this.inflight = run;
    await run;
    this.inflight = null;
    this.tryApplyServer();
  }

  /** A document read from Rust. */
  receiveServer(seq: number, snap: Snapshot): void {
    this.latest = { seq, snap };
    this.tryApplyServer();
  }

  private tryApplyServer() {
    if (this.dirty || this.inflight || this.disposed) return;
    const s = this.latest;
    if (!s || s.seq < this.ackSeq) return;
    if (snapshotsEqual(s.snap, this.baseline)) return;
    this.baseline = s.snap;
    this.ackSeq = s.seq;
    this.deps.apply(s.snap);
  }

  dispose(): void {
    this.disposed = true;
    if (this.timer) clearTimeout(this.timer);
  }
}
