// Production › Budget snapshot (FSD §39, §109, FSD-PROD-025/026; UX §3.34; mock 150).
// Advisory only: categories, line items, planned total and contingency. Values
// change only when the user edits them — never automatically.

import { useEffect, useState } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { Banner, Button, Dialog, Field, IconButton, PageHeader, Segmented, Select, Skeleton, TextArea, TextInput } from "../../../design-system";
import { call } from "../../../ipc/client";
import { reportError, useOp } from "../../../ipc/query";
import { toast } from "../../../app/toast";
import type { BudgetDto, BudgetLineDto, BudgetView } from "../../../api/schedule";
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
  const [amount, setAmount] = useState(editing ? minorToInput(line.amount, budget.currency) : "");
  const [notes, setNotes] = useState(editing ? (line.notes ?? "") : "");
  const [error, setError] = useState<string | null>(null);
  const save = async () => {
    const minor = parseMoney(amount, budget.currency);
    if (!description.trim()) return setError("A line needs a description.");
    if (minor === null || Number.isNaN(minor)) return setError("Enter a non-negative amount, e.g. 12,000 or 1500.50.");
    const args = { category, description, amount: minor, notes: notes.trim() || null };
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
      <Field label={`Amount (${budget.currency})`} required htmlFor="of-bl-amt" error={error}>
        <TextInput id="of-bl-amt" inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} onKeyDown={(e) => e.key === "Enter" && save()} />
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

function BudgetTable({ budget, onEdit, readOnly }: { budget: BudgetDto; onEdit?: (l: BudgetLineDto) => void; readOnly?: boolean }) {
  const cats = budget.categories.filter((c) => c.count > 0);
  return (
    <table className="tbl card">
      <thead>
        <tr>
          <th>Category</th>
          <th>Line item</th>
          <th className="num">Amount</th>
          {!readOnly && <th aria-label="Actions" />}
        </tr>
      </thead>
      <tbody>
        {cats.map((c) => (
          <CategoryRows key={c.category} budget={budget} category={c.category} total={c.total} onEdit={onEdit} readOnly={readOnly} />
        ))}
        {cats.length === 0 && (
          <tr>
            <td colSpan={4} className="muted">
              No lines yet. Add a line for each rough cost you expect.
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}

function CategoryRows({ budget, category, total, onEdit, readOnly }: { budget: BudgetDto; category: string; total: number; onEdit?: (l: BudgetLineDto) => void; readOnly?: boolean }) {
  const lines = budget.lines.filter((l) => l.category === category);
  return (
    <>
      {lines.map((l, i) => (
        <tr key={l.id}>
          <td>{i === 0 ? category : ""}</td>
          <td>
            {l.description}
            {l.notes && <div className="xs muted">{l.notes}</div>}
          </td>
          <td className="num">{formatMoney(l.amount, budget.currency)}</td>
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
        <td>{category} total</td>
        <td className="num">{formatMoney(total, budget.currency)}</td>
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
            Entered total <b>{formatMoney(q.data.enteredTotal, q.data.currency)}</b>
            {q.data.plannedTotal !== null && <> · Planned {formatMoney(q.data.plannedTotal, q.data.currency)}</>}
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
        title="Budget snapshot"
        sub="How much are we roughly planning to spend?"
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
              <div className="h4">Planned total</div>
              <div className="big">{b.plannedTotal !== null ? formatMoney(b.plannedTotal, b.currency) : "Not set"}</div>
              <Button size="xs" variant="ghost" onClick={() => setPlan(true)}>
                Edit plan
              </Button>
            </div>
            <div className="card pad">
              <div className="h4">Contingency</div>
              <div className="big">
                {b.contingencyMode === "percent" ? `${b.contingencyValue / 100}% ` : ""}
                {formatMoney(b.contingencyAmount, b.currency)}
              </div>
              <div className="xs muted">{b.contingencyMode === "percent" ? (b.plannedTotal !== null ? "of the planned total" : "of the entered total") : "fixed amount"}</div>
            </div>
            <div className="card pad">
              <div className="h4">Entered so far</div>
              <div className="big">{formatMoney(b.enteredTotal, b.currency)}</div>
              {b.plannedTotal !== null && (
                <div className={`xs ${b.enteredTotal > b.plannedTotal ? "" : "muted"}`} style={b.enteredTotal > b.plannedTotal ? { color: "var(--red)", fontWeight: 600 } : undefined}>
                  {b.enteredTotal > b.plannedTotal
                    ? `${formatMoney(b.enteredTotal - b.plannedTotal, b.currency)} over the plan`
                    : `${formatMoney(b.plannedTotal - b.enteredTotal, b.currency)} left in the plan`}
                </div>
              )}
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
              <span className="sm">{formatMoney(s.enteredTotal, s.currency)}</span>
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
