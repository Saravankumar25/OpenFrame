// Item drawer: the note editor (mock 030) and item detail / preview (mocks 031,
// 036). Text opens straight into editing and saves automatically (FSD §88.5,
// FSD-IDEA-011); nothing forces a title before closing.

import { useCallback, useEffect, useRef, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ExternalLink, FolderOpen, Pin, PinOff, X } from "lucide-react";
import { OpError, openAsset, openUrl, revealLocation } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import { Banner, Button, Chip, Drawer, Field, Skeleton, TextArea, TextInput } from "../../design-system";
import { toast } from "../../app/toast";
import { useVaultItem, vault } from "./api";
import { assetUrl } from "./ItemCard";
import {
  TYPE_LABEL,
  extLabel,
  formatBytes,
  formatDuration,
  isExternal,
  isTextType,
  isUnavailable,
  isVisualType,
  parseTags,
  relativeDay,
  type Scope,
} from "./model";

export type DrawerAction = "send" | "copy" | "move" | "collection" | "delete";

interface DrawerProps {
  scope: Scope;
  itemId: string;
  collections: VaultCollectionDto[];
  folders: VaultFolderDto[];
  /** Label of the copy action: "Copy to Project" or "Copy to Global Vault" (null = unavailable). */
  copyLabel: string | null;
  canSendToStory: boolean;
  focusTitle?: boolean;
  onClose: () => void;
  onAction: (action: DrawerAction, item: VaultItemDto) => void;
}

export function ItemDrawer(props: DrawerProps) {
  const q = useVaultItem(props.scope, props.itemId);
  if (q.isError) {
    return (
      <Drawer open onClose={props.onClose} title="Item not found">
        <Banner tone="warn">This item is no longer in the vault. It may have been deleted — look in Recently Deleted.</Banner>
      </Drawer>
    );
  }
  if (!q.data) {
    return (
      <Drawer open onClose={props.onClose} title="Loading…">
        <Skeleton h={120} />
      </Drawer>
    );
  }
  return <ItemEditor key={q.data.id} item={q.data} {...props} />;
}

type Fields = { title: string; body: string; caption: string; url: string; sourceText: string };
type FieldKey = keyof Fields;
const fromItem = (i: VaultItemDto): Fields => ({
  title: i.title ?? "",
  body: i.body ?? "",
  caption: i.caption ?? "",
  url: i.url ?? "",
  sourceText: i.sourceText ?? "",
});

/**
 * Debounced autosave with an editing buffer. Server values replace the buffer only
 * when they changed for another reason (undo, another window) — never as an echo
 * of our own save, so the caret never jumps while typing.
 */
function useAutosave(item: VaultItemDto, scope: Scope) {
  const [draft, setDraft] = useState<Fields>(() => fromItem(item));
  const [status, setStatus] = useState<"saved" | "saving" | "error">("saved");
  const [urlError, setUrlError] = useState<string | null>(null);
  const draftRef = useRef(draft);
  const baseline = useRef<Fields>(fromItem(item));
  const dirty = useRef(new Set<FieldKey>());
  const timer = useRef<number | null>(null);
  const inflight = useRef<Promise<void> | null>(null);

  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  // External changes (e.g. undo) flow into fields the user is not editing.
  useEffect(() => {
    const server = fromItem(item);
    const changed = (Object.keys(server) as FieldKey[]).filter((k) => server[k] !== baseline.current[k] && !dirty.current.has(k));
    if (changed.length === 0) return;
    baseline.current = { ...baseline.current, ...Object.fromEntries(changed.map((k) => [k, server[k]])) };
    setDraft((d) => ({ ...d, ...Object.fromEntries(changed.map((k) => [k, server[k]])) }));
  }, [item]);

  const flush = useCallback(async (): Promise<void> => {
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    if (inflight.current) await inflight.current;
    const keys = [...dirty.current];
    if (keys.length === 0) return;
    const snapshot = draftRef.current;
    dirty.current = new Set();
    const args: Record<string, string> = {};
    for (const k of keys) args[k] = snapshot[k];
    setStatus("saving");
    const run = (async () => {
      try {
        const saved = await vault.update({ store: scope, id: item.id, ...args });
        const s = fromItem(saved);
        for (const k of keys) baseline.current[k] = s[k];
        if (keys.includes("url")) setUrlError(null);
        setStatus(dirty.current.size > 0 ? "saving" : "saved");
      } catch (e) {
        for (const k of keys) dirty.current.add(k);
        setStatus("error");
        if (e instanceof OpError && e.is("validation") && keys.includes("url")) setUrlError(e.message);
        else reportError(e);
      }
    })();
    inflight.current = run;
    await run;
    inflight.current = null;
  }, [item.id, scope]);

  const set = useCallback(
    (k: FieldKey, v: string) => {
      setDraft((d) => ({ ...d, [k]: v }));
      draftRef.current = { ...draftRef.current, [k]: v };
      dirty.current.add(k);
      setStatus("saving");
      if (timer.current !== null) window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => void flush(), 600);
    },
    [flush],
  );

  // Never lose typing when the drawer closes or another item opens.
  useEffect(() => () => void flush(), [flush]);

  return { draft, set, flush, status, urlError };
}

function SaveIndicator({ status, onRetry }: { status: "saved" | "saving" | "error"; onRetry: () => void }) {
  if (status === "error") {
    return (
      <span className="row xs" role="status" style={{ color: "var(--red)" }}>
        Not saved yet.
        <Button size="xs" onClick={onRetry}>Retry</Button>
      </span>
    );
  }
  return <span className="xs muted" role="status">{status === "saving" ? "Saving…" : "Saved automatically"}</span>;
}

function TagEditor({ scope, item }: { scope: Scope; item: VaultItemDto }) {
  const [text, setText] = useState("");
  const add = () => {
    const tags = parseTags(text);
    if (tags.length === 0) return;
    vault.addTags(scope, [item.id], tags).then(() => setText(""), reportError);
  };
  return (
    <Field label="Tags" htmlFor="vv-tag-input">
      <div className="row wrap" style={{ gap: 5 }}>
        {item.tags.map((t) => (
          <span key={t} className="chip">
            {t}
            <button type="button" className="vv-x" aria-label={`Remove tag ${t}`} onClick={() => vault.removeTag(scope, [item.id], t).catch(reportError)}>
              <X size={11} />
            </button>
          </span>
        ))}
        <TextInput
          id="vv-tag-input"
          value={text}
          placeholder="Add tag…"
          style={{ flex: "1 1 120px", minWidth: 110 }}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === ",") {
              e.preventDefault();
              add();
            }
          }}
          onBlur={add}
        />
      </div>
    </Field>
  );
}

function CollectionEditor({ scope, item, collections, onMore }: { scope: Scope; item: VaultItemDto; collections: VaultCollectionDto[]; onMore: () => void }) {
  const mine = collections.filter((c) => item.collectionIds.includes(c.id));
  return (
    <Field label="Collections">
      <div className="row wrap" style={{ gap: 5 }}>
        {mine.map((c) => (
          <span key={c.id} className="chip a">
            {c.name}
            <button type="button" className="vv-x" aria-label={`Remove from ${c.name}`} onClick={() => vault.removeFromCollection(scope, [item.id], c.id).catch(reportError)}>
              <X size={11} />
            </button>
          </span>
        ))}
        {mine.length === 0 && <span className="xs muted">Not in a collection</span>}
        <Button size="xs" variant="ghost" onClick={onMore}>Add to Collection…</Button>
      </div>
    </Field>
  );
}

function Preview({ item, scope }: { item: VaultItemDto; scope: Scope }) {
  const [failed, setFailed] = useState(false);
  const a = item.asset;
  if (!a) return null;
  const url = assetUrl(a);
  const openExt = () => void openAsset(a.id, { global: scope === "global" }).catch(reportError);
  const facts = [a.originalName, formatBytes(a.byteSize), a.width && a.height ? `${a.width}×${a.height}` : null, formatDuration(a.durationMs)]
    .filter(Boolean)
    .join(" · ");
  let media = null;
  if (url && !failed) {
    if (isVisualType(item.itemType)) {
      media = <img src={url} alt={item.displayName} onError={() => setFailed(true)} className="vv-preview" />;
    } else if (item.itemType === "audio" || item.itemType === "voice") {
      media = <audio controls preload="metadata" src={url} onError={() => setFailed(true)} style={{ width: "100%" }} aria-label={`Play ${item.displayName}`} />;
    } else if (item.itemType === "video") {
      media = <video controls preload="metadata" src={url} onError={() => setFailed(true)} className="vv-preview" aria-label={`Play ${item.displayName}`} />;
    }
  }
  return (
    <div style={{ marginBottom: 12 }}>
      {media ?? (
        <div className="filecard card" style={{ marginBottom: 6 }}>
          <span className={`fic ${item.itemType === "pdf" ? "pdf" : item.itemType === "document" ? "doc" : item.itemType === "video" ? "vid" : ""}`}>
            {item.itemType === "pdf" ? "PDF" : extLabel(a.originalName)}
          </span>
          <div style={{ minWidth: 0 }}>
            <div className="b vv-ellipsis">{a.originalName}</div>
            <div className="xs muted">
              {item.itemType === "pdf"
                ? "PDFs open in your PDF app."
                : failed
                  ? "This file can't be previewed here, but it is stored safely."
                  : "Preview isn't available for this file type, but it is stored safely."}
            </div>
          </div>
        </div>
      )}
      <div className="row" style={{ justifyContent: "space-between" }}>
        <span className="xs muted vv-ellipsis" title={facts}>{facts}</span>
        {!isUnavailable(item) && (
          <Button size="xs" icon={<ExternalLink size={12} />} onClick={openExt}>Open externally</Button>
        )}
      </div>
    </div>
  );
}

function StorageMarker({ item, scope, onRelinked }: { item: VaultItemDto; scope: Scope; onRelinked: () => void }) {
  const a = item.asset;
  if (!a) return null;
  const relink = async () => {
    const picked = await openDialog({ multiple: false, directory: false, title: "Relink file" });
    if (typeof picked !== "string") return;
    try {
      await vault.relink(scope, item.id, picked);
      toast.success("File relinked. The item and its notes are unchanged.");
      onRelinked();
    } catch (e) {
      reportError(e);
    }
  };
  const openLocation = () => {
    // The folder is resolved from the asset id in Rust: stored paths are content and are
    // never handed back to the shell from the webview.
    void revealLocation(scope === "global" ? "globalAssetFolder" : "assetFolder", a.id).catch(() =>
      toast.error("That folder isn't available either. Reconnect the drive or relink the file."),
    );
  };
  if (isUnavailable(item)) {
    return (
      <div style={{ marginBottom: 12 }}>
        <Banner tone="err" actions={isExternal(item) ? <span className="row"><Button size="xs" onClick={() => void relink()}>Relink…</Button><Button size="xs" icon={<FolderOpen size={12} />} onClick={openLocation}>Open Location</Button></span> : undefined}>
          <b>File unavailable.</b> The original file could not be found{a.path ? <> at <span className="selectable-text">{a.path}</span></> : null}.
          <div>OpenFrame keeps this item and its notes. Nothing has been deleted.</div>
        </Banner>
        {isExternal(item) && <div className="xs muted" style={{ marginTop: 4 }}>This item links to a file outside the project. Files stored inside the project show a different marker.</div>}
      </div>
    );
  }
  return (
    <div className="row" style={{ marginBottom: 10, gap: 6 }}>
      {isExternal(item) ? (
        <>
          <Chip tone="b" title={a.path ?? undefined}>Linked file</Chip>
          <span className="xs muted">Stays in its original location on this computer.</span>
        </>
      ) : (
        <>
          <Chip tone="g">Stored in the vault</Chip>
          <span className="xs muted">{scope === "global" ? "Kept in your Global Idea Vault folder." : "Kept inside this project."}</span>
        </>
      )}
    </div>
  );
}

function ItemEditor({ item, scope, collections, folders, copyLabel, canSendToStory, focusTitle, onClose, onAction }: DrawerProps & { item: VaultItemDto }) {
  const { draft, set, flush, status, urlError } = useAutosave(item, scope);
  const titleRef = useRef<HTMLInputElement>(null);
  const bodyRef = useRef<HTMLTextAreaElement>(null);
  const text = isTextType(item.itemType);
  const label = TYPE_LABEL[item.itemType];
  const folder = folders.find((f) => f.id === item.folderId);

  useEffect(() => {
    if (focusTitle) titleRef.current?.focus();
    else if (text && !item.body) bodyRef.current?.focus();
    // Only on open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const close = () => {
    void flush();
    onClose();
  };
  const togglePin = () =>
    vault.setPinned(scope, [item.id], !item.pinned).then(() => toast.undoable(item.pinned ? "Unpinned" : "Pinned", scope), reportError);
  const heading = draft.title.trim() || (item.itemType === "note" ? "Untitled note" : item.title ? item.title : item.displayName);

  return (
    <Drawer
      open
      onClose={close}
      title={heading}
      typeLabel={isExternal(item) ? `${label} · external file` : label}
      width={text ? "default" : "w"}
      footer={
        <>
          <Button variant="danger" size="sm" onClick={() => onAction("delete", item)}>Delete</Button>
          <span className="grow" />
          {canSendToStory && <Button size="sm" onClick={() => void flush().then(() => onAction("send", item))}>Send to Story</Button>}
        </>
      }
    >
      <div className="row wrap" style={{ gap: 6, marginBottom: 12 }}>
        <Button size="xs" icon={item.pinned ? <PinOff size={12} /> : <Pin size={12} />} onClick={togglePin}>{item.pinned ? "Unpin" : "Pin"}</Button>
        {!text && <Button size="xs" onClick={() => titleRef.current?.focus()}>Rename</Button>}
        <Button size="xs" onClick={() => onAction("move", item)}>Move</Button>
        {copyLabel && <Button size="xs" onClick={() => void flush().then(() => onAction("copy", item))}>{copyLabel}</Button>}
      </div>

      {!text && item.itemType !== "url" && <Preview item={item} scope={scope} />}
      {!text && item.itemType !== "url" && <StorageMarker item={item} scope={scope} onRelinked={() => undefined} />}

      <Field label={text ? "Title (optional)" : item.itemType === "voice" ? "Name (optional)" : "Title"} htmlFor="vv-title">
        <TextInput
          id="vv-title"
          ref={titleRef}
          value={draft.title}
          placeholder={text ? (item.itemType === "quote" ? "Untitled quote" : "Untitled note") : "Leave blank to keep it untitled"}
          maxLength={300}
          onChange={(e) => set("title", e.target.value)}
          onBlur={() => void flush()}
        />
      </Field>

      {item.itemType === "url" && (
        <Field label="Link" htmlFor="vv-url" error={urlError}>
          <div className="row">
            <TextInput id="vv-url" value={draft.url} invalid={!!urlError} onChange={(e) => set("url", e.target.value)} onBlur={() => void flush()} />
            <Button size="sm" icon={<ExternalLink size={13} />} disabled={!item.url} onClick={() => item.url && void openUrl(item.url).catch(reportError)}>
              Open
            </Button>
          </div>
        </Field>
      )}

      {(text || item.itemType === "url") && (
        <Field label={item.itemType === "quote" ? "Quote" : item.itemType === "url" ? "Your note" : "Note"} htmlFor="vv-body">
          <TextArea
            id="vv-body"
            ref={bodyRef}
            value={draft.body}
            rows={item.itemType === "url" ? 4 : 12}
            placeholder={item.itemType === "quote" ? "The words, exactly as you heard or read them" : item.itemType === "url" ? "Why this link matters (optional)" : "Write anything…"}
            onChange={(e) => set("body", e.target.value)}
            onBlur={() => void flush()}
          />
        </Field>
      )}

      {item.itemType === "quote" && (
        <Field label="Source (optional)" htmlFor="vv-source">
          <TextInput id="vv-source" value={draft.sourceText} placeholder="Who said it, where you found it" onChange={(e) => set("sourceText", e.target.value)} onBlur={() => void flush()} />
        </Field>
      )}

      {!text && item.itemType !== "url" && (
        <>
          <Field label="Caption" htmlFor="vv-caption">
            <TextArea id="vv-caption" rows={3} value={draft.caption} placeholder="Optional" onChange={(e) => set("caption", e.target.value)} onBlur={() => void flush()} />
          </Field>
          <Field label="Source link" htmlFor="vv-srclink" error={urlError} hint="Optional">
            <TextInput id="vv-srclink" value={draft.url} placeholder="https://…" invalid={!!urlError} onChange={(e) => set("url", e.target.value)} onBlur={() => void flush()} />
          </Field>
        </>
      )}

      <div className="row" style={{ justifyContent: "space-between", margin: "-4px 0 12px" }}>
        <SaveIndicator status={status} onRetry={() => void flush()} />
        <span className="xs muted">Added {relativeDay(item.createdAt)} · Modified {relativeDay(item.updatedAt)}</span>
      </div>

      <CollectionEditor scope={scope} item={item} collections={collections} onMore={() => onAction("collection", item)} />
      <TagEditor scope={scope} item={item} />
      <Field label="Folder">
        <div className="row">
          <span className="sm">{folder ? folder.name : "No folder"}</span>
          <Button size="xs" variant="ghost" onClick={() => onAction("move", item)}>Move…</Button>
        </div>
      </Field>

      {item.sourceGlobalItemId && (
        <div className="xs muted" style={{ marginTop: 8 }}>Copied from your Global Idea Vault. Editing this copy never changes the original.</div>
      )}
    </Drawer>
  );
}
