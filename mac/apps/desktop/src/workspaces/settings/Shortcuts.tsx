// Settings > Keyboard shortcuts (SCRIPT-019; Screenplay and Production Depth Specification §20).
//
// Table: Command | Default | Current | Conflict. Change a shortcut by pressing the new keys,
// remove one, add several per command, restore one command or everything. A key that another
// command already uses is never taken silently: the screen asks before moving it. Every command
// also names its on-screen route, so nothing depends on the keyboard. The person's changes live
// on this computer (application database) and apply in every project.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { Plus, RotateCcw, X } from "lucide-react";
import { Banner, Button, ConfirmDialog, PageHeader, TextInput } from "../../design-system";
import { call, OpError } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ShortcutOverrides } from "../../ipc/generated/ShortcutOverrides";
import { toast } from "../../app/toast";
import "./writing.css";
import {
  MAX_KEYS_PER_COMMAND,
  SHORTCUT_COMMANDS,
  bindingsFor,
  checkChord,
  chordFromEvent,
  commandById,
  displayChord,
  findConflicts,
  toOverrides,
  useShortcuts,
  whoUses,
  type ShortcutCommand,
} from "../../app/shortcuts";

type Change = { commandId: string; keys: string[] | null };

interface Pending {
  title: string;
  body: string;
  confirm: string;
  changes: Change[];
}

const label = (id: string) => commandById(id)?.label ?? id;

/** `embedded`: shown inside the Home dialog (the dialog has the title). `onRecordingChange` lets
 * that dialog keep Escape for the key recorder while it waits for keys. */
export function ShortcutsSettings({ embedded = false, onRecordingChange }: { embedded?: boolean; onRecordingChange?: (recording: boolean) => void } = {}) {
  const overrides = useShortcuts((s) => s.overrides);
  const [filter, setFilter] = useState("");
  const [recording, setRecordingState] = useState<string | null>(null);
  // The dialog around this table learns about recording in the same update that shows the
  // recorder, so Escape pressed as soon as the recorder appears cancels the recording and never
  // closes the dialog.
  const setRecording = (id: string | null) => {
    setRecordingState(id);
    onRecordingChange?.(id !== null);
  };
  // Closed while recording: the dialog must not stay unable to close the next time.
  const reportRecording = useRef(onRecordingChange);
  reportRecording.current = onRecordingChange;
  useEffect(() => () => reportRecording.current?.(false), []);
  const [error, setError] = useState<{ id: string; text: string } | null>(null);
  const [pending, setPending] = useState<Pending | null>(null);
  const [resetAll, setResetAll] = useState(false);
  const [busy, setBusy] = useState(false);
  // After recording, removing or restoring, focus returns to the row's Add button.
  const addRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const [focusId, setFocusId] = useState<string | null>(null);
  useEffect(() => {
    if (!focusId || busy || recording) return;
    const el = addRefs.current[focusId];
    if (el && !el.disabled) el.focus();
    setFocusId(null);
  }, [focusId, busy, recording]);

  const conflicts = useMemo(() => findConflicts(overrides), [overrides]);
  const q = filter.trim().toLowerCase();
  const shown = SHORTCUT_COMMANDS.filter(
    (c) => !q || c.label.toLowerCase().includes(q) || c.group.toLowerCase().includes(q) || bindingsFor(c.id, overrides).some((k) => displayChord(k).toLowerCase().includes(q)),
  );
  const groups = [...new Set(shown.map((c) => c.group))];

  const save = async (changes: Change[], done: string) => {
    setBusy(true);
    try {
      const r = await call<ShortcutOverrides>("app.set_shortcuts", { changes });
      useShortcuts.getState().set(toOverrides(r));
      toast.success(done);
      setError(null);
    } catch (e) {
      if (e instanceof OpError && e.code.startsWith("validation")) setError({ id: changes[0]?.commandId ?? "", text: e.message });
      else reportError(e);
    } finally {
      setBusy(false);
    }
  };

  /** Without `chord` from these commands (the conflict resolution). */
  const takeFrom = (ids: string[], chord: string): Change[] =>
    ids.map((o) => ({ commandId: o, keys: bindingsFor(o, overrides).filter((k) => k !== chord) }));

  const addKey = (c: ShortcutCommand, chord: string) => {
    const current = bindingsFor(c.id, overrides);
    if (current.includes(chord)) {
      setError({ id: c.id, text: `${displayChord(chord)} is already a shortcut for ${c.label}.` });
      return;
    }
    const next: Change = { commandId: c.id, keys: [...current, chord] };
    const others = whoUses(chord, c.scope, overrides, c.id);
    if (others.length === 0) {
      void save([next], `${c.label}: ${displayChord(chord)}`);
      return;
    }
    setPending({
      title: `${displayChord(chord)} is already used`,
      body: `${displayChord(chord)} runs ${others.map(label).join(" and ")}. Move it to ${c.label}? ${others.map(label).join(" and ")} will keep any other shortcuts and its on-screen route.`,
      confirm: "Move it here",
      changes: [next, ...takeFrom(others, chord)],
    });
  };

  const removeKey = (c: ShortcutCommand, chord: string) => {
    setFocusId(c.id);
    void save([{ commandId: c.id, keys: bindingsFor(c.id, overrides).filter((k) => k !== chord) }], `Removed ${displayChord(chord)} from ${c.label}`);
  };

  const restore = (c: ShortcutCommand) => {
    const without = { ...overrides };
    delete without[c.id];
    const clashes = c.defaults.flatMap((k) => whoUses(k, c.scope, without, c.id).map((o) => ({ k, o })));
    const change: Change = { commandId: c.id, keys: null };
    if (clashes.length === 0) {
      void save([change], `Restored the default for ${c.label}`);
      return;
    }
    setPending({
      title: "Restore the default?",
      body: clashes.map(({ k, o }) => `${displayChord(k)} is now used by ${label(o)}.`).join(" ") + ` Restoring gives it back to ${c.label}.`,
      confirm: "Restore and move",
      changes: [change, ...clashes.flatMap(({ k, o }) => takeFrom([o], k))],
    });
  };

  const onRecordKey = (c: ShortcutCommand, e: ReactKeyboardEvent<HTMLButtonElement>) => {
    const plain = !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey;
    if (e.key === "Tab" && !e.ctrlKey && !e.metaKey && !e.altKey) {
      setRecording(null);
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape" && plain) {
      setRecording(null);
      setError(null);
      setFocusId(c.id);
      return;
    }
    const raw = chordFromEvent(e.nativeEvent);
    if (!raw) return; // a modifier on its own: keep listening
    const r = checkChord(raw);
    if ("error" in r) {
      setError({ id: c.id, text: `${displayChord(raw)}: ${r.error}` });
      return;
    }
    setRecording(null);
    setFocusId(c.id);
    addKey(c, r.chord);
  };

  const conflictOf = (c: ShortcutCommand) => {
    const others = new Set(conflicts.filter((x) => x.commandId === c.id).map((x) => x.otherId));
    return [...others].map(label);
  };

  const restoreAll = (
    <Button icon={<RotateCcw size={14} />} disabled={Object.keys(overrides).length === 0 || busy} onClick={() => setResetAll(true)}>
      Restore all defaults
    </Button>
  );

  return (
    <div className={embedded ? undefined : "content"} style={embedded ? undefined : { overflow: "auto" }}>
      {embedded ? (
        <div className="row" style={{ justifyContent: "flex-end" }}>{restoreAll}</div>
      ) : (
        <PageHeader
          title="Keyboard shortcuts"
          sub="Change, remove or add keys. Every command also has a button or menu on screen. Your shortcuts apply in every project on this computer."
          actions={restoreAll}
        />
      )}
      {conflicts.length > 0 && (
        <Banner tone="warn">Some shortcuts are used by two commands. Change one of them below; until then the first command in the list wins.</Banner>
      )}
      <div className="row" style={{ margin: "8px 0 12px" }}>
        <TextInput aria-label="Filter commands" placeholder="Filter commands or keys" value={filter} onChange={(e) => setFilter(e.target.value)} style={{ maxWidth: 320 }} />
        <span className="xs muted" aria-live="polite">{shown.length} commands</span>
      </div>
      <table className="tbl sc-table">
        <thead>
          <tr>
            <th scope="col">Command</th>
            <th scope="col">Default</th>
            <th scope="col">Current</th>
            <th scope="col">Conflict</th>
            <th scope="col"><span className="sr-only">Actions</span></th>
          </tr>
        </thead>
        {groups.map((g) => (
          <tbody key={g}>
              <tr className="sc-group">
                <th scope="rowgroup" colSpan={5} className="h4">{g}</th>
              </tr>
              {shown
                .filter((c) => c.group === g)
                .map((c) => {
                  const current = bindingsFor(c.id, overrides);
                  const changed = c.id in overrides;
                  const clash = conflictOf(c);
                  return (
                    <tr key={c.id} data-command={c.id}>
                      <th scope="row" style={{ fontWeight: 500, textAlign: "left" }}>
                        {c.label}
                        <div className="xs muted">On screen: {c.route}</div>
                      </th>
                      <td className="sm muted">{c.defaults.length ? c.defaults.map((k) => displayChord(k)).join(", ") : "None"}</td>
                      <td>
                        <div className="row" style={{ flexWrap: "wrap", gap: 4 }}>
                          {current.length === 0 && recording !== c.id && <span className="sm muted">None</span>}
                          {current.map((k) => (
                            <span key={k} className="chip out sc-key">
                              <kbd>{displayChord(k)}</kbd>
                              <button type="button" className="sc-x" aria-label={`Remove ${displayChord(k)} from ${c.label}`} disabled={busy} onClick={() => removeKey(c, k)}>
                                <X size={11} aria-hidden />
                              </button>
                            </span>
                          ))}
                          {recording === c.id && (
                            <RecordButton label={c.label} onKey={(e) => onRecordKey(c, e)} onBlur={() => setRecording(null)} />
                          )}
                          {changed && <span className="xs muted">(changed)</span>}
                        </div>
                        {error?.id === c.id && (
                          <div role="alert" className="xs errtxt" style={{ marginTop: 4 }}>
                            {error.text}
                          </div>
                        )}
                      </td>
                      <td className="sm">{clash.length ? `Also used by ${clash.join(", ")}` : <span className="muted">None</span>}</td>
                      <td>
                        <div className="row" style={{ gap: 4, justifyContent: "flex-end" }}>
                          <Button
                            ref={(el) => {
                              addRefs.current[c.id] = el;
                            }}
                            size="sm"
                            icon={<Plus size={13} />}
                            disabled={busy || current.length >= MAX_KEYS_PER_COMMAND || recording === c.id}
                            aria-label={`Add a shortcut for ${c.label}`}
                            onClick={() => {
                              setError(null);
                              setRecording(c.id);
                            }}
                          >
                            Add
                          </Button>
                          <Button
                            size="sm"
                            disabled={busy || !changed}
                            aria-label={`Restore the default for ${c.label}`}
                            onClick={() => {
                              setFocusId(c.id);
                              restore(c);
                            }}
                          >
                            Restore default
                          </Button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
          </tbody>
        ))}
      </table>
      {shown.length === 0 && <p className="sm muted">No command matches “{filter}”.</p>}

      <ConfirmDialog
        open={!!pending}
        onOpenChange={(v) => !v && setPending(null)}
        title={pending?.title ?? ""}
        confirmLabel={pending?.confirm ?? "Continue"}
        busy={busy}
        onConfirm={() => {
          const p = pending;
          setPending(null);
          if (p) void save(p.changes, "Shortcuts updated");
        }}
      >
        <p>{pending?.body}</p>
      </ConfirmDialog>
      <ConfirmDialog
        open={resetAll}
        onOpenChange={setResetAll}
        title="Restore all keyboard shortcuts?"
        confirmLabel="Restore all defaults"
        danger
        busy={busy}
        onConfirm={() => {
          setResetAll(false);
          setBusy(true);
          void call<ShortcutOverrides>("app.reset_shortcuts", {})
            .then((r) => {
              useShortcuts.getState().set(toOverrides(r));
              toast.success("All shortcuts are back to their defaults");
            })
            .catch(reportError)
            .finally(() => setBusy(false));
        }}
      >
        <p>Every shortcut you changed goes back to its default. This can't be undone, but you can change them again at any time.</p>
      </ConfirmDialog>
    </div>
  );
}

/** Focused key catcher: the next key combination becomes the new shortcut. */
function RecordButton({ label, onKey, onBlur }: { label: string; onKey: (e: ReactKeyboardEvent<HTMLButtonElement>) => void; onBlur: () => void }) {
  const ref = useRef<HTMLButtonElement | null>(null);
  // Focused before the first paint: the keys pressed once it shows go to it.
  useLayoutEffect(() => {
    ref.current?.focus();
  }, []);
  return (
    <button
      ref={ref}
      type="button"
      className="chip sc-record"
      aria-label={`Press the new keys for ${label}. Escape cancels.`}
      onKeyDown={onKey}
      onBlur={onBlur}
    >
      Press the new keys… (Esc cancels)
    </button>
  );
}
