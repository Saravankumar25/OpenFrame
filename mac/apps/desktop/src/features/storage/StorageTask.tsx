// One long storage action (vault backup, export, restore, move; project move) shown in a dialog:
// a confirmation first when the action changes where data lives, then progress with a real
// Cancel, then the result or the plain error. The task itself runs in Rust; this only follows it.

import { useState, type ReactNode } from "react";
import type { JsonValue } from "../../ipc/generated/serde_json/JsonValue";
import type { StorageTaskStarted } from "../../ipc/generated/StorageTaskStarted";
import { Banner, Button, Dialog } from "../../design-system";
import { reportError } from "../../ipc/query";
import { packagesApi } from "../packages/api";
import { TaskProgressBar } from "../packages/parts";
import { isFinished, useTask } from "../packages/store";

export interface StorageAction {
  title: string;
  /** Shown before anything starts. */
  confirm: ReactNode;
  confirmLabel: string;
  start: () => Promise<StorageTaskStarted>;
  /** What finished, in plain words. */
  done: (result: JsonValue | null) => ReactNode;
  onFinished?: () => void;
}

export function StorageTaskDialog({ action, onClose }: { action: StorageAction; onClose: () => void }) {
  const [taskId, setTaskId] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const task = useTask(taskId);
  const running = !!taskId && !isFinished(task);
  const finished = isFinished(task);

  const start = async () => {
    setStarting(true);
    try {
      const r = await action.start();
      setTaskId(r.taskId);
    } catch (e) {
      reportError(e);
    } finally {
      setStarting(false);
    }
  };
  const close = () => {
    if (finished) action.onFinished?.();
    onClose();
  };

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !running && close()}
      title={action.title}
      size="md"
      dismissible={!running}
      footer={
        finished ? (
          <Button variant="primary" onClick={close}>Close</Button>
        ) : (
          <>
            <Button onClick={close} disabled={running}>Cancel</Button>
            <Button variant="primary" disabled={running || starting || !!taskId} onClick={() => void start()}>
              {action.confirmLabel}
            </Button>
          </>
        )
      }
    >
      {!taskId ? (
        <div className="sm">{action.confirm}</div>
      ) : task?.state === "completed" ? (
        <Banner tone="ok">{action.done(task.result)}</Banner>
      ) : task?.state === "failed" ? (
        <Banner tone="err">{task.error?.message ?? "This did not finish. Nothing was changed."}</Banner>
      ) : task?.state === "cancelled" ? (
        <Banner tone="info">Cancelled. Nothing was changed.</Banner>
      ) : (
        <TaskProgressBar
          task={task}
          note="Nothing changes until the copy is checked. Cancel leaves everything where it was."
          onCancel={() => void packagesApi.cancelTask(taskId).catch(reportError)}
        />
      )}
    </Dialog>
  );
}
