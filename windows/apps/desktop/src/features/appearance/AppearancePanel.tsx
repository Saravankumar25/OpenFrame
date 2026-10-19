// Settings > Appearance (ADR-0014; Screenplay and Production Depth Specification 47A.9 to 47A.16):
// Mode (Light, Dark, System), Theme (OpenFrame Default and the fifteen built-in themes), Import
// theme (file or pasted text, checked and previewed before anything is saved) and My Themes
// (apply, rename, export, delete). Appearance is a personal preference kept in the app database;
// it never touches a project.

import { useRef, useState, useSyncExternalStore, type KeyboardEvent } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Check, Download, FileUp, ClipboardPaste, Pencil, Trash2 } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useCommand } from "../../ipc/query";
import type { AppearanceMode } from "../../ipc/generated/AppearanceMode";
import type { AppearanceState } from "../../ipc/generated/AppearanceState";
import type { MyTheme } from "../../ipc/generated/MyTheme";
import type { ExportThemeResult } from "../../ipc/generated/ExportThemeResult";
import { Banner, Button, Chip, ConfirmDialog, Dialog, Field, Segmented, Skeleton, TextInput } from "../../design-system";
import { toast } from "../../app/toast";
import { BUILT_IN_THEMES, resolveMode, type ResolvedMode, type ThemeData } from "../../theme/engine";
import { useAppearanceState, useStoreAppearance } from "../../theme/useAppearance";
import { ThemePreview } from "./ThemePreview";
import { ThemeImportDialog, type ImportSource } from "./ThemeImportDialog";
import "./appearance.css";

const MODES = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
] as const satisfies readonly { value: AppearanceMode; label: string }[];

function subscribeSystem(cb: () => void): () => void {
  const q = typeof window.matchMedia === "function" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
  q?.addEventListener("change", cb);
  return () => q?.removeEventListener("change", cb);
}

/** The mode the screen is showing now (System resolved, updates live). */
function useShownMode(pref: AppearanceMode | undefined): ResolvedMode {
  const systemDark = useSyncExternalStore(subscribeSystem, () => resolveMode("system") === "dark");
  if (pref === "light" || pref === "dark") return pref;
  return systemDark ? "dark" : "light";
}

export function AppearancePanel() {
  const state = useAppearanceState();
  const data = state.data;
  const shown = useShownMode(data?.mode);
  const [announce, setAnnounce] = useState("");
  const [importing, setImporting] = useState<ImportSource | null>(null);

  const store = useStoreAppearance();
  const setMode = useCommand<{ mode: AppearanceMode }, AppearanceState>("appearance.set_mode", { onSuccess: store });
  const setTheme = useCommand<{ themeId: string }, AppearanceState>("appearance.set_theme", {
    onSuccess: (r, a) => {
      store(r);
      setAnnounce(`${themeName(a.themeId, data)} theme applied.`);
    },
  });

  if (state.isLoading) return <Skeleton h={220} />;
  if (state.isError || !data) {
    return (
      <Banner tone="err" actions={<Button size="sm" onClick={() => void state.refetch()}>Try Again</Button>}>
        The appearance settings could not be loaded. OpenFrame Default is shown. {state.error?.message}
      </Banner>
    );
  }

  const pickFile = async () => {
    try {
      const picked = await openDialog({ multiple: false, directory: false, filters: [{ name: "OpenFrame theme", extensions: ["json"] }] });
      if (typeof picked === "string") setImporting({ kind: "file", path: picked });
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <div className="appearance">
      <div className="sr-only" aria-live="polite">{announce}</div>
      {data.fallbackNotice && <Banner tone="warn">{data.fallbackNotice}</Banner>}

      <section className="ap-section" aria-labelledby="ap-mode">
        <h2 id="ap-mode">Mode</h2>
        <Segmented<AppearanceMode>
          ariaLabel="Mode"
          value={data.mode}
          options={MODES}
          onChange={(mode) => setMode.mutate({ mode }, { onSuccess: () => setAnnounce(`${MODES.find((m) => m.value === mode)?.label} mode.`) })}
        />
        <p className="ap-note">
          {data.mode === "system"
            ? `System follows Windows and changes with it. It is ${shown === "dark" ? "Dark" : "Light"} now.`
            : "Light and Dark work with every theme."}
        </p>
      </section>

      <section className="ap-section" aria-labelledby="ap-theme">
        <h2 id="ap-theme">Theme</h2>
        <ThemeGrid
          label="Theme"
          themes={BUILT_IN_THEMES}
          selected={data.themeId}
          mode={shown}
          onPick={(id) => setTheme.mutate({ themeId: id })}
        />
        <p className="ap-note">Themes change colours only. Your screenplay pages, PDFs and printed call sheets always stay black on white.</p>
      </section>

      <section className="ap-section" aria-labelledby="ap-mine">
        <div className="row" style={{ alignItems: "center", marginBottom: 8 }}>
          <h2 id="ap-mine" tabIndex={-1} style={{ margin: 0 }}>My Themes</h2>
          <span className="grow" />
          <Button size="sm" icon={<FileUp size={14} />} onClick={() => void pickFile()}>Import Theme File…</Button>
          <Button size="sm" icon={<ClipboardPaste size={14} />} onClick={() => setImporting({ kind: "text" })}>Paste Theme…</Button>
        </div>
        <MyThemes state={data} mode={shown} onApplied={(name) => setAnnounce(`${name} theme applied.`)} />
      </section>

      {importing && <ThemeImportDialog source={importing} onClose={() => setImporting(null)} onSaved={(msg) => setAnnounce(msg)} />}
    </div>
  );
}

function themeName(id: string, data: AppearanceState | undefined): string {
  return BUILT_IN_THEMES.find((t) => t.id === id)?.name ?? data?.myThemes.find((t) => t.id === id)?.name ?? "The";
}

/** Radio group of theme cards with arrow-key movement (one tab stop). */
function ThemeGrid({ label, themes, selected, mode, onPick }: {
  label: string;
  themes: ThemeData[];
  selected: string;
  mode: ResolvedMode;
  onPick: (id: string) => void;
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const current = Math.max(0, themes.findIndex((t) => t.id === selected));
  const onKey = (e: KeyboardEvent<HTMLButtonElement>, i: number) => {
    if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    const delta = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    const to = e.key === "Home" ? 0 : e.key === "End" ? themes.length - 1 : delta ? (i + delta + themes.length) % themes.length : -1;
    if (to < 0) return;
    e.preventDefault();
    refs.current[to]?.focus();
    onPick(themes[to].id);
  };
  return (
    <div className="ap-grid" role="radiogroup" aria-label={label}>
      {themes.map((t, i) => (
        <button
          key={t.id}
          ref={(el) => {
            refs.current[i] = el;
          }}
          type="button"
          role="radio"
          aria-checked={t.id === selected}
          tabIndex={i === current ? 0 : -1}
          className="ap-card"
          data-theme={t.id}
          data-mode={mode}
          onClick={() => onPick(t.id)}
          onKeyDown={(e) => onKey(e, i)}
        >
          <ThemePreview themeId={t.id} mode={mode} />
          <strong>
            {t.name}
            {t.id === selected && <Check size={13} aria-hidden />}
          </strong>
          <small>{t.description}</small>
        </button>
      ))}
    </div>
  );
}

function MyThemes({ state, mode, onApplied }: { state: AppearanceState; mode: ResolvedMode; onApplied: (name: string) => void }) {
  const [renaming, setRenaming] = useState<MyTheme | null>(null);
  const [deleting, setDeleting] = useState<MyTheme | null>(null);
  const store = useStoreAppearance();
  const apply = useCommand<{ themeId: string }, AppearanceState>("appearance.set_theme", { onSuccess: store });
  const remove = useCommand<{ id: string }, AppearanceState>("appearance.delete_theme", {
    onSuccess: (r, a) => {
      store(r);
      setDeleting(null);
      toast.success(a.id === state.themeId ? "Theme deleted. OpenFrame Default is applied." : "Theme deleted.");
      // The row and its button are gone: put focus on the section heading, not on the page body.
      window.setTimeout(() => document.getElementById("ap-mine")?.focus(), 0);
    },
  });

  const exportTheme = async (t: MyTheme) => {
    try {
      // A name is text, not a path: no folder separators or reserved characters in the suggestion.
      const fileName = `${t.name.replace(/[\\/:*?"<>|]+/g, " ").trim() || t.id}.json`;
      const path = await saveDialog({ defaultPath: fileName, filters: [{ name: "OpenFrame theme", extensions: ["json"] }] });
      if (!path) return;
      const r = await call<ExportThemeResult>("appearance.export_theme", { id: t.id, path });
      toast.success(`Saved ${r.fileName}.`);
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <>
      {state.myThemes.length === 0 ? (
        <p className="ap-note" style={{ marginTop: 0 }}>No imported themes yet. Theme files are small .json files, for example from the OpenFrame website.</p>
      ) : (
    <div className="ap-mine" role="list" aria-label="My Themes">
      {state.myThemes.map((t) => {
        const active = t.id === state.themeId;
        return (
          <div key={t.id} className="ap-row" role="listitem" aria-label={t.name}>
            {t.theme ? <ThemePreview themeId={t.id} mode={mode} /> : <span className="ap-pv" aria-hidden="true" />}
            <div className="grow">
              <div className="ap-name">{t.name}</div>
              {t.theme?.author && <div className="xs muted">By {t.theme.author}</div>}
              {t.problem && <div className="xs" style={{ color: "var(--red-t)" }}>This theme cannot be used. {t.problem}</div>}
            </div>
            {active && <Chip tone="a">In use</Chip>}
            {t.theme && !active && (
              <Button size="sm" onClick={() => apply.mutate({ themeId: t.id }, { onSuccess: () => onApplied(t.name) })}>Apply</Button>
            )}
            {t.theme && <Button size="sm" variant="ghost" icon={<Pencil size={13} />} aria-label={`Rename ${t.name}`} onClick={() => setRenaming(t)}>Rename</Button>}
            {t.theme && <Button size="sm" variant="ghost" icon={<Download size={13} />} aria-label={`Export ${t.name}`} onClick={() => void exportTheme(t)}>Export</Button>}
            <Button size="sm" variant="ghost" icon={<Trash2 size={13} />} aria-label={`Delete ${t.name}`} onClick={() => setDeleting(t)}>Delete</Button>
          </div>
        );
      })}
    </div>
      )}
      {renaming && <RenameThemeDialog theme={renaming} onClose={() => setRenaming(null)} />}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(v) => !v && setDeleting(null)}
        title={`Delete the theme "${deleting?.name ?? ""}"?`}
        confirmLabel="Delete Theme"
        danger
        busy={remove.isPending}
        onConfirm={() => deleting && remove.mutate({ id: deleting.id })}
      >
        <p style={{ marginTop: 0 }}>
          The theme is removed from My Themes. This cannot be undone; export it first if you may want it again.
          {deleting?.id === state.themeId && " It is in use, so OpenFrame Default is applied."} Your projects are not affected.
        </p>
      </ConfirmDialog>
    </>
  );
}

function RenameThemeDialog({ theme, onClose }: { theme: MyTheme; onClose: () => void }) {
  const [name, setName] = useState(theme.name);
  const store = useStoreAppearance();
  const rename = useCommand<{ id: string; name: string }, AppearanceState>("appearance.rename_theme", {
    onSuccess: (r) => {
      store(r);
      onClose();
    },
  });
  const trimmed = name.trim();
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Rename theme"
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" type="submit" form="ap-rename" disabled={!trimmed || trimmed.length > 60 || rename.isPending}>Rename</Button>
        </>
      }
    >
      <form
        id="ap-rename"
        onSubmit={(e) => {
          e.preventDefault();
          if (trimmed) rename.mutate({ id: theme.id, name: trimmed });
        }}
      >
        <Field label="Theme name" htmlFor="ap-rename-name" hint="Up to 60 characters.">
          <TextInput id="ap-rename-name" value={name} maxLength={60} autoFocus onChange={(e) => setName(e.target.value)} />
        </Field>
      </form>
    </Dialog>
  );
}
