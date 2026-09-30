// Characters (FSD §13, UX §3.9, mocks 067–069): a lightweight directory whose
// most useful feature is the list of screenplay scenes each character appears
// in (derived from character cues), plus typed relationships and optional
// reference links to Scene Cards.

import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, Image as ImageIcon, MoreHorizontal, Network, Plus, Users, X } from "lucide-react";
import { Banner, Button, Checkbox, Chip, ConfirmDialog, Dialog, EmptyState, Field, Menu, Select, Skeleton, TextArea, TextInput, cx } from "../../design-system";
import { inTauri } from "../../ipc/client";
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
  if (image?.available && image.path && inTauri()) {
    return <img src={convertFileSrc(image.path)} alt="" width={size} height={size} style={{ borderRadius: "50%", objectFit: "cover", flex: "none" }} />;
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
          <TextInput id="rel-type" list="rel-suggestions" value={type} onChange={(e) => setType(e.target.value)} placeholder="father of" autoFocus />
          <datalist id="rel-suggestions">
            {RELATION_SUGGESTIONS.map((s) => (
              <option key={s} value={s} />
            ))}
          </datalist>
        </Field>
        <Field label="Character">
          <Select value={to} onChange={setTo} ariaLabel="To character" options={opts} />
        </Field>
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
        <p style={{ marginTop: 0 }}>The character moves to Recently Deleted. You can restore it or press Ctrl+Z.</p>
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

/** Relationship map (mock 069): an optional visual helper, not an analysis system. */
function RelationshipMap({ chars, onBack, readOnly }: { chars: StoryCharacterDto[]; onBack: () => void; readOnly: boolean }) {
  const rels = useRelationships();
  const [add, setAdd] = useState(false);
  const list = rels.data ?? [];
  const nodes = chars.filter((c) => !c.archived && list.some((r) => r.fromCharacterId === c.id || r.toCharacterId === c.id));
  const W = 640;
  const H = 360;
  const R = Math.min(W, H) / 2 - 50;
  const pos = new Map(nodes.map((n, i) => [n.id, { x: W / 2 + R * Math.cos((2 * Math.PI * i) / nodes.length - Math.PI / 2), y: H / 2 + R * Math.sin((2 * Math.PI * i) / nodes.length - Math.PI / 2) }]));
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
      <p className="muted" style={{ fontSize: 12.5 }}>Optional visual helper — not a character-analysis system.</p>
      {list.length === 0 ? (
        <EmptyState title="No relationships yet.">Add a relationship such as “father of” or “colleague of”.</EmptyState>
      ) : (
        <>
          <svg viewBox={`0 0 ${W} ${H}`} width="100%" style={{ maxHeight: 380 }} role="img" aria-label="Relationship map">
            {list.map((r) => {
              const a = pos.get(r.fromCharacterId);
              const b = pos.get(r.toCharacterId);
              if (!a || !b) return null;
              return (
                <g key={r.id}>
                  <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke="var(--line2)" strokeWidth={2} />
                  <text x={(a.x + b.x) / 2} y={(a.y + b.y) / 2 - 4} textAnchor="middle" fontSize={11} fill="var(--ink2)">
                    {r.relationshipType}
                  </text>
                </g>
              );
            })}
            {nodes.map((n) => {
              const p = pos.get(n.id)!;
              return (
                <g key={n.id}>
                  <circle cx={p.x} cy={p.y} r={26} fill="var(--accent-soft)" stroke="var(--accent)" />
                  <text x={p.x} y={p.y + 4} textAnchor="middle" fontSize={11} fontWeight={700} fill="var(--ink)">
                    {n.name.toUpperCase().slice(0, 10)}
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
