// Small building blocks shared by the package dialogs.

import { FolderOpen } from "lucide-react";
import type { ReactNode } from "react";
import { revealLocation } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { BackupFileEntry } from "../../ipc/generated/BackupFileEntry";
import type { TaskProgress } from "../../ipc/generated/TaskProgress";
import { Banner, Button, Chip, type ChipTone } from "../../design-system";

/** Progress of a background package task with a real Cancel (cancels the task). */
export function TaskProgressBar({ task, onCancel }: { task: TaskProgress | undefined; onCancel: () => void }) {
  const pct = Math.round((task?.progress ?? 0) * 100);
  return (
    <div className="card pad" role="status" aria-live="polite">
      <div className="row" style={{ marginBottom: 6 }}>
        <b className="sm">{task?.message ?? "Starting…"}</b>
        <span className="grow" />
        <span className="xs muted">{pct}%</span>
      </div>
      <div style={{ height: 6, borderRadius: 3, background: "var(--line)", overflow: "hidden" }} aria-hidden>
        <div style={{ width: `${pct}%`, height: "100%", background: "var(--accent)", transition: "width .2s" }} />
      </div>
      <div className="row" style={{ marginTop: 8 }}>
        <span className="xs muted">Your project stays open and saved while this runs.</span>
        <span className="grow" />
        <Button size="sm" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

const STATUS_TONE: Record<string, ChipTone> = { Included: "g", Copied: "g", External: "y", Missing: "r" };
const STATUS_LABEL: Record<string, string> = {
  Included: "Included",
  Copied: "Included (copied)",
  External: "External — not copied",
  Missing: "Missing",
};

/** "What this backup contains" rows (mockup 169). */
export function FileStatusTable({ rows }: { rows: { label: ReactNode; status: string; count: number }[] }) {
  return (
    <table className="tbl">
      <tbody>
        {rows.map((r, i) => (
          <tr key={i}>
            <td>
              {r.label} ({r.count})
            </td>
            <td style={{ width: 170 }}>
              <Chip tone={STATUS_TONE[r.status] ?? "default"}>{STATUS_LABEL[r.status] ?? r.status}</Chip>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Names of linked files with their status (never claims a missing file is included). */
export function LinkedFiles({ files }: { files: BackupFileEntry[] }) {
  if (files.length === 0) return null;
  return (
    <ul className="xs muted" style={{ margin: "6px 0 0", paddingLeft: 18 }}>
      {files.slice(0, 8).map((f) => (
        <li key={f.assetId}>
          {f.name} — {STATUS_LABEL[f.status] ?? f.status}
          {f.path ? <span className="selectable-text"> · {f.path}</span> : null}
        </li>
      ))}
      {files.length > 8 && <li>and {files.length - 8} more</li>}
    </ul>
  );
}

export function ShowInFolder({ path }: { path: string }) {
  return (
    <Button size="sm" icon={<FolderOpen size={13} />} onClick={() => void revealLocation("exported", undefined, path).catch(reportError)}>
      Show in folder
    </Button>
  );
}

export function DoneBanner({ title, children, path }: { title: string; children?: ReactNode; path?: string }) {
  return (
    <Banner tone="ok" actions={path ? <ShowInFolder path={path} /> : undefined}>
      <b>{title}</b>
      {children && <div className="sm">{children}</div>}
      {path && <div className="xs muted selectable-text" style={{ wordBreak: "break-all" }}>{path}</div>}
    </Banner>
  );
}
