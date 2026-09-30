// Open the day's call sheet, creating it from the shooting day when none exists
// (FSD-CALL-001: "Create Call Sheet" from a day).

import { call, OpError } from "../../../ipc/client";
import { reportError } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import { useNav } from "../../../app/stores";

export async function openCallSheetForDay(dayId: string, existingId?: string | null): Promise<void> {
  const go = useNav.getState().go;
  if (existingId) {
    go({ workspace: "callsheets", params: { callSheetId: existingId } });
    return;
  }
  try {
    const id = await call<string>("callsheets.create", { dayId });
    toast.undoable("Created a call sheet from the shooting day");
    go({ workspace: "callsheets", params: { callSheetId: id } });
  } catch (e) {
    if (e instanceof OpError && e.code === "conflict.call_sheet_exists" && e.detail) {
      go({ workspace: "callsheets", params: { callSheetId: e.detail } });
      return;
    }
    reportError(e);
  }
}
