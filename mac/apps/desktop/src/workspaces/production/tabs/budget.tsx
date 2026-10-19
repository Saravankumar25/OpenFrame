// Production › Budget (FSD §39, §109, FSD-PROD-025/026; production depth spec §33; UX §3.34; mock 150).
// Advisory only: categories, lines with a Planned and an Actual amount, category totals, the
// difference, a planned total and contingency. Values change only when the user edits them,
// never automatically.

import { useEffect, useState } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { Banner, Button, Dialog, Field, IconButton, PageHeader, Segmented, Select, Skeleton, TextArea, TextInput } from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError, useOp } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import type { BudgetCategoryTotal, BudgetDto, BudgetLineDto, BudgetView } from "../../../api/schedule";
import { BUDGET_TABLES, formatDateTime, formatMoney, minorToInput, parseMoney } from "../../../api/schedule";
import { BudgetExportButton } from "../../../features/export/buttons";
import "../../call-sheets/shared/schedule-ui.css";

export const tab = { id: "budget", label: "Budget", order: 110 };

const CURRENCIES = ["USD", "EUR", "GBP", "INR", "CAD", "AUD", "NZD", "JPY", "CHF", "SEK", "NOK", "DKK", "ZAR", "BRL", "MXN", "SGD", "AED"];

async function run(op: string, args: object, text: string): Promise<boolean> {
  try {
    await call(op, args);
    toast.undoable(text);
    return true;
  } catch (e) {
    reportError(e);
    return false;
  }
}

function LineDialog({ budget, line, categories, onClose }: { budget: BudgetDto; line: BudgetLineDto | "new"; categories: string[]; onClose: () => void }) {
  const editing = line !== "new";
  const [category, setCategory] = useState(editing ? line.category : categories[0]);
  const [description, setDescription] = useState(editing ? line.description : "");
  const [planned, setPlanned] = useState(editing ? minorToInput(line.planned, budget.currency) : "");
  const [actual, setActual] = useState(editing && line.actual !== null ? minorToInput(line.actual, budget.currency) : "");
  const [notes, setNotes] = useState(editing ? (line.notes ?? "") : "");
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    const plannedMinor = parseMoney(planned, budget.currency);
    const actualMinor = parseMoney(actual, budget.currency);
    if (!description.trim()) return setError("A line needs a description.");
    if (plannedMinor === null || Number.isNaN(plannedMinor)) return setError("Enter the planned amount as a non-negative number, for example 12,000 or 1500.50.");
    if (actualMinor !== null && Number.isNaN(actualMinor)) return setError("Enter the actual amount as a non-negative number, or leave it empty.");
    const args = { category, description, planned: plannedMinor, actual: actualMinor, notes: notes.trim() || null };
    const ok = editing
      ? await run("budget.update_line", { id: line.id, ...args, expectedRev: line.rev }, `Edited “${description.trim()}”`)
      : await run("budget.add_line", { budgetId: budget.id, ...args }, `Added “${description.trim()}”`);
    if (ok) onClose();
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={editing ? "Edit line" : "Add Line"}
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={save}>
            {editing ? "Save" : "Add Line"}
          </Button>
        </>
      }
    >
      <Field label="Category" htmlFor="of-bl-cat">
        <Select id="of-bl-cat" value={category} onChange={setCategory} options={categories.map((c) => ({ value: c, label: c }))} />
      </Field>
      <Field label="Line item" required htmlFor="of-bl-desc">
        <TextInput id="of-bl-desc" value={description} onChange={(e) => setDescription(e.target.value)} maxLength={200} autoFocus />
      </Field>
      <Field label={`Planned (${budget.currency})`} required htmlFor="of-bl-amt">
        <TextInput id="of-bl-amt" inputMode="decimal" value={planned} onChange={(e) => setPlanned(e.target.value)} onKeyDown={(e) => e.key === "Enter" && save()} />
      </Field>
      <Field label={`Actual (${budget.currency})`} hint="Leave empty until the money is spent." htmlFor="of-bl-act" error={error}>
        <TextInput id="of-bl-act" inputMode="decimal" value={actual} onChange={(e) => setActual(e.target.value)} onKeyDown={(e) => e.key === "Enter" && save()} />
      </Field>
      <Field label="Notes" htmlFor="of-bl-notes">
        <TextArea id="of-bl-notes" value={notes} onChange={(e) => setNotes(e.target.value)} rows={2} />
      </Field>
    </Dialog>
  );
}

function PlanDialog({ budget, onClose }: { budget: BudgetDto; onClose: () => void }) {
  const [currency, setCurrency] = useState(budget.currency);
  const [planned, setPlanned] = useState(minorToInput(budget.plannedTotal, budget.currency));
  const [mode, setMode] = useState<"percent" | "amount">(budget.contingencyMode === "amount" ? "amount" : "percent");
  const [value, setValue] = useState(
    budget.contingencyMode === "amount" ? minorToInput(budget.contingencyValue, budget.currency) : String(budget.contingencyValue / 100),
  );
  const [notes, setNotes] = useState(budget.notes ?? "");
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    const p = parseMoney(planned, currency);
    if (Number.isNaN(p)) return setError("Enter the planned total as a non-negative amount.");
    let cv: number;
    if (mode === "percent") {
      const pct = value.trim() === "" ? 0 : Number(value.replace("%", ""));
      if (!Number.isFinite(pct) || pct < 0 || pct > 100) return setError("Contingency must be between 0% and 100%.");
      cv = Math.round(pct * 100);
    } else {
      const a = parseMoney(value, currency);
      if (a === null) cv = 0;
      else if (Number.isNaN(a)) return setError("Enter the contingency as a non-negative amount.");
      else cv = a;
    }
    const ok = await run(
      "budget.update",
      { id: budget.id, currency, plannedTotal: p, contingencyMode: mode, contingencyValue: cv, notes: notes.trim() || null, expectedRev: budget.rev },
      "Changed the budget plan",
    );
    if (ok) onClose();
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Budget plan"
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={save}>
            Save
          </Button>
        </>
      }
    >
      <Field label="Currency" htmlFor="of-b-cur">
        <Select id="of-b-cur" value={currency} onChange={setCurrency} options={CURRENCIES.map((c) => ({ value: c, label: c }))} />
      </Field>
      <Field label="Planned total" hint="Your overall estimate. Optional." htmlFor="of-b-plan">
        <TextInput id="of-b-plan" inputMode="decimal" value={planned} onChange={(e) => setPlanned(e.target.value)} />
      </Field>
      <Field label="Contingency">
        <div className="row">
          <Segmented ariaLabel="Contingency type" value={mode} onChange={setMode} options={[{ value: "percent", label: "Percent" }, { value: "amount", label: "Amount" }]} />
          <TextInput aria-label="Contingency value" inputMode="decimal" value={value} onChange={(e) => setValue(e.target.value)} style={{ maxWidth: 140 }} />
          {mode === "percent" && <span className="sm muted">%</span>}
        </div>
      </Field>
      <Field label="Notes" htmlFor="of-b-notes" error={error}>
        <TextArea id="of-b-notes" value={notes} onChange={(e) => setNotes(e.target.value)} rows={2} />
      </Field>
    </Dialog>
  );
}

/** "under plan" or "over plan" text for a Planned minus Actual difference (no meaning by colour alone). */
function differenceText(difference: number, currency: string): string {
  if (difference === 0) return "On plan";
  return difference > 0 ? `${formatMoney(difference, currency)} under plan` : `${formatMoney(-difference, currency)} over plan`;
}

function Difference({ value, currency }: { value: number | null; currency: string }) {
  if (value === null) return <span className="muted">Not spent yet</span>;
  return <span className={value < 0 ? "of-over" : undefined}>{differenceText(value, currency)}</span>;
}

function BudgetTable({ budget, onEdit, readOnly }: { budget: BudgetDto; onEdit?: (l: BudgetLineDto) => void; readOnly?: boolean }) {
  const cats = budget.categories.filter((c) => c.count > 0);
  const cols = readOnly ? 5 : 6;
  return (
    <table className="tbl card" aria-label="Budget lines">
      <thead>
        <tr>
          <th scope="col">Category</th>
          <th scope="col">Line item</th>
          <th scope="col" className="num">Planned</th>
          <th scope="col" className="num">Actual</th>
          <th scope="col" className="num">Difference</th>
          {!readOnly && <th scope="col" aria-label="Actions" />}
        </tr>
      </thead>
      <tbody>
        {cats.map((c) => (
          <CategoryRows key={c.category} budget={budget} total={c} onEdit={onEdit} readOnly={readOnly} />
        ))}
        {cats.length === 0 && (
          <tr>
            <td colSpan={cols} className="muted">
              No lines yet. Add a line for each rough cost you expect.
            </td>
          </tr>
        )}
      </tbody>
      {cats.length > 0 && (
        <tfoot>
          <tr className="of-sub">
            <td />
            <td>All lines</td>
            <td className="num">{formatMoney(budget.plannedLinesTotal, budget.currency)}</td>
            <td className="num">{budget.actualCount > 0 ? formatMoney(budget.actualTotal, budget.currency) : "None yet"}</td>
            <td className="num">
              <Difference value={budget.actualCount > 0 ? budget.difference : null} currency={budget.currency} />
            </td>
            {!readOnly && <td />}
          </tr>
        </tfoot>
      )}
    </table>
  );
}

function CategoryRows({ budget, total, onEdit, readOnly }: { budget: BudgetDto; total: BudgetCategoryTotal; onEdit?: (l: BudgetLineDto) => void; readOnly?: boolean }) {
  const lines = budget.lines.filter((l) => l.category === total.category);
  return (
    <>
      {lines.map((l, i) => (
        <tr key={l.id}>
          <td>{i === 0 ? total.category : ""}</td>
          <td>
            {l.description}
            {l.notes && <div className="xs muted">{l.notes}</div>}
          </td>
          <td className="num">{formatMoney(l.planned, budget.currency)}</td>
          <td className="num">{l.actual !== null ? formatMoney(l.actual, budget.currency) : <span className="muted">Not entered</span>}</td>
          <td className="num">
            <Difference value={l.difference} currency={budget.currency} />
          </td>
          {!readOnly && (
            <td className="num" style={{ whiteSpace: "nowrap" }}>
              <IconButton label={`Edit ${l.description}`} onClick={() => onEdit?.(l)}>
                <Pencil size={14} />
              </IconButton>
              <IconButton label={`Delete ${l.description}`} onClick={() => run("budget.delete_line", { id: l.id }, `Deleted “${l.description}”`)}>
                <Trash2 size={14} />
              </IconButton>
            </td>
          )}
        </tr>
      ))}
      <tr className="of-sub">
        <td />
        <td>{total.category} total</td>
        <td className="num">{formatMoney(total.planned, budget.currency)}</td>
        <td className="num">{total.actualCount > 0 ? formatMoney(total.actual, budget.currency) : ""}</td>
        <td className="num">{total.actualCount > 0 && <Difference value={total.difference} currency={budget.currency} />}</td>
        {!readOnly && <td />}
      </tr>
    </>
  );
}

function SnapshotDialog({ id, onClose }: { id: string; onClose: () => void }) {
  const q = useOp<BudgetDto>("budget.get_snapshot", { id }, BUDGET_TABLES);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={q.data?.label ?? "Budget snapshot"}
      sub={q.data?.frozenAt ? `Saved ${formatDateTime(q.data.frozenAt)} · read-only` : undefined}
      size="lg"
      footer={
        <>
          <BudgetExportButton budgetId={id} />
          <Button onClick={onClose}>Close</Button>
        </>
      }
    >
      {q.data ? (
        <>
          <div className="sm" style={{ marginBottom: 6 }}>
            Planned <b>{formatMoney(q.data.plannedLinesTotal, q.data.currency)}</b> · Actual{" "}
            <b>{q.data.actualCount > 0 ? formatMoney(q.data.actualTotal, q.data.currency) : "none yet"}</b>
            {q.data.plannedTotal !== null && <> · Overall plan {formatMoney(q.data.plannedTotal, q.data.currency)}</>}
          </div>
          <BudgetTable budget={q.data} readOnly />
        </>
      ) : (
        <Skeleton h={120} />
      )}
    </Dialog>
  );
}

export default function BudgetTab() {
  const q = useOp<BudgetView>("budget.get", {}, BUDGET_TABLES);
  const [currency, setCurrency] = useState("USD");
  const [line, setLine] = useState<BudgetLineDto | "new" | null>(null);
  const [plan, setPlan] = useState(false);
  const [snapName, setSnapName] = useState<string | null>(null);
  const [viewSnap, setViewSnap] = useState<string | null>(null);
  useEffect(() => {
    try {
      const guess = new Intl.NumberFormat().resolvedOptions().locale;
      if (guess.endsWith("-IN")) setCurrency("INR");
      else if (guess.endsWith("-GB")) setCurrency("GBP");
    } catch {
      /* keep USD */
    }
  }, []);
  if (q.isLoading || !q.data) return <div>{q.error ? <Banner tone="err">{q.error.message}</Banner> : <Skeleton h={28} w={240} />}</div>;
  const v = q.data;
  const b = v.current;
  return (
    <div>
      <PageHeader
        title="Budget"
        sub="What did we plan to spend, and what have we spent?"
        actions={
          b && (
            <>
              <BudgetExportButton />
              <Button onClick={() => setSnapName("")}>Save snapshot</Button>
              <Button variant="primary" icon={<Plus size={15} />} onClick={() => setLine("new")}>
                Add Line
              </Button>
            </>
          )
        }
      />
      <div style={{ marginBottom: 10 }}>
        <Banner tone="info">Use a simple estimate for planning. Formal accounting is outside OpenFrame.</Banner>
      </div>
      {b && v.reviewReminder && (
        <div style={{ marginBottom: 10 }}>
          <Banner
            tone="warn"
            actions={
              <Button size="xs" onClick={() => run("budget.mark_reviewed", { id: b.id }, "Marked the budget as reviewed")}>
                Mark reviewed
              </Button>
            }
          >
            <b>Review budget.</b> {v.reviewReminder}
          </Banner>
        </div>
      )}
      {!b ? (
        <div className="card pad" style={{ maxWidth: 440 }}>
          <div className="h4">Start a budget</div>
          <Field label="Currency" htmlFor="of-b-start-cur">
            <Select id="of-b-start-cur" value={currency} onChange={setCurrency} options={CURRENCIES.map((c) => ({ value: c, label: c }))} />
          </Field>
          <Button variant="primary" onClick={() => run("budget.create", { currency }, "Started a budget snapshot")}>
            Start budget
          </Button>
          <div className="hint" style={{ marginTop: 6 }}>Optional. You can ignore the budget entirely.</div>
        </div>
      ) : (
        <>
          <div className="grid g3 of-tiles" style={{ marginBottom: 12 }}>
            <div className="card pad">
              <div className="h4">Planned</div>
              <div className="big">{formatMoney(b.plannedLinesTotal, b.currency)}</div>
              <div className="xs muted">
                {b.plannedTotal !== null ? `Overall plan ${formatMoney(b.plannedTotal, b.currency)}` : "Sum of the Planned amounts"}
                {" · "}
                {b.contingencyMode === "percent" ? `${b.contingencyValue / 100}% contingency ` : "Contingency "}
                {formatMoney(b.contingencyAmount, b.currency)}
              </div>
              <Button size="xs" variant="ghost" onClick={() => setPlan(true)}>
                Edit plan
              </Button>
            </div>
            <div className="card pad">
              <div className="h4">Actual</div>
              <div className="big">{b.actualCount > 0 ? formatMoney(b.actualTotal, b.currency) : "None yet"}</div>
              <div className="xs muted">
                {b.actualCount} of {b.lines.length} {b.lines.length === 1 ? "line has" : "lines have"} an Actual amount
              </div>
            </div>
            <div className="card pad">
              <div className="h4">Difference</div>
              <div className="big">{b.actualCount > 0 ? <Difference value={b.difference} currency={b.currency} /> : "Not spent yet"}</div>
              <div className="xs muted">Planned minus Actual, over the lines with an Actual amount</div>
            </div>
          </div>
          <BudgetTable budget={b} onEdit={(l) => setLine(l)} />
          {b.notes && <div className="sm" style={{ marginTop: 8, whiteSpace: "pre-wrap" }}>{b.notes}</div>}
          <div className="hint" style={{ marginTop: 8 }}>
            Budget values never change automatically. If the script or schedule changes, review the budget yourself.
          </div>
        </>
      )}
      {v.snapshots.length > 0 && (
        <div className="card pad" style={{ marginTop: 12, maxWidth: 560 }}>
          <div className="h4">Saved snapshots</div>
          {v.snapshots.map((s) => (
            <div key={s.id} className="li">
              <span className="grow">
                {s.label ?? "Snapshot"} <span className="xs muted">{s.frozenAt ? formatDateTime(s.frozenAt) : ""}</span>
              </span>
              <span className="sm">
                Planned {formatMoney(s.plannedLinesTotal, s.currency)} · Actual {formatMoney(s.actualTotal, s.currency)}
              </span>
              <Button size="xs" onClick={() => setViewSnap(s.id)}>
                View
              </Button>
              <IconButton label={`Delete snapshot ${s.label ?? ""}`.trim()} onClick={() => run("budget.delete_snapshot", { id: s.id }, `Deleted budget snapshot “${s.label ?? "Snapshot"}”`)}>
                <Trash2 size={14} />
              </IconButton>
            </div>
          ))}
        </div>
      )}
      {b && line && <LineDialog budget={b} line={line} categories={v.categories} onClose={() => setLine(null)} />}
      {b && plan && <PlanDialog budget={b} onClose={() => setPlan(false)} />}
      {viewSnap && <SnapshotDialog id={viewSnap} onClose={() => setViewSnap(null)} />}
      {b && snapName !== null && (
        <Dialog
          open
          onOpenChange={(open) => !open && setSnapName(null)}
          title="Save budget snapshot"
          size="sm"
          footer={
            <>
              <Button onClick={() => setSnapName(null)}>Cancel</Button>
              <Button
                variant="primary"
                disabled={!snapName.trim()}
                onClick={async () => {
                  if (await run("budget.save_snapshot", { id: b.id, label: snapName }, `Saved budget snapshot “${snapName.trim()}”`)) setSnapName(null);
                }}
              >
                Save snapshot
              </Button>
            </>
          }
        >
          <Field label="Snapshot name" required htmlFor="of-b-snap" hint="A frozen copy you can look back at later.">
            <TextInput id="of-b-snap" value={snapName} onChange={(e) => setSnapName(e.target.value)} placeholder="Pre-production estimate" autoFocus />
          </Field>
        </Dialog>
      )}
    </div>
  );
}
