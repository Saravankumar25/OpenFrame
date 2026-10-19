// Import a theme (spec 47A.11): read a file the user picked, or pasted text, have Rust check it
// (format, limits, contrast), show the preview in Light and Dark, then Save to My Themes or Save and
// Apply. Nothing is stored until the user saves, and Rust checks the source again on save.

import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { call, type OpError } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { AppearanceState } from "../../ipc/generated/AppearanceState";
import type { ThemeCheck } from "../../ipc/generated/ThemeCheck";
import { Banner, Button, Dialog, Field, Skeleton, TextArea } from "../../design-system";
import { toast } from "../../app/toast";
import { PREVIEW_ID, setPreviewTheme } from "../../theme/engine";
import { myThemeData, useStoreAppearance } from "../../theme/useAppearance";
import { ThemePreview } from "./ThemePreview";

export type ImportSource = { kind: "file"; path: string } | { kind: "text" };

/** Largest theme accepted (bytes), as in Rust. */
export const MAX_THEME_BYTES = 10 * 1024;

export function ThemeImportDialog({ source, onClose, onSaved }: {
  source: ImportSource;
  onClose: () => void;
  onSaved: (announcement: string) => void;
}) {
  const [text, setText] = useState("");
  const [result, setResult] = useState<ThemeCheck | null>(null);
  const args = source.kind === "file" ? { path: source.path } : { text };
  const bytes = new TextEncoder().encode(text).length;
  const store = useStoreAppearance();

  const check = useMutation<ThemeCheck, OpError, object>({
    mutationFn: (a) => call<ThemeCheck>("appearance.check_theme", a),
    onSuccess: (r) => setResult(r),
    onError: reportError,
  });
  const save = useMutation<AppearanceState, OpError, boolean>({
    // The digest makes Rust refuse the save if the file changed after this preview.
    mutationFn: (apply) => call<AppearanceState>("appearance.import_theme", { ...args, apply, expectedDigest: result?.digest ?? undefined }),
    onSuccess: (s, apply) => {
      store(s);
      const name = result?.theme?.name ?? "The theme";
      toast.success(apply ? `${name} is saved and applied.` : `${name} is saved to My Themes.`);
      onSaved(apply ? `${name} theme applied.` : `${name} saved to My Themes.`);
      onClose();
    },
    onError: reportError,
  });

  // A file is checked as soon as the dialog opens.
  const { mutate: runCheck } = check;
  useEffect(() => {
    if (source.kind === "file") runCheck({ path: source.path });
  }, [source, runCheck]);

  // Show the checked theme in the preview cards only, and remove it again when the dialog closes.
  const previewTheme = result?.theme ?? null;
  useEffect(() => {
    setPreviewTheme(previewTheme ? myThemeData([{ id: previewTheme.id, name: previewTheme.name, theme: previewTheme, problem: null }])[0] : null);
    return () => setPreviewTheme(null);
  }, [previewTheme]);

  const usable = result !== null && result.status !== "rejected";
  const failures = result?.contrast.filter((r) => !r.passes) ?? [];
  const summary = check.isPending
    ? "Checking the theme."
    : !result
      ? ""
      : result.status === "ok"
        ? "Check finished: readable in Light and Dark."
        : result.status === "corrected"
          ? "Check finished: small readability changes were made."
          : "Check finished: this theme cannot be used.";
  // After a check the button that was pressed is gone: move focus to the result.
  const resultRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (result) resultRef.current?.focus();
  }, [result]);

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Import theme"
      sub={source.kind === "file" ? "Checked before anything is saved." : "Paste the text of an OpenFrame theme file. It is checked before anything is saved."}
      size="lg"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          {source.kind === "text" && !result && (
            <Button variant="primary" disabled={!text.trim() || bytes > MAX_THEME_BYTES || check.isPending} onClick={() => check.mutate({ text })}>
              Check Theme
            </Button>
          )}
          {result && <Button disabled={!usable || save.isPending} onClick={() => save.mutate(false)}>Save to My Themes</Button>}
          {result && <Button variant="primary" disabled={!usable || save.isPending} onClick={() => save.mutate(true)}>Save and Apply</Button>}
        </>
      }
    >
      {source.kind === "text" && !result && (
        <Field
          label="Theme text"
          htmlFor="ap-paste"
          hint={`${(bytes / 1024).toFixed(1)} KB of 10 KB.`}
          error={bytes > MAX_THEME_BYTES ? "This is larger than 10 KB. Theme files are small; this is not one." : undefined}
        >
          <TextArea id="ap-paste" className="textarea ap-paste" value={text} spellCheck={false} autoFocus invalid={bytes > MAX_THEME_BYTES} onChange={(e) => setText(e.target.value)} />
        </Field>
      )}
      <div className="sr-only" aria-live="polite">{summary}</div>
      {check.isPending && <Skeleton h={120} />}
      {result && (
        <div ref={resultRef} tabIndex={-1} aria-label="Check result" role="group" style={{ outline: "none" }}>
          {result.theme && (
            <>
              <div className="h4" style={{ marginBottom: 2 }}>{result.theme.name}</div>
              <div className="xs muted">
                {result.theme.author ? `By ${result.theme.author}. ` : ""}
                {result.theme.description}
              </div>
              <div className="ap-previews">
                {(["light", "dark"] as const).map((m) => (
                  <div key={m} role="group" className="ap-card" data-theme={PREVIEW_ID} data-mode={m} aria-label={`${m === "light" ? "Light" : "Dark"} preview`}>
                    <ThemePreview themeId={PREVIEW_ID} mode={m} />
                    <strong>{m === "light" ? "Light" : "Dark"}</strong>
                    <small>Main text, muted text and the accent colour.</small>
                  </div>
                ))}
              </div>
            </>
          )}
          {result.status === "ok" && <Banner tone="ok">This theme is readable in Light and Dark.</Banner>}
          {result.status === "corrected" && (
            <Banner tone="info">
              Small changes were made so text stays readable:
              <ul className="ap-list">{result.notes.map((n) => <li key={n}>{n}</li>)}</ul>
            </Banner>
          )}
          {result.status === "rejected" && (
            <Banner tone="err">
              This theme cannot be used. Nothing was changed.
              <ul className="ap-list">{result.problems.map((p) => <li key={p}>{p}</li>)}</ul>
            </Banner>
          )}
          {result.contrast.length > 0 && (
            <details style={{ marginTop: 8 }}>
              <summary className="sm">Readability check ({failures.length === 0 ? "all pass" : `${failures.length} below 4.5 to 1`})</summary>
              <table className="ap-contrast">
                <thead>
                  <tr><th scope="col">Mode</th><th scope="col">Text</th><th scope="col">Contrast</th><th scope="col">Result</th></tr>
                </thead>
                <tbody>
                  {result.contrast.map((r) => (
                    <tr key={`${r.mode}-${r.label}`}>
                      <td>{r.mode === "light" ? "Light" : "Dark"}</td>
                      <td>{r.label}</td>
                      <td className="num">{r.ratio.toFixed(2)} to 1</td>
                      <td>{r.passes ? "Pass" : "Too low"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </details>
          )}
          {source.kind === "text" && (
            <Button size="sm" variant="ghost" style={{ marginTop: 8 }} onClick={() => setResult(null)}>Edit the text</Button>
          )}
        </div>
      )}
    </Dialog>
  );
}
