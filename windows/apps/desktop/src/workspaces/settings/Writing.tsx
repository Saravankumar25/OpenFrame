// Settings > Writing (SCRIPT-020; Screenplay and Production Depth Specification §19).
//
// Proofing for this project: the on and off switch, the main language and a second language for
// mixed-language scripts, chosen from the spelling dictionaries installed on this computer.
// Each change is one undoable project step. The personal dictionary belongs to the person at
// this computer and applies in every project. Nothing is downloaded and no text leaves the
// computer: the operating system's own dictionaries do the checking.

import { useEffect, useMemo, useState } from "react";
import { Plus, X } from "lucide-react";
import { Banner, Button, Checkbox, Field, PageHeader, Select, Skeleton, TextInput } from "../../design-system";
import { call, OpError, onAppEvent } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { WritingProofingState } from "../../ipc/generated/WritingProofingState";
import type { ProofingSettings } from "../../ipc/generated/ProofingSettings";
import type { WritingDictionary } from "../../ipc/generated/WritingDictionary";
import { toast } from "../../app/toast";
import "./writing.css";

const NONE = "";

/** "en-US" → "English (United States)"; unknown tags are shown as they are. */
export function languageName(tag: string): string {
  try {
    const names = new Intl.DisplayNames(["en"], { type: "language" });
    return names.of(tag) ?? tag;
  } catch {
    return tag;
  }
}

function languageOptions(installed: string[], current: string[]): { value: string; label: string }[] {
  const all = [...installed];
  for (const c of current) if (c && !all.includes(c)) all.push(c);
  return all
    .map((t) => ({ value: t, label: installed.includes(t) ? languageName(t) : `${languageName(t)}, not installed on this computer` }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

export function WritingSettings() {
  const [state, setState] = useState<WritingProofingState | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = () =>
    call<WritingProofingState>("writing.proofing", {})
      .then((s) => {
        setState(s);
        setLoadError(null);
      })
      .catch((e: unknown) => setLoadError(e instanceof OpError ? e.message : "Proofing settings could not be shown right now."));

  useEffect(() => {
    void load();
    // Undo and redo of a proofing change, or a change in another window.
    const un = onAppEvent((ev) => {
      if (ev.type === "dataChanged" && ev.tables.includes("project")) void load();
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const update = async (patch: { enabled?: boolean; mainLanguage?: string; secondLanguage?: string }, done: string) => {
    setBusy(true);
    try {
      const settings = await call<ProofingSettings>("writing.set_proofing", patch);
      setState((s) => (s ? { ...s, settings } : s));
      toast.undoable(done);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  const s = state?.settings;
  const installed = state?.installed ?? [];
  const options = useMemo(
    () => (s ? languageOptions(installed, [s.mainLanguage, s.secondLanguage ?? ""]) : []),
    // The joined list stands in for the array, which is a new object on every load.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [installed.join(","), s?.mainLanguage, s?.secondLanguage],
  );
  const canEdit = !!state?.canEdit && !busy;
  const missing = s ? [s.mainLanguage, s.secondLanguage].filter((l): l is string => !!l && !installed.includes(l)) : [];

  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader title="Writing" sub="Spell checking for this project and your personal dictionary." />
      <section className="card pad" style={{ maxWidth: 720 }} aria-label="Spelling">
        <div className="h4">Spelling</div>
        {loadError && <Banner tone="err">{loadError}</Banner>}
        {!state && !loadError && <Skeleton h={120} />}
        {state && s && (
          <>
            <Checkbox
              checked={s.enabled}
              disabled={!canEdit}
              label="Check spelling as I write"
              onChange={(v) => void update({ enabled: v }, v ? "Turned spell checking on" : "Turned spell checking off")}
            />
            {state.serviceError && (
              <Banner tone="warn">Spell checking is not available right now. Your writing is not affected; try again in a moment.</Banner>
            )}
            {!state.adapterAvailable && (
              <Banner tone="info">On this computer the page uses the system's own spell checking. The language and personal dictionary settings below apply once OpenFrame can read this system's dictionaries.</Banner>
            )}
            {state.adapterAvailable && !state.serviceError && installed.length === 0 && (
              <Banner tone="warn">No spelling dictionaries are installed on this computer. Add a language in Windows Settings, then come back here.</Banner>
            )}
            {missing.length > 0 && installed.length > 0 && (
              <Banner tone="warn">{missing.map(languageName).join(" and ")} {missing.length === 1 ? "is" : "are"} not installed on this computer, so words in {missing.length === 1 ? "that language" : "those languages"} are not underlined here.</Banner>
            )}
            <div className="row" style={{ gap: 16, alignItems: "flex-start", marginTop: 10 }}>
              <Field label="Main language" htmlFor="proof-main">
                {options.length === 0 ? (
                  <span className="sm muted">{languageName(s.mainLanguage)}</span>
                ) : (
                  <Select id="proof-main" value={s.mainLanguage} options={options} onChange={(v) => canEdit && void update({ mainLanguage: v }, `Main spelling language: ${languageName(v)}`)} />
                )}
              </Field>
              <Field label="Second language" htmlFor="proof-second" hint="For scripts that mix two languages">
                <Select
                  id="proof-second"
                  value={s.secondLanguage ?? NONE}
                  options={[{ value: NONE, label: "None" }, ...options.filter((o) => o.value !== s.mainLanguage)]}
                  onChange={(v) => canEdit && void update({ secondLanguage: v }, v ? `Second spelling language: ${languageName(v)}` : "Removed the second spelling language")}
                />
              </Field>
            </div>
            <p className="xs muted">
              Words in a writing system with no installed dictionary, for example Telugu or Hindi on most computers, are never underlined. Character names in the script are never underlined.
              {installed.length > 0 && ` Dictionaries on this computer: ${installed.map(languageName).join(", ")}.`}
            </p>
            {!state.canEdit && <p className="xs muted">Your role lets you read this project, so these settings can't be changed here.</p>}
          </>
        )}
      </section>
      <PersonalDictionary />
    </div>
  );
}

function PersonalDictionary() {
  const [words, setWords] = useState<string[] | null>(null);
  const [word, setWord] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  const load = () =>
    call<WritingDictionary>("writing.dictionary", {})
      .then((d) => setWords(d.words))
      .catch(() => setWords([]));

  useEffect(() => {
    void load();
    const un = onAppEvent((ev) => {
      if (ev.type === "module" && ev.module === "writing" && ev.name === "dictionaryChanged") void load();
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const add = async () => {
    const w = word.trim();
    if (!w) return;
    try {
      const d = await call<WritingDictionary>("writing.dictionary_add", { word: w });
      setWords(d.words);
      setWord("");
      setError(null);
      toast.success(`Added “${w}” to your dictionary`);
    } catch (e) {
      if (e instanceof OpError && e.code.startsWith("validation")) setError(e.message);
      else reportError(e);
    }
  };

  const remove = async (w: string) => {
    try {
      const d = await call<WritingDictionary>("writing.dictionary_remove", { word: w });
      setWords(d.words);
      toast.info(`Removed “${w}” from your dictionary`, {
        label: "Undo",
        run: () => void call<WritingDictionary>("writing.dictionary_add", { word: w }).then((r) => setWords(r.words)).catch(reportError),
      });
    } catch (e) {
      reportError(e);
    }
  };

  const q = filter.trim().toLowerCase();
  const shown = (words ?? []).filter((w) => !q || w.toLowerCase().includes(q));

  return (
    <section className="card pad" style={{ maxWidth: 720, marginTop: 12 }} aria-label="Personal dictionary">
      <div className="h4">Personal dictionary</div>
      <p className="xs muted" style={{ marginTop: 0 }}>Words here are never underlined, in any project on this computer. Right-click an underlined word in the script to add it.</p>
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          void add();
        }}
      >
        <TextInput aria-label="New word" placeholder="Add a word" value={word} maxLength={64} onChange={(e) => setWord(e.target.value)} invalid={!!error} />
        <Button type="submit" icon={<Plus size={14} />} disabled={!word.trim()}>Add</Button>
      </form>
      {error && <div role="alert" className="xs" style={{ color: "var(--red)", marginTop: 4 }}>{error}</div>}
      {words === null ? (
        <Skeleton h={60} />
      ) : words.length === 0 ? (
        <p className="sm muted">Your dictionary is empty.</p>
      ) : (
        <>
          {words.length > 12 && (
            <TextInput aria-label="Filter words" placeholder="Filter words" value={filter} onChange={(e) => setFilter(e.target.value)} style={{ marginTop: 8, maxWidth: 260 }} />
          )}
          <p className="xs muted" aria-live="polite">{words.length === 1 ? "1 word" : `${words.length} words`}</p>
          <ul className="wr-words" aria-label="Words in your dictionary">
            {shown.map((w) => (
              <li key={w} className="chip out">
                <span>{w}</span>
                <button type="button" className="sc-x" aria-label={`Remove ${w}`} onClick={() => void remove(w)}>
                  <X size={11} aria-hidden />
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
