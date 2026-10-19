// Screenplay picker (ADR-0012 amendment of 2026-10-08): a project may hold
// several screenplays, each with its own drafts, locked drafts, scene numbers
// and page setup. The picker always shows which screenplay is open and lets
// the writer switch, add a new one, rename the open one or move it to
// Recently Deleted (kept 30 days, restorable with everything in it).

import { useState } from "react";
import { ChevronDown, Clapperboard } from "lucide-react";
import { Banner, Button, Dialog, Field, Menu, TextInput, type MenuItemSpec } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { CreatedScreenplay } from "../../ipc/generated/CreatedScreenplay";
import type { ScreenplayDto } from "../../ipc/generated/ScreenplayDto";
import type { ScreenplaySummary } from "../../ipc/generated/ScreenplaySummary";
import { toast } from "../../app/toast";

const MAX_TITLE = 200;

/** Another screenplay of the list already uses this title (case and outer spaces ignored). */
export function titleTaken(screenplays: ScreenplaySummary[], title: string, exceptId?: string): boolean {
  const t = title.trim().toLowerCase();
  return screenplays.some((s) => s.id !== exceptId && s.title.trim().toLowerCase() === t);
}

function draftCountLabel(s: ScreenplaySummary): string {
  return `${s.draftCount} draft${s.draftCount === 1 ? "" : "s"}`;
}

export function ScreenplayPicker({ screenplays, openId, canEdit, canDelete = false, onSwitch, onNew, onRename, onDelete }: {
  screenplays: ScreenplaySummary[];
  openId: string;
  canEdit: boolean;
  /** Owner and Editor (capability SoftDelete). */
  canDelete?: boolean;
  onSwitch: (id: string) => void;
  onNew: () => void;
  onRename: () => void;
  onDelete?: () => void;
}) {
  const open = screenplays.find((s) => s.id === openId);
  const title = open?.title ?? "Screenplay";
  const items: MenuItemSpec[] = [
    { label: screenplays.length > 1 ? `Screenplays in this project (${screenplays.length})` : "Screenplay", header: true },
    ...screenplays.map((s) => ({
      label: `${s.title} · ${draftCountLabel(s)}${s.id === openId ? " · Open" : ""}`,
      disabled: s.id === openId,
      onSelect: () => onSwitch(s.id),
    })),
    ...(canEdit
      ? [
          { label: "New Screenplay…", separatorBefore: true, onSelect: onNew },
          { label: "Rename Screenplay…", onSelect: onRename },
        ]
      : []),
    ...(canDelete && onDelete ? [{ label: "Delete Screenplay…", separatorBefore: !canEdit, danger: true, onSelect: onDelete }] : []),
  ];
  return (
    <Menu
      trigger={
        <button type="button" className="btn" style={{ padding: "5px 10px" }} aria-label={`Screenplay: ${title}. Switch screenplay`}>
          <Clapperboard size={13} aria-hidden /> <b>{title}</b> <ChevronDown size={13} aria-hidden />
        </button>
      }
      items={items}
    />
  );
}

function errorMessage(e: unknown, fallback: string): string {
  const m = (e as { message?: string }).message;
  if (!(e as { code?: string }).code?.startsWith("validation")) reportError(e);
  return m ?? fallback;
}

/** Add another screenplay beside the existing ones (they are not changed). */
export function NewScreenplayDialog({ episodeId, screenplays, onClose, onCreated }: {
  episodeId: string | null;
  screenplays: ScreenplaySummary[];
  onClose: () => void;
  onCreated: (r: CreatedScreenplay) => void;
}) {
  const [title, setTitle] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const taken = title.trim() !== "" && titleTaken(screenplays, title);
  const create = async () => {
    setBusy(true);
    setErr(null);
    try {
      const r = await call<CreatedScreenplay>("screenplay.create", { title: title.trim(), ...(episodeId ? { episodeId } : {}) });
      toast.undoable(`Created screenplay “${r.screenplay.title}”`);
      onCreated(r);
    } catch (e) {
      setErr(errorMessage(e, "The screenplay could not be created."));
    } finally {
      setBusy(false);
    }
  };
  const shown = err ?? (taken ? "Another screenplay here already has this title. Choose a different title." : null);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="New Screenplay"
      sub="A separate screenplay with its own drafts, locked drafts, scene numbers and page setup. Your other screenplays are not changed."
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!title.trim() || taken || busy} onClick={() => void create()}>Create Screenplay</Button>
        </>
      }
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (title.trim() && !taken && !busy) void create();
        }}
      >
        <Field label="Title" required htmlFor="ns-title" error={shown} hint="For example the short version, or a second episode you shoot together.">
          <TextInput id="ns-title" value={title} autoFocus maxLength={MAX_TITLE} invalid={!!shown} onChange={(e) => setTitle(e.target.value)} />
        </Field>
      </form>
    </Dialog>
  );
}

/** Rename the open screenplay; its drafts and text stay as they are. */
export function RenameScreenplayDialog({ screenplay, screenplays, onClose }: {
  screenplay: ScreenplayDto;
  screenplays: ScreenplaySummary[];
  onClose: () => void;
}) {
  const [title, setTitle] = useState(screenplay.title);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const unchanged = title.trim() === screenplay.title.trim();
  const taken = title.trim() !== "" && titleTaken(screenplays, title, screenplay.id);
  const save = async () => {
    setBusy(true);
    setErr(null);
    try {
      const r = await call<ScreenplayDto>("screenplay.rename", { screenplayId: screenplay.id, title: title.trim() });
      toast.undoable(`Renamed the screenplay to “${r.title}”`);
      onClose();
    } catch (e) {
      setErr(errorMessage(e, "The screenplay could not be renamed."));
    } finally {
      setBusy(false);
    }
  };
  const ready = !!title.trim() && !taken && !unchanged && !busy;
  const shown = err ?? (taken ? "Another screenplay here already has this title. Choose a different title." : null);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Rename Screenplay"
      sub="Only the name changes. Drafts and their text stay as they are."
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!ready} onClick={() => void save()}>Rename</Button>
        </>
      }
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (ready) void save();
        }}
      >
        <Field label="Title" required htmlFor="rs-title" error={shown}>
          <TextInput id="rs-title" value={title} autoFocus maxLength={MAX_TITLE} invalid={!!shown} onChange={(e) => setTitle(e.target.value)} />
        </Field>
      </form>
    </Dialog>
  );
}

/**
 * Move the open screenplay, with all its drafts, to Recently Deleted. The core refuses it while
 * the screenplay is the Production Source and says why; that reason is shown here.
 */
export function DeleteScreenplayDialog({ screenplay, screenplays, onClose, onDeleted }: {
  screenplay: ScreenplayDto;
  screenplays: ScreenplaySummary[];
  onClose: () => void;
  /** Called with the screenplay to open next, or null when none is left. */
  onDeleted: (nextId: string | null) => void;
}) {
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const others = screenplays.filter((s) => s.id !== screenplay.id);
  const remove = async () => {
    setBusy(true);
    setErr(null);
    try {
      await call("screenplay.delete", { screenplayId: screenplay.id });
      toast.undoable(`Moved “${screenplay.title}” to Recently Deleted`);
      onDeleted(others[0]?.id ?? null);
    } catch (e) {
      const code = (e as { code?: string }).code ?? "";
      if (!code.startsWith("conflict") && !code.startsWith("validation")) reportError(e);
      setErr((e as { message?: string }).message ?? "The screenplay could not be deleted.");
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={`Delete “${screenplay.title}”?`}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="danger" disabled={busy} onClick={() => void remove()}>Delete Screenplay</Button>
        </>
      }
    >
      <p>
        “{screenplay.title}” and all its drafts move to Recently Deleted. They stay there for 30 days, and you can restore the screenplay
        with its drafts, locked drafts, scene numbers and page setup as they are now.
      </p>
      <p className="sm muted">
        {others.length > 0
          ? "Your other screenplays are not changed."
          : "This is the only screenplay in this project. Afterwards you can start a new one or restore this one from Recently Deleted."}
      </p>
      {err && <Banner tone="warn">{err}</Banner>}
    </Dialog>
  );
}
