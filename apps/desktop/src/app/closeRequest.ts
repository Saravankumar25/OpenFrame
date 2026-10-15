// Window close (Windows phase W2). The close button and Alt+F4 reach Rust as a close request; Rust
// holds the window open and sends `of://close-requested`. This answers at once ("flushing", so
// Rust's fallback timer stands down), stores the text every editor still holds, then lets the
// window close. If some text could not be stored, the leave guard asks the user first ("held":
// the window stays, and the next close request starts again). If this page never answers, Rust
// closes the window by itself after a few seconds, so a stuck page cannot keep the app open.

import { useEffect } from "react";
import { closeWindowStep as step, inTauri, onCloseRequested } from "../ipc/client";
import { reportError } from "../ipc/query";
import { flushPendingSaves } from "./pendingSaves";
import { useLeaveGuard } from "./shell/SaveStatus";

export async function answerCloseRequest(): Promise<void> {
  await step("flushing");
  if (await flushPendingSaves()) {
    await step("close");
    return;
  }
  await step("held");
  useLeaveGuard.getState().ask("this project", () => void step("close").catch(reportError));
}

export function useCloseRequest(): void {
  useEffect(() => {
    if (!inTauri()) return;
    const un = onCloseRequested(() => void answerCloseRequest().catch(reportError));
    return () => {
      void un.then((f) => f());
    };
  }, []);
}
