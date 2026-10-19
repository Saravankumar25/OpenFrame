// Call Sheets workspace (FSD §38, §105, §147–148; UX §3.31; mocks 141–147).
// List of call sheets; `params.callSheetId` opens the paper-style editor.

import { useNav } from "../../app/stores";
import { CallSheetEditor } from "./CallSheetEditor";
import { CallSheetList } from "./CallSheetList";
import "./shared/schedule-ui.css";

export default function CallSheetsWorkspace() {
  const id = useNav((s) => s.route.params?.callSheetId);
  return id ? <CallSheetEditor key={id} id={id} /> : <CallSheetList />;
}
