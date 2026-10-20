// Export entry points for each workspace (FSD §60.1 "Every major useful
// workspace must have an obvious Export action"). Each button opens the shared
// ExportDialog configured for one document type; workspaces only pass ids.

import { useState } from "react";
import { Download } from "lucide-react";
import { Button, type ButtonProps } from "../../design-system";
import { ExportDialog } from "./ExportDialog";
import { PDF_ONLY, TABLE_FORMATS, type ExportFormatOption, type ExportScope } from "./model";

interface Trigger {
  label?: string;
  size?: ButtonProps["size"];
  variant?: ButtonProps["variant"];
  disabled?: boolean;
}

function useOpen() {
  const [open, setOpen] = useState(false);
  return { open, show: () => setOpen(true), hide: () => setOpen(false) };
}

function TriggerButton({ label = "Export", size, variant, disabled, onClick }: Trigger & { onClick: () => void }) {
  return (
    <Button size={size} variant={variant} disabled={disabled} icon={<Download size={14} />} onClick={onClick}>
      {label}
    </Button>
  );
}

type Item = { id: string; label: string };

// ------------------------------------------------------------------ Story (mock 072)

export function StoryExportButton({ episodeId, selectedIds, ...t }: Trigger & { episodeId: string | null; selectedIds: string[] }) {
  const d = useOpen();
  const scopes: ExportScope[] = [
    { value: "board", label: "Current page (whole Beat Board)", args: { scope: "board", episodeId } },
    {
      value: "selected",
      label: selectedIds.length ? `Selected items only (${selectedIds.length})` : "Selected items only",
      args: { scope: "selected", episodeId, itemIds: selectedIds },
      disabled: selectedIds.length === 0,
    },
    { value: "project", label: "Entire project", args: { scope: "project" } },
  ];
  const formats: ExportFormatOption[] = [
    { value: "pdf", label: "PDF" },
    { value: "txt", label: "Plain text" },
    { value: "csv", label: "CSV" },
  ];
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Beat Board"
        source={<>Source: <b>current Beat Board</b></>}
        documentName="Beat Board"
        op="story.export_board"
        formats={formats}
        scopes={scopes}
        options={[
          { kind: "choice", key: "layout", label: "Layout", default: "outline", formats: ["pdf"], options: [{ value: "outline", label: "Outline PDF" }, { value: "board", label: "Board PDF" }] },
          { kind: "checkbox", key: "includeParking", label: "Include Parking Lot", default: false },
          { kind: "checkbox", key: "includeNotes", label: "Include notes", default: false },
          { kind: "checkbox", key: "includeComments", label: "Include comments", default: false },
        ]}
      />
    </>
  );
}

// ------------------------------------------------------------------ Daily View

/** The day shown on the Daily View, as a document of its own named after the day (owner request
 * 2026-10-20): "Shoot Day 4 - Daily view", with what the page shows. */
export function DailyViewExportButton({ dayId, dayLabel, ...t }: Trigger & { dayId: string; dayLabel: string }) {
  const d = useOpen();
  const name = `${dayLabel} - Daily view`;
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Daily View"
        sub="Everything this page shows for the day: scenes in order, location, cast and call times, notes, breakdown items."
        source={<>Source: <b>{dayLabel}</b></>}
        documentName={name}
        plainName
        op="schedule.export_daily"
        baseArgs={{ dayId }}
        formats={[TABLE_FORMATS[0], { value: "xlsx", label: "Spreadsheet (XLSX)" }, { value: "csv", label: "CSV" }]}
        note="Private notes excluded. Export creates a snapshot and does not change the schedule."
      />
    </>
  );
}

// ------------------------------------------------------------------ Story page

/** The free Story page (owner request 2026-10-20): the text as written, as TXT, PDF or DOCX. */
export function StoryPageExportButton({ onBeforeOpen, ...t }: Trigger & { onBeforeOpen?: () => Promise<unknown> | void }) {
  const d = useOpen();
  return (
    <>
      <TriggerButton {...t} onClick={() => void Promise.resolve(onBeforeOpen?.()).then(d.show)} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Story"
        source={<>Source: <b>the Story page</b></>}
        documentName="Story"
        op="story.export_page"
        formats={[
          { value: "txt", label: "Plain text (TXT)" },
          { value: "pdf", label: "PDF" },
          { value: "docx", label: "Word (DOCX)" },
        ]}
        note="Export creates a copy of the page; the page does not change."
      />
    </>
  );
}

// ------------------------------------------------------------------ Breakdown & Catalog

export function BreakdownExportButton({ sceneId, scenes, ...t }: Trigger & { sceneId?: string | null; scenes: Item[] }) {
  const d = useOpen();
  const scopes: ExportScope[] = [
    { value: "all", label: "All scenes of the Production Source" },
    { value: "current", label: "Current scene", args: { sceneIds: sceneId ? [sceneId] : [] }, disabled: !sceneId },
    { value: "selected", label: "Selected scenes", picker: { argKey: "sceneIds", items: scenes, emptyText: "No scenes in the Production Source yet." } },
  ];
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Breakdown"
        source="Source: the active Production Source"
        documentName="Breakdown Report"
        op="breakdown.export_report"
        formats={TABLE_FORMATS}
        scopes={scopes}
        options={[
          { kind: "choice", key: "content", label: "Contents", default: "both", formats: ["pdf", "xlsx"], options: [{ value: "both", label: "Breakdown by scene and Catalog" }, { value: "scenes", label: "Breakdown by scene" }, { value: "catalog", label: "Catalog with scene usage" }] },
          { kind: "choice", key: "csvContent", arg: "content", label: "Table", default: "scenes", formats: ["csv"], options: [{ value: "scenes", label: "Breakdown by scene" }, { value: "catalog", label: "Catalog with scene usage" }] },
          { kind: "checkbox", key: "includeSuggested", label: "Include suggested elements (marked as suggested)", default: true },
          { kind: "checkbox", key: "includeNotes", label: "Include element notes", default: false },
        ]}
      />
    </>
  );
}

export function CatalogExportButton(t: Trigger) {
  const d = useOpen();
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Catalog"
        sub="The export is a snapshot, not a live connection to the Catalog."
        source="Source: Catalog and the active Production Source"
        documentName="Catalog"
        op="breakdown.export_report"
        baseArgs={{ content: "catalog" }}
        formats={[TABLE_FORMATS[2], TABLE_FORMATS[1], TABLE_FORMATS[0]]}
      />
    </>
  );
}

// ------------------------------------------------------------------ Moodboard (mock 122)

export function MoodboardExportButton({ board, ...t }: Trigger & { board: { id: string; name: string } }) {
  const d = useOpen();
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title={`Export moodboard: ${board.name}`}
        documentName={`Moodboard - ${board.name}`}
        op="moodboard.export_pdf"
        baseArgs={{ moodboardId: board.id }}
        formats={PDF_ONLY}
        options={[
          { kind: "checkbox", key: "includeTitle", label: "Project title and board name", default: true },
          { kind: "checkbox", key: "includeImages", label: "Images", default: true },
          { kind: "checkbox", key: "includeCaptions", label: "Captions", default: true },
          { kind: "checkbox", key: "includeInternalNotes", label: "Internal notes", default: false },
        ]}
        note="Private / internal notes excluded unless you tick Internal notes. Export creates a snapshot and does not change your project."
      />
    </>
  );
}

// ------------------------------------------------------------------ Storyboards

export function StoryboardExportButton({ current, boards, ...t }: Trigger & { current?: { id: string; name: string } | null; boards: Item[] }) {
  const d = useOpen();
  const scopes: ExportScope[] = [
    ...(current ? [{ value: "current", label: `Current storyboard (${current.name})`, args: { storyboardIds: [current.id] } }] : []),
    { value: "all", label: "All storyboards" },
    { value: "selected", label: "Selected storyboards", picker: { argKey: "storyboardIds", items: boards } },
  ];
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Storyboard"
        sub="A clean storyboard sheet: panels with shot numbers, descriptions and framing."
        documentName={current ? `Storyboard - ${current.name}` : "Storyboards"}
        op="storyboard.export_sheet"
        formats={PDF_ONLY}
        scopes={scopes}
        options={[{ kind: "checkbox", key: "includeNotes", label: "Include shot notes", default: false }]}
      />
    </>
  );
}

// ------------------------------------------------------------------ Shot List

export function ShotListExportButton({ scene, scenes, ...t }: Trigger & { scene?: Item | null; scenes: Item[] }) {
  const d = useOpen();
  const scopes: ExportScope[] = [
    ...(scene ? [{ value: "current", label: `Current scene (${scene.label})`, args: { sceneIds: [scene.id] } }] : []),
    { value: "selected", label: "Selected scenes", picker: { argKey: "sceneIds", items: scenes } },
    { value: "project", label: "Whole project" },
  ];
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Shot List"
        documentName="Shot List"
        op="shot.export_list"
        formats={TABLE_FORMATS}
        scopes={scopes}
        options={[{ kind: "checkbox", key: "includeRemoved", label: "Whole project: also shots of scenes removed from the script", default: false }]}
      />
    </>
  );
}

// ------------------------------------------------------------------ Schedule (mock 139)

export function ScheduleExportButton({ days, currentDayId, sourceLabel, ...t }: Trigger & { days: Item[]; currentDayId?: string | null; sourceLabel?: string | null }) {
  const d = useOpen();
  const current = days.find((x) => x.id === currentDayId);
  const scopes: ExportScope[] = [
    { value: "all", label: "Entire schedule" },
    { value: "day", label: current ? `Current day only (${current.label})` : "Current day only", args: { dayIds: current ? [current.id] : [] }, disabled: !current },
    { value: "days", label: "Selected days", picker: { argKey: "dayIds", items: days, emptyText: "Create a shooting day first." } },
  ];
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Shooting Schedule"
        source={<>Source: <b>Shooting Schedule{sourceLabel ? ` · ${sourceLabel}` : ""}</b></>}
        documentName="Shooting Schedule"
        op="schedule.export_schedule"
        formats={[TABLE_FORMATS[0], { value: "xlsx", label: "Spreadsheet (XLSX)" }, { value: "csv", label: "CSV" }]}
        scopes={scopes}
        options={[
          { kind: "checkbox", key: "includeUnscheduled", label: "Include unscheduled scenes", default: true },
          { kind: "checkbox", key: "includeNotes", label: "Include scene notes", default: false },
        ]}
        note="Private notes excluded. Export is a snapshot of the schedule; the schedule is not changed."
      />
    </>
  );
}

// ------------------------------------------------------------------ Day Out of Days

export function DoodExportButton({ args, ...t }: Trigger & { args: Record<string, unknown> }) {
  const d = useOpen();
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Day Out of Days"
        source={<>Source: <b>Day Out of Days</b> · the cast and days shown</>}
        documentName="Day Out of Days"
        op="schedule.export_dood"
        baseArgs={args}
        formats={[TABLE_FORMATS[0], { value: "xlsx", label: "Spreadsheet (XLSX)" }, { value: "csv", label: "CSV" }]}
        note="Corrections are marked with *. Export is a snapshot; the schedule is not changed."
      />
    </>
  );
}

// ------------------------------------------------------------------ Call Sheet (mock 147)

export function CallSheetExportButton({ id, title, status, ...t }: Trigger & { id: string; title: string; status: string }) {
  const d = useOpen();
  const frozen = status === "Final";
  return (
    <>
      <TriggerButton label="Export PDF" {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Call Sheet"
        sub="Creates the PDF snapshot to send."
        source={<>Source: <b>{title}</b> · {status}</>}
        documentName={title}
        op="callsheets.export_pdf"
        baseArgs={{ id }}
        formats={PDF_ONLY}
        options={[{ kind: "checkbox", key: "includeOptional", label: "Include the added sections (weather, extras, department notes, advance schedule, crew, contacts, attachments, reference images)", default: true }]}
        note={
          frozen
            ? "This call sheet is final: the PDF is made from its frozen snapshot and never changes it. Private notes excluded."
            : "Private notes excluded. Export does not change the shooting schedule. Finalize the call sheet before sending it."
        }
      />
    </>
  );
}

// ------------------------------------------------------------------ Sides (mock 148)

export function SidesExportButton({ args, documentName, ...t }: Trigger & { args: Record<string, unknown>; documentName: string }) {
  const d = useOpen();
  return (
    <>
      <TriggerButton label="Export PDF" {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Sides"
        sub="Only the selected scenes, in screenplay format, with the draft name on every page."
        documentName={documentName}
        op="sides.export_pdf"
        baseArgs={args}
        formats={PDF_ONLY}
        note="Sides are a snapshot of the screenplay source. Editing the PDF never changes the screenplay. Private notes excluded."
      />
    </>
  );
}

// ------------------------------------------------------------------ Reports (mock 149)

export function ReportExportButton({ reportType, filter, reportId, title, ...t }: Trigger & { reportType?: string | null; filter?: string | null; reportId?: string | null; title: string }) {
  const d = useOpen();
  const baseArgs = reportId ? { reportId } : { reportType, filter: filter || null };
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title={`Export ${title}`}
        source={reportId ? <>Source: saved report <b>{title}</b></> : <>Source: <b>{title}</b>{filter ? ` · filtered by “${filter}”` : ""}</>}
        documentName={title}
        op="reports.export_report"
        baseArgs={baseArgs}
        formats={TABLE_FORMATS}
      />
    </>
  );
}

// ------------------------------------------------------------------ Budget

export function BudgetExportButton({ budgetId, ...t }: Trigger & { budgetId?: string | null }) {
  const d = useOpen();
  return (
    <>
      <TriggerButton {...t} onClick={d.show} />
      <ExportDialog
        open={d.open}
        onClose={d.hide}
        title="Export Budget"
        sub="A simple budget summary: planned total, contingency, category totals and line items."
        documentName={budgetId ? "Budget Snapshot" : "Budget Summary"}
        op="budget.export_summary"
        baseArgs={budgetId ? { budgetId } : {}}
        formats={TABLE_FORMATS}
        options={[{ kind: "checkbox", key: "includeNotes", label: "Include line notes", default: false }]}
      />
    </>
  );
}
