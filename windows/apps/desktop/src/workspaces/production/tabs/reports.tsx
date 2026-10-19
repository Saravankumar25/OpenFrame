// Production › Reports (FSD §58, §110, FSD-PROD-023; UX §3.33; mock 149).
// Read-only projections generated on demand. Nothing persists unless the user
// saves a snapshot.

import { useDeferredValue, useState } from "react";
import { Save, Search, Trash2 } from "lucide-react";
import { Banner, Button, IconButton, PageHeader, Skeleton, TextInput } from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError, useOp } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import { useNav } from "../../../app/stores";
import type { ReportData, ReportSaved, ReportSavedRow } from "../../../api/schedule";
import { REPORT_TABLES, formatDateTime } from "../../../api/schedule";
import { ReportExportButton } from "../../../features/export/buttons";
import "../../call-sheets/shared/schedule-ui.css";

export const tab = { id: "reports", label: "Reports", order: 100 };

const REPORTS = [
  { value: "scene", label: "Scene Report" },
  { value: "location", label: "Location Report" },
  { value: "cast_scene", label: "Cast Scene Report" },
  { value: "prop", label: "Prop Report" },
  { value: "schedule", label: "Schedule Report" },
  { value: "breakdown_completeness", label: "Breakdown Completeness" },
] as const;

function ReportTable({ data }: { data: ReportData }) {
  return (
    <>
      <div className="sm muted" style={{ marginBottom: 6 }}>
        {data.sourceLabel ? `Source: ${data.sourceLabel} · ` : ""}Generated {formatDateTime(data.generatedAt)}
      </div>
      {data.columns.length > 0 && (
        <table className="tbl card">
          <thead>
            <tr>
              {data.columns.map((c) => (
                <th key={c}>{c}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.rows.map((r, i) => (
              <tr key={i}>
                {r.map((cell, j) => (
                  <td key={j} style={{ whiteSpace: "pre-wrap" }}>
                    {cell}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {data.note && <div className="empty" style={{ height: "auto", padding: 20 }}>{data.note}</div>}
    </>
  );
}

export default function ReportsTab() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const [type, setType] = useState<string>("scene");
  const [filter, setFilter] = useState("");
  const deferredFilter = useDeferredValue(filter.trim());
  const savedId = route.params?.reportId;
  const live = useOp<ReportData>("reports.generate", { reportType: type, filter: deferredFilter || null }, REPORT_TABLES, { enabled: !savedId });
  const list = useOp<ReportSavedRow[]>("reports.list", {}, ["production_report"]);
  const saved = useOp<ReportSaved>("reports.get", { id: savedId ?? "" }, ["production_report"], { enabled: !!savedId });
  const save = async () => {
    try {
      const id = await call<string>("reports.save", { reportType: type, filter: filter.trim() || null });
      toast.undoable("Saved a report snapshot");
      go({ workspace: "production", sub: "reports", params: { reportId: id } });
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <div className="of-tab">
      <PageHeader
        title="Reports"
        sub="What concise production information do I need to see or send?"
        actions={
          <>
            {!savedId && (
              <Button icon={<Save size={15} />} onClick={save} disabled={!live.data}>
                Save snapshot
              </Button>
            )}
            <ReportExportButton
              reportType={savedId ? null : type}
              filter={savedId ? null : filter.trim() || null}
              reportId={savedId ?? null}
              title={savedId ? (saved.data?.title ?? "Saved report") : (REPORTS.find((r) => r.value === type)?.label ?? "Report")}
              disabled={savedId ? !saved.data : !live.data}
            />
          </>
        }
      />
      <div className="row" style={{ alignItems: "stretch", gap: 14, flex: 1, minHeight: 0 }}>
        <div className="vpanel of-scroll" role="listbox" aria-label="Reports">
          {REPORTS.map((r) => (
            <button
              key={r.value}
              type="button"
              role="option"
              aria-selected={!savedId && type === r.value}
              className={`vi btn ghost${!savedId && type === r.value ? " on" : ""}`}
              style={{ width: "100%", justifyContent: "flex-start" }}
              onClick={() => {
                setType(r.value);
                if (savedId) go({ workspace: "production", sub: "reports" });
              }}
            >
              {r.label}
            </button>
          ))}
          {list.data && list.data.length > 0 && (
            <>
              <div className="vh">Saved snapshots</div>
              {list.data.map((s) => (
                <div key={s.id} className="row" style={{ gap: 2 }}>
                  <button
                    type="button"
                    className={`vi btn ghost grow${savedId === s.id ? " on" : ""}`}
                    style={{ justifyContent: "flex-start", whiteSpace: "normal", textAlign: "left" }}
                    onClick={() => go({ workspace: "production", sub: "reports", params: { reportId: s.id } })}
                  >
                    <span>
                      {s.title}
                      <br />
                      <span className="xs muted">{formatDateTime(s.generatedAt)}</span>
                    </span>
                  </button>
                  <IconButton
                    label={`Delete ${s.title}`}
                    onClick={async () => {
                      try {
                        await call("reports.delete", { id: s.id });
                        toast.undoable("Deleted a saved report");
                        if (savedId === s.id) go({ workspace: "production", sub: "reports" });
                      } catch (e) {
                        reportError(e);
                      }
                    }}
                  >
                    <Trash2 size={14} />
                  </IconButton>
                </div>
              ))}
            </>
          )}
        </div>
        <div className="grow of-scroll">
          {savedId ? (
            saved.data ? (
              <>
                <Banner tone="info" actions={<Button size="xs" onClick={() => go({ workspace: "production", sub: "reports" })}>Back to live reports</Button>}>
                  Saved snapshot “{saved.data.title}”{saved.data.data.filter ? ` (filtered by “${saved.data.data.filter}”)` : ""}. It does not change when production data changes.
                </Banner>
                <h2 style={{ fontSize: 16, margin: "10px 0 4px" }}>{saved.data.data.title}</h2>
                <ReportTable data={saved.data.data} />
              </>
            ) : saved.error ? (
              <Banner tone="err">{saved.error.message}</Banner>
            ) : (
              <Skeleton h={120} />
            )
          ) : (
            <>
              <div className="row" style={{ gap: 6, marginBottom: 8, maxWidth: 360 }}>
                <Search size={15} aria-hidden />
                <TextInput
                  aria-label="Filter report rows"
                  placeholder="Filter rows (scene, location, name…)"
                  value={filter}
                  maxLength={200}
                  onChange={(e) => setFilter(e.target.value)}
                />
                {filter && (
                  <Button size="xs" variant="ghost" onClick={() => setFilter("")}>
                    Clear
                  </Button>
                )}
              </div>
              {live.data ? (
                <>
                  <h2 style={{ fontSize: 16, margin: "0 0 4px" }}>{live.data.title}</h2>
                  <ReportTable data={live.data} />
                </>
              ) : live.error ? (
                <Banner tone="err">{live.error.message}</Banner>
              ) : (
                <Skeleton h={120} />
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
