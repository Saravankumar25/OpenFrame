// Characters (FSD §13, UX §3.9, mocks 067–069): a lightweight directory whose
// most useful feature is the list of screenplay scenes each character appears
// in (derived from character cues), plus typed relationships and optional
// reference links to Scene Cards.

import { useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, Image as ImageIcon, MoreHorizontal, Network, Plus, Users, X } from "lucide-react";
import { Banner, Button, Checkbox, Chip, ConfirmDialog, Dialog, EmptyState, Field, Menu, Select, Skeleton, TextArea, TextInput, cx } from "../../design-system";
import { assetSrc } from "../../ipc/assetSrc";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import { useNav } from "../../app/stores";
import { story, useCharacter, useCharacters, useRelationships, useStoryBoard } from "../../api/story";
import type { AssetInfo } from "../../ipc/generated/AssetInfo";
import type { StoryCharacterDetail } from "../../ipc/generated/StoryCharacterDetail";
import type { StoryCharacterDto } from "../../ipc/generated/StoryCharacterDto";
import type { StoryCharacterUsage } from "../../ipc/generated/StoryCharacterUsage";
import type { StoryItem } from "../../ipc/generated/StoryItem";
import type { StoryRelationshipDto } from "../../ipc/generated/StoryRelationshipDto";

const RELATION_SUGGESTIONS = ["father of", "mother of", "son of", "daughter of", "sibling of", "partner of", "friend of", "colleague of", "mentor of", "rival of", "enemy of"];

function Avatar({ name, image, size = 34 }: { name: string; image: AssetInfo | null; size?: number }) {
  const initials = name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join("");
  const src = assetSrc(image);
  if (src) {
    return <img src={src} alt="" width={size} height={size} style={{ borderRadius: "50%", objectFit: "cover", flex: "none" }} />;
  }
  return (
    <span className="avatar" aria-hidden style={{ width: size, height: size, flex: "none", fontSize: size * 0.38 }}>
      {initials || "?"}
    </span>
  );
}

export function CharactersView({
  episodeId,
  readOnly,
  focusId,
  wantNew,
  onNewHandled,
}: {
  episodeId: string | null;
  readOnly: boolean;
  focusId?: string;
  wantNew: boolean;
  onNewHandled: () => void;
}) {
  const [showArchived, setShowArchived] = useState(false);
  const list = useCharacters(episodeId, showArchived);
  const [selected, setSelected] = useState<string | null>(focusId ?? null);
  const [dialog, setDialog] = useState<"new" | null>(null);
  const [map, setMap] = useState(false);
  const chars = useMemo(() => list.data ?? [], [list.data]);
  const current = selected && chars.some((c) => c.id === selected) ? selected : (chars[0]?.id ?? null);

  useEffect(() => {
    if (focusId) setSelected(focusId);
  }, [focusId]);
  useEffect(() => {
    if (wantNew && !readOnly) {
      setDialog("new");
      onNewHandled();
    }
  }, [wantNew, readOnly, onNewHandled]);

  if (list.isLoading) return <Skeleton h={200} />;

  return (
    <div className="story-chars">
      {map ? (
        <RelationshipMap chars={chars} onBack={() => setMap(false)} readOnly={readOnly} />
      ) : (
        <>
          <section className="card story-char-list" aria-label="Characters">
            <div className="row" style={{ padding: "10px 12px", borderBottom: "1px solid var(--line)" }}>
              <b>Characters</b>
              <span className="chip">{chars.filter((c) => !c.archived).length}</span>
              <span className="grow" />
              {!readOnly && (
                <Button size="sm" variant="primary" icon={<Plus size={14} />} onClick={() => setDialog("new")}>
                  Add Character
                </Button>
              )}
            </div>
            <div style={{ overflow: "auto", flex: 1 }}>
              {chars.length === 0 ? (
                <EmptyState icon={<Users size={28} />} title="No characters yet.">
                  A lightweight directory. Its most useful feature is the list of scenes each character appears in.
                </EmptyState>
              ) : (
                chars.map((c) => (
                  <button
                    key={c.id}
                    type="button"
                    className={cx("li story-char-row", c.id === current && "sel")}
                    onClick={() => setSelected(c.id)}
                    aria-current={c.id === current}
                  >
                    <Avatar name={c.name} image={c.image} />
                    <span className="grow" style={{ textAlign: "left", minWidth: 0 }}>
                      <b style={{ textTransform: "uppercase" }}>{c.name}</b>
                      <span className="muted truncate" style={{ display: "block", fontSize: 12 }}>
                        {c.roleLabel ?? c.description ?? ""}
                      </span>
                    </span>
                    {c.archived && <Chip tone="out">Archived</Chip>}
                    <Chip tone="b">{c.sceneCount} scene{c.sceneCount === 1 ? "" : "s"}</Chip>
                  </button>
                ))
              )}
            </div>
            <div style={{ padding: "8px 12px", borderTop: "1px solid var(--line)" }}>
              <Checkbox checked={showArchived} onChange={setShowArchived} label="Show archived characters" />
            </div>
          </section>
          <section className="card story-char-detail" aria-label="Character details">
            {current ? (
              <CharacterDetail id={current} episodeId={episodeId} readOnly={readOnly} chars={chars} onOpenMap={() => setMap(true)} onDeleted={() => setSelected(null)} />
            ) : (
              <EmptyState title="Create your first character.">Name is all you need. Role, image and notes are optional.</EmptyState>
            )}
          </section>
        </>
      )}
      {dialog === "new" && (
        <NewCharacterDialog
          existing={chars}
          episodeId={episodeId}
          onClose={() => setDialog(null)}
          onDone={(id) => {
            setDialog(null);
            setSelected(id);
          }}
        />
      )}
    </div>
  );
}

function CharacterDetail({
  id,
  episodeId,
  readOnly,
  chars,
  onOpenMap,
  onDeleted,
}: {
  id: string;
  episodeId: string | null;
  readOnly: boolean;
  chars: StoryCharacterDto[];
  onOpenMap: () => void;
  onDeleted: () => void;
}) {
  const detail = useCharacter(id, episodeId);
  const go = useNav((s) => s.go);
  const [edit, setEdit] = useState(false);
  const [addRel, setAddRel] = useState(false);
  const [usage, setUsage] = useState<StoryCharacterUsage | null>(null);
  const d = detail.data;
  if (!d) return <Skeleton h={160} />;
  const c = d.character;

  const pickImage = async () => {
    try {
      const path = await open({ multiple: false, directory: false, title: "Choose an image", filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp"] }] });
      if (typeof path === "string") await story.setCharacterImage(c.id, path);
    } catch (e) {
      reportError(e);
    }
  };
  const askDelete = async () => {
    try {
      setUsage(await story.characterUsage(c.id));
    } catch (e) {
      reportError(e);
    }
  };
  const archive = async (archived: boolean) => {
    try {
      await story.setCharacterArchived(c.id, archived);
      toast.undoable(`${archived ? "Archived" : "Unarchived"} ${c.name}`);
    } catch (e) {
      reportError(e);
    }
  };

  return (
    <div style={{ padding: 16, overflow: "auto", height: "100%" }}>
      <div className="row" style={{ alignItems: "flex-start", gap: 12 }}>
        <Avatar name={c.name} image={c.image} size={56} />
        <div className="grow">
          <h2 style={{ margin: 0, textTransform: "uppercase", fontSize: 18 }}>{c.name}</h2>
          <div className="muted">{[c.roleLabel, c.description].filter(Boolean).join(" · ") || "No role yet"}</div>
          {c.archived && <Chip tone="out">Archived</Chip>}
        </div>
        {!readOnly && <Button size="sm" onClick={() => setEdit(true)}>Edit</Button>}
        <Button size="sm" icon={<Network size={14} />} onClick={onOpenMap}>
          Open Relationship Map
        </Button>
        {!readOnly && (
          <Menu
            align="end"
            trigger={
              <button type="button" className="iconbtn" aria-label={`More actions for ${c.name}`}>
                <MoreHorizontal size={16} />
              </button>
            }
            items={[
              { label: c.image ? "Change image…" : "Add image…", icon: <ImageIcon size={14} />, onSelect: () => void pickImage() },
              { label: "Remove image", disabled: !c.image, onSelect: () => void story.clearCharacterImage(c.id).catch(reportError) },
              { label: c.archived ? "Unarchive" : "Archive", onSelect: () => void archive(!c.archived) },
              { label: "Delete…", danger: true, separatorBefore: true, onSelect: () => void askDelete() },
            ]}
          />
        )}
      </div>

      <div className="h4" style={{ marginTop: 18 }}>Scenes this character appears in</div>
      {d.scenes.length ? (
        <div className="row" style={{ flexWrap: "wrap", gap: 6 }}>
          {d.scenes.map((s) => (
            <button key={s.sceneId} type="button" className="chip b" title={s.heading} onClick={() => go({ workspace: "screenplay", params: { sceneId: s.sceneId } })}>
              Sc {s.number}
            </button>
          ))}
        </div>
      ) : (
        <p className="muted" style={{ fontSize: 12.5 }}>
          No screenplay scenes mention {c.name} yet. Scenes appear here when the current draft has dialogue cues with this name.
        </p>
      )}

      <div className="row" style={{ marginTop: 18 }}>
        <div className="h4" style={{ margin: 0 }}>Relationships</div>
        <span className="grow" />
        {!readOnly && (
          <Button size="xs" icon={<Plus size={12} />} onClick={() => setAddRel(true)} disabled={chars.length < 2}>
            Add Relationship
          </Button>
        )}
      </div>
      <RelationshipList rels={d.relationships} readOnly={readOnly} />

      <LinkedCards detail={d} episodeId={episodeId} readOnly={readOnly} />

      <NotesField id={c.id} value={c.notes ?? ""} readOnly={readOnly} />

      {edit && <EditCharacterDialog c={c} onClose={() => setEdit(false)} />}
      {addRel && <AddRelationshipDialog from={c} chars={chars} onClose={() => setAddRel(false)} />}
      {usage && (
        <DeleteCharacterDialog
          c={c}
          usage={usage}
          onClose={() => setUsage(null)}
          onArchive={() => {
            setUsage(null);
            void archive(true);
          }}
          onDelete={() => {
            setUsage(null);
            void story
              .deleteCharacter(c.id)
              .then(() => {
                toast.undoable(`Deleted character ${c.name}`);
                onDeleted();
              })
              .catch(reportError);
          }}
        />
      )}
    </div>
  );
}

function RelationshipList({ rels, readOnly }: { rels: StoryRelationshipDto[]; readOnly: boolean }) {
  if (rels.length === 0) return <p className="muted" style={{ fontSize: 12.5 }}>No relationships yet.</p>;
  return (
    <ul style={{ listStyle: "none", padding: 0, margin: "6px 0" }}>
      {rels.map((r) => (
        <li key={r.id} className="row" style={{ gap: 8, padding: "3px 0" }}>
          <b style={{ textTransform: "uppercase" }}>{r.fromName}</b>
          <span className="muted">── {r.relationshipType} ──</span>
          <b style={{ textTransform: "uppercase" }}>{r.toName}</b>
          {r.note && <span className="muted" style={{ fontSize: 12 }}>({r.note})</span>}
          <span className="grow" />
          {!readOnly && (
            <button
              type="button"
              className="iconbtn sm"
              aria-label={`Remove relationship ${r.fromName} ${r.relationshipType} ${r.toName}`}
              onClick={() =>
                void story
                  .deleteRelationship(r.id)
                  .then(() => toast.undoable("Removed relationship"))
                  .catch(reportError)
              }
            >
              <X size={12} />
            </button>
          )}
        </li>
      ))}
    </ul>
  );
}

function allCards(items: StoryItem[], out: { id: string; label: string }[] = []) {
  for (const i of items) {
    if (i.kind === "card") out.push({ id: i.id, label: i.shortDescription || i.sceneHeading || "Blank scene card" });
    if (i.kind === "sequence") allCards(i.items, out);
  }
  return out;
}

function LinkedCards({ detail, episodeId, readOnly }: { detail: StoryCharacterDetail; episodeId: string | null; readOnly: boolean }) {
  const board = useStoryBoard(episodeId);
  const [pick, setPick] = useState("");
  const c = detail.character;
  const linked = new Set(detail.linkedCards.map((l) => l.cardId));
  const options = useMemo(() => {
    const b = board.data;
    if (!b) return [];
    const cards: { id: string; label: string }[] = [];
    b.acts.forEach((a) => allCards(a.items, cards));
    allCards(b.parking, cards);
    return cards.filter((x) => !linked.has(x.id));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [board.data, detail.linkedCards]);
  return (
    <>
      <div className="h4" style={{ marginTop: 18 }}>Linked Scene Cards (reference only)</div>
      {detail.linkedCards.length === 0 && <p className="muted" style={{ fontSize: 12.5 }}>Optional. Scene Cards never need characters.</p>}
      <ul style={{ listStyle: "none", padding: 0, margin: "6px 0" }}>
        {detail.linkedCards.map((l) => (
          <li key={l.cardId} className="row" style={{ gap: 8, padding: "3px 0" }}>
            <span className="grow truncate">{l.shortDescription || "Blank scene card"}</span>
            <span className="muted" style={{ fontSize: 12 }}>{l.location}</span>
            {!readOnly && (
              <button type="button" className="iconbtn sm" aria-label="Unlink card" onClick={() => void story.unlinkCard(c.id, l.cardId).catch(reportError)}>
                <X size={12} />
              </button>
            )}
          </li>
        ))}
      </ul>
      {!readOnly && options.length > 0 && (
        <div className="row" style={{ gap: 6 }}>
          <Select value={pick} onChange={setPick} ariaLabel="Scene Card to link" options={[{ value: "", label: "Link a Scene Card…" }, ...options.map((o) => ({ value: o.id, label: o.label.slice(0, 80) }))]} />
          <Button
            size="sm"
            disabled={!pick}
            onClick={() => {
              void story.linkCard(c.id, pick).catch(reportError);
              setPick("");
            }}
          >
            Link
          </Button>
        </div>
      )}
    </>
  );
}

function NotesField({ id, value, readOnly }: { id: string; value: string; readOnly: boolean }) {
  const [v, setV] = useState(value);
  const last = useRef(value);
  useEffect(() => {
    setV(value);
    last.current = value;
  }, [id, value]);
  return (
    <Field label="Notes" htmlFor="char-notes">
      <TextArea
        id="char-notes"
        rows={5}
        value={v}
        disabled={readOnly}
        onChange={(e) => setV(e.target.value)}
        onBlur={() => {
          if (v !== last.current) {
            last.current = v;
            void story.updateCharacter({ id, notes: v }).catch(reportError);
          }
        }}
      />
    </Field>
  );
}

function NewCharacterDialog({ existing, episodeId, onClose, onDone }: { existing: StoryCharacterDto[]; episodeId: string | null; onClose: () => void; onDone: (id: string) => void }) {
  const [name, setName] = useState("");
  const [role, setRole] = useState("");
  const [desc, setDesc] = useState("");
  const [image, setImage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const match = existing.find((c) => c.name.trim().toLowerCase() === name.trim().toLowerCase() && name.trim());
  const create = async () => {
    setBusy(true);
    try {
      const r = await story.createCharacter({ name, roleLabel: role || null, description: desc || null, imagePath: image, episodeId: null });
      toast.undoable(`Added character ${name.trim()}`);
      onDone(r.id);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  const pick = async () => {
    try {
      const p = await open({ multiple: false, directory: false, title: "Choose an image", filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp"] }] });
      if (typeof p === "string") setImage(p);
    } catch (e) {
      reportError(e);
    }
  };
  void episodeId;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title="New Character"
      footer={
        match ? (
          <>
            <Button onClick={onClose}>Cancel</Button>
            <Button disabled={busy} onClick={() => void create()}>Create Another Character Anyway</Button>
            <Button variant="primary" onClick={() => onDone(match.id)}>Use Existing</Button>
          </>
        ) : (
          <>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" disabled={busy || !name.trim()} onClick={() => void create()}>Create Character</Button>
          </>
        )
      }
    >
      <Field label="Name" required htmlFor="nc-name">
        <TextInput id="nc-name" value={name} onChange={(e) => setName(e.target.value)} autoFocus />
      </Field>
      {match && (
        <Banner tone="warn">
          Possible match: a character named {match.name.toUpperCase()}
          {match.description ? ` (${match.description})` : ""} already exists.
        </Banner>
      )}
      <Field label="Role / title" htmlFor="nc-role">
        <TextInput id="nc-role" value={role} onChange={(e) => setRole(e.target.value)} placeholder="Protagonist" />
      </Field>
      <Field label="Image">
        <div className="row" style={{ gap: 8 }}>
          <Button size="sm" icon={<ImageIcon size={14} />} onClick={() => void pick()}>
            {image ? "Change image…" : "Choose image…"}
          </Button>
          {image && <span className="muted truncate" style={{ fontSize: 12 }}>{image.split(/[\\/]/).pop()}</span>}
        </div>
      </Field>
      <Field label="Short description" htmlFor="nc-desc">
        <TextInput id="nc-desc" value={desc} onChange={(e) => setDesc(e.target.value)} placeholder="Add later" />
      </Field>
    </Dialog>
  );
}

function EditCharacterDialog({ c, onClose }: { c: StoryCharacterDto; onClose: () => void }) {
  const [name, setName] = useState(c.name);
  const [role, setRole] = useState(c.roleLabel ?? "");
  const [desc, setDesc] = useState(c.description ?? "");
  const save = async () => {
    try {
      await story.updateCharacter({ id: c.id, name, roleLabel: role, description: desc });
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={`Edit ${c.name}`}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim()} onClick={() => void save()}>Save</Button>
        </>
      }
    >
      <Field label="Name" required htmlFor="ec-name" hint="Renaming doesn't change screenplay text; use Find & Replace in the screenplay for that.">
        <TextInput id="ec-name" value={name} onChange={(e) => setName(e.target.value)} autoFocus />
      </Field>
      <Field label="Role / title" htmlFor="ec-role">
        <TextInput id="ec-role" value={role} onChange={(e) => setRole(e.target.value)} />
      </Field>
      <Field label="Short description" htmlFor="ec-desc">
        <TextArea id="ec-desc" rows={3} value={desc} onChange={(e) => setDesc(e.target.value)} />
      </Field>
    </Dialog>
  );
}

function AddRelationshipDialog({ from, chars, onClose }: { from: StoryCharacterDto; chars: StoryCharacterDto[]; onClose: () => void }) {
  const others = chars.filter((c) => c.id !== from.id);
  const [fromId, setFromId] = useState(from.id);
  const [to, setTo] = useState(others[0]?.id ?? "");
  const [type, setType] = useState("");
  const [note, setNote] = useState("");
  const save = async () => {
    try {
      await story.createRelationship(fromId, to, type, note || undefined);
      toast.undoable("Added relationship");
      onClose();
    } catch (e) {
      reportError(e);
    }
  };
  const opts = chars.map((c) => ({ value: c.id, label: c.name.toUpperCase() }));
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title="Add Relationship"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!type.trim() || !to || to === fromId} onClick={() => void save()}>Add Relationship</Button>
        </>
      }
    >
      <div className="row" style={{ gap: 8, alignItems: "flex-end" }}>
        <Field label="Character">
          <Select value={fromId} onChange={setFromId} ariaLabel="From character" options={opts} />
        </Field>
        <Field label="is" htmlFor="rel-type">
          <TextInput id="rel-type" value={type} maxLength={80} onChange={(e) => setType(e.target.value)} placeholder="e.g. second cousin of" autoFocus />
        </Field>
        <Field label="Character">
          <Select value={to} onChange={setTo} ariaLabel="To character" options={opts} />
        </Field>
      </div>
      {/* Any wording is allowed (owner change list item 2); these only save typing. */}
      <div className="rel-suggest" role="group" aria-label="Common relationships">
        <span className="xs muted">Type your own, or pick one:</span>
        {RELATION_SUGGESTIONS.map((s) => (
          <button key={s} type="button" className={"chip" + (type === s ? " a" : "")} aria-pressed={type === s} onClick={() => setType(s)}>
            {s}
          </button>
        ))}
      </div>
      <Field label="Note (optional)" htmlFor="rel-note">
        <TextInput id="rel-note" value={note} onChange={(e) => setNote(e.target.value)} />
      </Field>
    </Dialog>
  );
}

function DeleteCharacterDialog({
  c,
  usage,
  onClose,
  onArchive,
  onDelete,
}: {
  c: StoryCharacterDto;
  usage: StoryCharacterUsage;
  onClose: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  if (!usage.inUse) {
    return (
      <ConfirmDialog open onOpenChange={(v) => !v && onClose()} title={`Delete ${c.name}?`} confirmLabel="Delete Character" danger onConfirm={onDelete}>
        <p style={{ marginTop: 0 }}>The character moves to Recently Deleted. You can restore it or use Undo.</p>
      </ConfirmDialog>
    );
  }
  const uses = [
    usage.screenplayScenes ? `${usage.screenplayScenes} screenplay scene${usage.screenplayScenes === 1 ? "" : "s"}` : null,
    usage.castLinks ? "Cast & Crew" : null,
    usage.catalogLinks ? "the Production Catalog" : null,
  ].filter(Boolean);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={`${c.name} is in use`}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="danger" onClick={onDelete}>Delete Anyway</Button>
          <Button variant="primary" onClick={onArchive}>Archive Instead</Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>
        {c.name} appears in {uses.join(" and ")}. Archiving removes the character from this list without breaking those links.
      </p>
      <p className="muted">Screenplay text is never changed. If you delete, the character goes to Recently Deleted and can be restored.</p>
    </Dialog>
  );
}

const NODE_R = 20;
const initialOf = (name: string) => (name.trim()[0] ?? "?").toUpperCase();

interface MapEdge {
  path: string;
  label: { x: number; y: number };
}

/**
 * Lays out the relationship map so nothing overlaps (owner change list item 3): characters sit on a
 * circle in the order with the fewest crossing lines, two relationships between the same pair bend
 * apart, and each label takes the first spot along its line that is clear of other labels and of
 * the characters.
 */
export function layoutRelationshipMap(ids: string[], rels: { id: string; from: string; to: string }[]) {
  const n = ids.length;
  const W = 720;
  const H = Math.max(380, Math.min(560, 260 + n * 34));
  const R = Math.min(W, H) / 2 - 62;
  const order = n <= 8 ? fewestCrossings(ids, rels) : ids;
  const step = (2 * Math.PI) / Math.max(n, 1);
  const pos = new Map(order.map((id, i) => [id, { x: W / 2 + R * Math.cos(step * i - Math.PI / 2), y: H / 2 + R * Math.sin(step * i - Math.PI / 2) }]));
  const placed: { x: number; y: number }[] = [...pos.values()].map((p) => ({ x: p.x, y: p.y + 6 }));
  const pairCount = new Map<string, number>();
  const edges = new Map<string, MapEdge>();
  for (const r of rels) {
    const a = pos.get(r.from);
    const b = pos.get(r.to);
    if (!a || !b) continue;
    const key = [r.from, r.to].sort().join("|");
    const k = pairCount.get(key) ?? 0;
    pairCount.set(key, k + 1);
    // Parallel relationships between one pair alternate sides with a growing bend.
    const dir = r.from < r.to ? 1 : -1;
    const bend = k === 0 ? 0 : Math.ceil(k / 2) * 46 * (k % 2 ? 1 : -1) * dir;
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const len = Math.hypot(dx, dy) || 1;
    const cx = (a.x + b.x) / 2 + (-dy / len) * bend;
    const cy = (a.y + b.y) / 2 + (dx / len) * bend;
    const at = (t: number) => ({
      x: (1 - t) * (1 - t) * a.x + 2 * (1 - t) * t * cx + t * t * b.x,
      y: (1 - t) * (1 - t) * a.y + 2 * (1 - t) * t * cy + t * t * b.y,
    });
    let label = at(0.5);
    for (const t of [0.5, 0.38, 0.62, 0.28, 0.72, 0.44, 0.56]) {
      const p = at(t);
      if (placed.every((q) => Math.abs(q.x - p.x) > 70 || Math.abs(q.y - p.y) > 18)) {
        label = p;
        break;
      }
    }
    placed.push(label);
    edges.set(r.id, { path: "M " + a.x + " " + a.y + " Q " + cx + " " + cy + " " + b.x + " " + b.y, label });
  }
  return { W, H, pos, edges };
}

/** The circle order with the fewest crossing lines (two chords cross when their ends interleave). */
function fewestCrossings(ids: string[], rels: { from: string; to: string }[]): string[] {
  if (ids.length < 4) return ids;
  const pairs = rels.map((r) => [ids.indexOf(r.from), ids.indexOf(r.to)]).filter(([a, b]) => a >= 0 && b >= 0 && a !== b);
  const perm = ids.map((_, i) => i);
  const slot = new Array<number>(ids.length);
  let best = perm.slice();
  let bestScore = Infinity;
  const crossings = () => {
    for (let i = 0; i < perm.length; i++) slot[perm[i]] = i;
    let c = 0;
    for (let i = 0; i < pairs.length; i++) {
      const lo = Math.min(slot[pairs[i][0]], slot[pairs[i][1]]);
      const hi = Math.max(slot[pairs[i][0]], slot[pairs[i][1]]);
      for (let j = i + 1; j < pairs.length; j++) {
        const p = slot[pairs[j][0]];
        const q = slot[pairs[j][1]];
        if (p === lo || p === hi || q === lo || q === hi) continue;
        if (p > lo && p < hi ? !(q > lo && q < hi) : q > lo && q < hi) c++;
      }
    }
    return c;
  };
  // The first character stays in place (rotations are the same picture); every order of the rest is tried.
  const visit = (k: number) => {
    if (bestScore === 0) return;
    if (k === perm.length) {
      const c = crossings();
      if (c < bestScore) {
        bestScore = c;
        best = perm.slice();
      }
      return;
    }
    for (let i = k; i < perm.length; i++) {
      [perm[k], perm[i]] = [perm[i], perm[k]];
      visit(k + 1);
      [perm[k], perm[i]] = [perm[i], perm[k]];
    }
  };
  visit(1);
  return best.map((i) => ids[i]);
}

/** Relationship map (mock 069): an optional visual helper, not an analysis system. */
function RelationshipMap({ chars, onBack, readOnly }: { chars: StoryCharacterDto[]; onBack: () => void; readOnly: boolean }) {
  const rels = useRelationships();
  const [add, setAdd] = useState(false);
  const list = rels.data ?? [];
  const nodes = chars.filter((c) => !c.archived && list.some((r) => r.fromCharacterId === c.id || r.toCharacterId === c.id));
  const map = layoutRelationshipMap(
    nodes.map((n) => n.id),
    list.map((r) => ({ id: r.id, from: r.fromCharacterId, to: r.toCharacterId })),
  );
  const { W, H, pos } = map;
  return (
    <section className="card" style={{ flex: 1, padding: 16, overflow: "auto" }} aria-label="Character relationships">
      <div className="row">
        <Button size="sm" variant="ghost" icon={<ArrowLeft size={14} />} onClick={onBack}>
          Back to Characters
        </Button>
        <h2 style={{ margin: 0, fontSize: 16 }}>Character relationships</h2>
        <span className="grow" />
        {!readOnly && chars.length >= 2 && (
          <Button size="sm" variant="primary" icon={<Plus size={14} />} onClick={() => setAdd(true)}>
            Add Relationship
          </Button>
        )}
      </div>
      <p className="muted" style={{ fontSize: 12.5 }}>Optional visual helper, not a character-analysis system.</p>
      {list.length === 0 ? (
        <EmptyState title="No relationships yet.">Add a relationship such as “father of” or “colleague of”.</EmptyState>
      ) : (
        <>
          <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ maxHeight: 520 }} role="img" aria-label="Relationship map">
            {list.map((r) => {
              const e = map.edges.get(r.id);
              if (!e) return null;
              return (
                <g key={r.id}>
                  <path d={e.path} fill="none" stroke="var(--line2)" strokeWidth={2} />
                  <text
                    x={e.label.x}
                    y={e.label.y}
                    textAnchor="middle"
                    dominantBaseline="middle"
                    fontSize={11}
                    fill="var(--ink2)"
                    stroke="var(--panel)"
                    strokeWidth={5}
                    strokeLinejoin="round"
                    paintOrder="stroke"
                  >
                    {r.relationshipType}
                  </text>
                </g>
              );
            })}
            {nodes.map((n) => {
              const p = pos.get(n.id)!;
              return (
                <g key={n.id}>
                  <circle cx={p.x} cy={p.y} r={NODE_R} fill="var(--accent-soft)" stroke="var(--accent)" strokeWidth={1.5} />
                  <text x={p.x} y={p.y + 1} textAnchor="middle" dominantBaseline="middle" fontSize={13} fontWeight={700} fill="var(--accent-d)">
                    {initialOf(n.name)}
                  </text>
                  <text x={p.x} y={p.y + NODE_R + 14} textAnchor="middle" fontSize={11.5} fontWeight={700} fill="var(--ink)" stroke="var(--panel)" strokeWidth={4} strokeLinejoin="round" paintOrder="stroke">
                    {n.name.toUpperCase()}
                  </text>
                </g>
              );
            })}
          </svg>
          <RelationshipList rels={list} readOnly={readOnly} />
        </>
      )}
      {add && chars[0] && <AddRelationshipDialog from={chars[0]} chars={chars.filter((c) => !c.archived)} onClose={() => setAdd(false)} />}
    </section>
  );
}
