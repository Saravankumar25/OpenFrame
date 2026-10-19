import { useEffect } from "react";
import { onAppEvent } from "../ipc/client";
import { useNav } from "./stores";
import { toast } from "./toast";

/** Shell-level reactions to backend events (project closed elsewhere, background task results). */
export function useAppEvents(): void {
  useEffect(() => {
    const un = onAppEvent((ev) => {
      if (ev.type === "projectClosed") {
        useNav.getState().reset();
      } else if (ev.type === "task") {
        if (ev.state === "failed" && ev.error) toast.error(ev.error.message);
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);
}
