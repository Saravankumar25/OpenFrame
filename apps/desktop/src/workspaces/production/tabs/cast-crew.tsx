// Cast & Crew (FSD §30, §100; UX §3.25, mocks 118–120):
// "Who is involved and what role do they have? A small-team directory — no
// payroll, no contracts." The Character is the story object; the cast
// member is the real person attached to it.

import { useMemo, useState } from "react";
import { ImagePlus, Plus, UserRound, X } from "lucide-react";
import { call } from "../../../ipc/client";
import { reportError, useCommand, useOp } from "../../../ipc/query";
import type { ProductionCastDirectory } from "../../../ipc/generated/ProductionCastDirectory";
import type { ProductionCastMemberDto } from "../../../ipc/generated/ProductionCastMemberDto";
import type { ProductionCastCharacterDto } from "../../../ipc/generated/ProductionCastCharacterDto";
import type { ProductionCrewMemberDto } from "../../../ipc/generated/ProductionCrewMemberDto";
import {
  Banner,
  Button,
  Checkbox,
  Chip,
  ConfirmDialog,
  ContextMenu,
  Dialog,
  Drawer,
  EmptyState,
  Field,
  IconButton,
  PageHeader,
  Segmented,
  Select,
  Skeleton,
  TextArea,
  TextInput,
} from "../../../design-system";
import { PROD_TABLES, assetUrl, useCanEdit } from "../../../api/production";
import { useNav } from "../../../app/stores";
import { toast } from "../../../app/toast";
import { BufferedField, SceneChips, pickImages } from "../components/fields";
import type { ProductionCastAssignArgs } from "../../../ipc/generated/ProductionCastAssignArgs";
import type { ProductionCastCreateArgs } from "../../../ipc/generated/ProductionCastCreateArgs";
import type { ProductionCastUpdateArgs } from "../../../ipc/generated/ProductionCastUpdateArgs";
import type { ProductionCrewCreateArgs } from "../../../ipc/generated/ProductionCrewCreateArgs";
import type { ProductionCrewUpdateArgs } from "../../../ipc/generated/ProductionCrewUpdateArgs";

export const tab = { id: "cast-crew", label: "Cast & Crew", order: 30 };

const DEPARTMENTS = ["Direction", "Camera", "Sound", "Art", "Costume", "Makeup", "Production", "Editing"];
const PEOPLE_TABLES = [...PROD_TABLES, "project_member"];

function Avatar({ member, size = 30 }: { member: ProductionCastMemberDto; size?: number }) {
  const url = assetUrl(member.photo);
  return url ? (
    <img src={url} alt="" style={{ width: size, height: size, borderRadius: "50%", objectFit: "cover", flex: "none" }} />
  ) : (
    <span className="photo face" aria-hidden style={{ width: size, height: size, borderRadius: "50%", display: "inline-flex", alignItems: "center", justifyContent: "center", color: "#fff9", flex: "none" }}>
      <UserRound size={size * 0.55} />
    </span>
  );
}

export default function CastCrewTab() {
  const route = useNav((s) => s.route);
  const canEdit = useCanEdit();
  const castParam = route.params?.castMemberId ?? null;
  const crewParam = route.params?.crewMemberId ?? null;
  const [view, setView] = useState<"cast" | "crew">(crewParam ? "crew" : "cast");
  const [selectedCast, setSelectedCast] = useState<string | null>(castParam);
  const [selectedCrew, setSelectedCrew] = useState<string | null>(crewParam);
  const [lastParams, setLastParams] = useState(`${castParam}|${crewParam}`);
  if (`${castParam}|${crewParam}` !== lastParams) {
    setLastParams(`${castParam}|${crewParam}`);
    if (castParam) {
      setView("cast");
      setSelectedCast(castParam);
    } else if (crewParam) {
      setView("crew");
      setSelectedCrew(crewParam);
    }
  }
  const [search, setSearch] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [assign, setAssign] = useState<{ member: ProductionCastMemberDto | null; character: string | null } | null>(null);
  const [addingCrew, setAddingCrew] = useState(false);
  const [deleting, setDeleting] = useState<{ kind: "cast" | "crew"; id: string; name: string } | null>(null);

  const dir = useOp<ProductionCastDirectory>("cast.list", { includeArchived: showArchived || !!selectedCast }, PEOPLE_TABLES);
  const crew = useOp<ProductionCrewMemberDto[]>("crew.list", { includeArchived: showArchived || !!selectedCrew }, PEOPLE_TABLES);
  const q = search.trim().toLowerCase();

  const members = (dir.data?.members ?? []).filter(
    (m) => (showArchived || !m.archived || m.id === selectedCast) && (!q || `${m.personName} ${m.characterName ?? ""} ${m.contact ?? ""} ${m.availabilityNotes ?? ""}`.toLowerCase().includes(q)),
  );
  const unassigned = (dir.data?.characters ?? []).filter((c) => !c.primaryCastId && (!q || c.name.toLowerCase().includes(q)));
  const crewList = (crew.data ?? []).filter(
    (c) => (showArchived || !c.archived || c.id === selectedCrew) && (!q || `${c.personName} ${c.role} ${c.department ?? ""} ${c.contact ?? ""}`.toLowerCase().includes(q)),
  );
  const castMember = (dir.data?.members ?? []).find((m) => m.id === selectedCast) ?? null;
  const crewMember = (crew.data ?? []).find((c) => c.id === selectedCrew) ?? null;

  const archive = (kind: "cast" | "crew", id: string, archived: boolean, name: string) =>
    void call(`${kind}.set_archived`, { id, archived })
      .then(() => toast.undoable(archived ? `Archived ${name}` : `Restored ${name} from the archive`))
      .catch(reportError);

  return (
    <div>
      <PageHeader
        title="Cast & Crew"
        sub="Who is involved and what role do they have? A small-team directory — no payroll, no contracts."
        actions={
          canEdit && (
            <Button variant="primary" icon={<Plus size={15} />} onClick={() => (view === "cast" ? setAssign({ member: null, character: null }) : setAddingCrew(true))}>
              Add Person
            </Button>
          )
        }
      />
      <div className="toolbar">
        <Segmented ariaLabel="Directory" value={view} onChange={setView} options={[{ value: "cast", label: "Cast" }, { value: "crew", label: "Crew" }]} />
        <TextInput aria-label="Search" placeholder="Search" value={search} onChange={(e) => setSearch(e.target.value)} style={{ flex: "0 0 230px" }} />
        <span className="sp" />
        <Checkbox checked={showArchived} onChange={setShowArchived} label="Show archived" />
      </div>

      {view === "cast" ? (
        dir.isLoading ? (
          <Skeleton h={140} />
        ) : dir.error ? (
          <Banner tone="err">{dir.error.message}</Banner>
        ) : members.length === 0 && unassigned.length === 0 ? (
          <EmptyState title="No cast yet." actions={canEdit && <Button variant="primary" onClick={() => setAssign({ member: null, character: null })}>Add Person</Button>}>
            Characters from the Production Source appear here automatically. Add the actor who plays each one, with a contact and availability notes.
          </EmptyState>
        ) : (
          <div className="card">
            <table className="tbl">
              <thead>
                <tr>
                  <th>Person</th>
                  <th>Character</th>
                  <th>Contact</th>
                  <th>Availability note</th>
                  <th>Scenes</th>
                </tr>
              </thead>
              <tbody>
                {members.map((m) => (
                  <ContextMenu
                    key={m.id}
                    items={[
                      { label: "Open", onSelect: () => setSelectedCast(m.id) },
                      { label: "Assign Character…", disabled: !canEdit, onSelect: () => setAssign({ member: m, character: m.characterName }) },
                      { label: m.archived ? "Restore from Archive" : "Archive", disabled: !canEdit, onSelect: () => archive("cast", m.id, !m.archived, m.personName) },
                      { label: "Delete…", danger: true, separatorBefore: true, disabled: !canEdit, onSelect: () => setDeleting({ kind: "cast", id: m.id, name: m.personName }) },
                    ]}
                  >
                    <tr className={m.id === selectedCast ? "sel" : undefined} tabIndex={0} onClick={() => setSelectedCast(m.id)} onKeyDown={(e) => e.key === "Enter" && setSelectedCast(m.id)} style={{ cursor: "pointer", opacity: m.archived ? 0.55 : 1 }}>
                      <td>
                        <span className="row">
                          <Avatar member={m} />
                          <b>{m.personName}</b>
                          {!m.isPrimary && <Chip tone="p">Alternate</Chip>}
                          {m.archived && <Chip>Archived</Chip>}
                        </span>
                      </td>
                      <td>{m.characterName ?? "—"}</td>
                      <td className="muted">{m.contact ?? "—"}</td>
                      <td className="muted">{m.availabilityNotes ?? "—"}</td>
                      <td>{m.scenes.filter((s) => s.inSource).length}</td>
                    </tr>
                  </ContextMenu>
                ))}
                {unassigned.map((c) => (
                  <tr key={`char-${c.name}`}>
                    <td>
                      <span className="row">
                        <span className="photo face" aria-hidden style={{ width: 30, height: 30, borderRadius: "50%" }} />
                        <b className="muted">— unassigned —</b>
                      </span>
                    </td>
                    <td>{c.name}</td>
                    <td className="muted">—</td>
                    <td>
                      <span className="row">
                        <Chip tone="y">Needs casting</Chip>
                        {canEdit && <Button size="xs" onClick={() => setAssign({ member: null, character: c.name })}>Assign</Button>}
                      </span>
                    </td>
                    <td>{c.scenes.filter((s) => s.inSource).length}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )
      ) : crew.isLoading ? (
        <Skeleton h={140} />
      ) : crew.error ? (
        <Banner tone="err">{crew.error.message}</Banner>
      ) : crewList.length === 0 ? (
        <EmptyState title="No crew yet." actions={canEdit && <Button variant="primary" onClick={() => setAddingCrew(true)}>Add Person</Button>}>
          Add the people working on the film with their role and department. This is a production directory, not HR software.
        </EmptyState>
      ) : (
        <>
          <div className="card">
            <table className="tbl">
              <thead>
                <tr>
                  <th>Person</th>
                  <th>Role</th>
                  <th>Department</th>
                  <th>Contact</th>
                </tr>
              </thead>
              <tbody>
                {crewList.map((c) => (
                  <ContextMenu
                    key={c.id}
                    items={[
                      { label: "Open", onSelect: () => setSelectedCrew(c.id) },
                      { label: c.archived ? "Restore from Archive" : "Archive", disabled: !canEdit, onSelect: () => archive("crew", c.id, !c.archived, c.personName) },
                      { label: "Delete…", danger: true, separatorBefore: true, disabled: !canEdit, onSelect: () => setDeleting({ kind: "crew", id: c.id, name: c.personName }) },
                    ]}
                  >
                    <tr className={c.id === selectedCrew ? "sel" : undefined} tabIndex={0} onClick={() => setSelectedCrew(c.id)} onKeyDown={(e) => e.key === "Enter" && setSelectedCrew(c.id)} style={{ cursor: "pointer", opacity: c.archived ? 0.55 : 1 }}>
                      <td><b>{c.personName}</b> {c.archived && <Chip>Archived</Chip>}</td>
                      <td>{c.role}</td>
                      <td>{c.department ?? "—"}</td>
                      <td className="muted">{c.contact ?? "—"}</td>
                    </tr>
                  </ContextMenu>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
      {view === "crew" && <div className="xs muted" style={{ marginTop: 8 }}>Departments: {DEPARTMENTS.join(" · ")}. This is a production directory, not HR software.</div>}

      {view === "cast" && castMember && (
        <CastDrawer
          key={castMember.id}
          member={castMember}
          canEdit={canEdit}
          onClose={() => setSelectedCast(null)}
          onAssign={() => setAssign({ member: castMember, character: castMember.characterName })}
          onArchive={() => archive("cast", castMember.id, !castMember.archived, castMember.personName)}
          onDelete={() => setDeleting({ kind: "cast", id: castMember.id, name: castMember.personName })}
        />
      )}
      {view === "crew" && crewMember && (
        <CrewDrawer
          key={crewMember.id}
          member={crewMember}
          canEdit={canEdit}
          onClose={() => setSelectedCrew(null)}
          onArchive={() => archive("crew", crewMember.id, !crewMember.archived, crewMember.personName)}
          onDelete={() => setDeleting({ kind: "crew", id: crewMember.id, name: crewMember.personName })}
        />
      )}
      {assign && (
        <AssignDialog
          member={assign.member}
          initialCharacter={assign.character}
          characters={dir.data?.characters ?? []}
          members={dir.data?.members ?? []}
          onClose={() => setAssign(null)}
        />
      )}
      {addingCrew && <AddCrewDialog onClose={() => setAddingCrew(false)} onCreated={(id) => setSelectedCrew(id)} />}
      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(v) => !v && setDeleting(null)}
        title={`Remove ${deleting?.name ?? ""} from the ${deleting?.kind === "crew" ? "crew" : "cast"}?`}
        confirmLabel="Remove"
        danger
        onConfirm={() => {
          const d = deleting;
          if (!d) return;
          void call(`${d.kind}.delete`, { id: d.id })
            .then(() => {
              toast.undoable(`Removed ${d.name}. Restore from Recently Deleted if needed.`);
              setDeleting(null);
              if (d.kind === "cast") setSelectedCast(null);
              else setSelectedCrew(null);
            })
            .catch(reportError);
        }}
      >
        <p className="sm" style={{ marginTop: 0 }}>
          Exported documents and call sheets that already mention this person keep their copies. Archiving is the better choice if they may return.
        </p>
      </ConfirmDialog>
    </div>
  );
}

function CastDrawer({ member, canEdit, onClose, onAssign, onArchive, onDelete }: {
  member: ProductionCastMemberDto;
  canEdit: boolean;
  onClose: () => void;
  onAssign: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  const update = useCommand<ProductionCastUpdateArgs, ProductionCastMemberDto>("cast.update");
  const setPhoto = async (remove: boolean) => {
    try {
      const path = remove ? null : (await pickImages(false))[0];
      if (!remove && !path) return;
      await call("cast.set_photo", { id: member.id, path });
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="Cast member"
      title={member.personName}
      footer={
        <>
          {canEdit && <Button variant="ghost" onClick={onDelete}>Delete</Button>}
          <span className="grow" />
          {canEdit && <Button onClick={onArchive}>{member.archived ? "Restore" : "Archive"}</Button>}
        </>
      }
    >
      <div className="row" style={{ marginBottom: 10 }}>
        <Avatar member={member} size={56} />
        {canEdit && (
          <>
            <Button size="sm" icon={<ImagePlus size={14} />} onClick={() => void setPhoto(false)}>{member.photo ? "Change Photo" : "Add Photo"}</Button>
            {member.photo && <IconButton label="Remove photo" onClick={() => void setPhoto(true)}><X size={14} /></IconButton>}
          </>
        )}
      </div>
      <Field label="Character">
        <div className="row">
          <b>{member.characterName ?? "—"}</b>
          <Chip tone={member.isPrimary ? "g" : "p"}>{member.isPrimary ? "Primary" : "Alternate"}</Chip>
          <span className="grow" />
          {canEdit && <Button size="xs" onClick={onAssign}>Assign Character…</Button>}
        </div>
      </Field>
      <BufferedField id="cm-name" label="Person name" required value={member.personName} disabled={!canEdit} maxLength={200} onCommit={(v) => update.mutate({ id: member.id, personName: v })} />
      <BufferedField id="cm-contact" label="Contact" value={member.contact} disabled={!canEdit} maxLength={500} onCommit={(v) => update.mutate({ id: member.id, contact: v })} />
      <BufferedField
        id="cm-avail"
        label="Availability notes"
        multiline
        value={member.availabilityNotes}
        disabled={!canEdit}
        maxLength={4000}
        placeholder="e.g. Free Jun 9–20, weekends only"
        hint="Notes only — referenced when scheduling."
        onCommit={(v) => update.mutate({ id: member.id, availabilityNotes: v })}
      />
      <BufferedField id="cm-notes" label="Notes" multiline value={member.notes} disabled={!canEdit} maxLength={20000} onCommit={(v) => update.mutate({ id: member.id, notes: v })} />
      <Field label="Scenes">
        <SceneChips scenes={member.scenes} empty="No scenes in the Production Source mention this character yet." />
      </Field>
    </Drawer>
  );
}

/** "Assign character" (mock 119). */
function AssignDialog({ member, initialCharacter, characters, members, onClose }: {
  member: ProductionCastMemberDto | null;
  initialCharacter: string | null;
  characters: ProductionCastCharacterDto[];
  members: ProductionCastMemberDto[];
  onClose: () => void;
}) {
  const [person, setPerson] = useState(member?.personName ?? "");
  const [character, setCharacter] = useState<string | null>(initialCharacter);
  const [newCharacter, setNewCharacter] = useState("");
  const chosen = newCharacter.trim() || character;
  const current = characters.find((c) => c.name === chosen);
  const hasOtherPrimary = !!current?.primaryCastId && current.primaryCastId !== member?.id;
  const [primary, setPrimary] = useState<"primary" | "alternate" | null>(null);
  const role = primary ?? (hasOtherPrimary ? "alternate" : "primary");
  const byId = useMemo(() => new Map(members.map((m) => [m.id, m.personName])), [members]);
  const create = useCommand<ProductionCastCreateArgs, ProductionCastMemberDto>("cast.create", {
    onSuccess: (m) => {
      toast.undoable(`${m.personName} plays ${m.characterName}`);
      onClose();
    },
  });
  const assign = useCommand<ProductionCastAssignArgs, ProductionCastMemberDto>("cast.assign", {
    onSuccess: (m) => {
      toast.undoable(`${m.personName} plays ${m.characterName}`);
      onClose();
    },
  });
  const submit = () => {
    if (!chosen || !person.trim()) return;
    const storyId = current?.storyCharacterId ?? null;
    const target = storyId ? { characterId: storyId } : { characterName: chosen };
    if (member) assign.mutate({ id: member.id, ...target, primary: role === "primary" });
    else create.mutate({ personName: person.trim(), ...target, primary: role === "primary" });
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={member ? `Assign a character to ${member.personName}` : "Add Person"}
      sub="Link the real person to the character they play."
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!chosen || !person.trim() || create.isPending || assign.isPending} onClick={submit}>Assign</Button>
        </>
      }
    >
      <Field label="Person" required htmlFor="as-person">
        <TextInput id="as-person" autoFocus={!member} disabled={!!member} value={person} maxLength={200} onChange={(e) => setPerson(e.target.value)} placeholder="Actor's name" />
      </Field>
      <Field label="Choose a character">
        {characters.length === 0 ? (
          <div className="sm muted">No characters found in the Production Source yet. Create one below.</div>
        ) : (
          <div role="radiogroup" aria-label="Characters" className="card" style={{ maxHeight: 220, overflow: "auto" }}>
            {characters.map((c) => {
              const on = !newCharacter.trim() && character === c.name;
              const n = c.scenes.filter((s) => s.inSource).length;
              return (
                <button key={c.name} type="button" role="radio" aria-checked={on} className={`li${on ? " sel" : ""}`} style={{ width: "100%", textAlign: "left", border: 0, background: on ? undefined : "transparent" }} onClick={() => { setCharacter(c.name); setNewCharacter(""); setPrimary(null); }}>
                  <b className="grow">{c.name}</b>
                  <span className="xs muted">
                    {n} scene{n === 1 ? "" : "s"} · {c.primaryCastId ? `played by ${byId.get(c.primaryCastId) ?? "someone"}` : "no actor"}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </Field>
      <Field label="Create a new character" htmlFor="as-new" hint="For a character that isn't in the script yet.">
        <TextInput id="as-new" value={newCharacter} maxLength={200} onChange={(e) => { setNewCharacter(e.target.value); setPrimary(null); }} placeholder="e.g. THE STATION MASTER" />
      </Field>
      <Field label="Casting">
        <Segmented
          ariaLabel="Primary or alternate"
          value={role}
          onChange={(v) => setPrimary(v)}
          options={[{ value: "primary", label: "Primary actor" }, { value: "alternate", label: "Alternate / double" }]}
        />
      </Field>
      {role === "primary" && hasOtherPrimary && (
        <Banner tone="info">{byId.get(current!.primaryCastId!) ?? "The current actor"} becomes an alternate for {chosen}.</Banner>
      )}
      <div className="xs muted" style={{ marginTop: 8 }}>One character has one primary actor by default. Alternates or double-casting can be added when needed.</div>
    </Dialog>
  );
}

function CrewDrawer({ member, canEdit, onClose, onArchive, onDelete }: {
  member: ProductionCrewMemberDto;
  canEdit: boolean;
  onClose: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  const update = useCommand<ProductionCrewUpdateArgs, ProductionCrewMemberDto>("crew.update");
  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="Crew member"
      title={member.personName}
      footer={
        <>
          {canEdit && <Button variant="ghost" onClick={onDelete}>Delete</Button>}
          <span className="grow" />
          {canEdit && <Button onClick={onArchive}>{member.archived ? "Restore" : "Archive"}</Button>}
        </>
      }
    >
      <BufferedField id="cr-name" label="Person name" required value={member.personName} disabled={!canEdit} maxLength={200} onCommit={(v) => update.mutate({ id: member.id, personName: v })} />
      <BufferedField id="cr-role" label="Role" required value={member.role} disabled={!canEdit} maxLength={120} onCommit={(v) => update.mutate({ id: member.id, role: v })} />
      <Field label="Department" htmlFor="cr-dept">
        <Select
          id="cr-dept"
          value={member.department ?? ""}
          options={[{ value: "", label: "—" }, ...Array.from(new Set([...DEPARTMENTS, ...(member.department ? [member.department] : [])])).map((d) => ({ value: d, label: d }))]}
          onChange={(v) => canEdit && update.mutate({ id: member.id, department: v })}
        />
      </Field>
      <BufferedField id="cr-contact" label="Contact" value={member.contact} disabled={!canEdit} maxLength={500} onCommit={(v) => update.mutate({ id: member.id, contact: v })} />
      <BufferedField id="cr-notes" label="Notes" multiline value={member.notes} disabled={!canEdit} maxLength={20000} onCommit={(v) => update.mutate({ id: member.id, notes: v })} />
    </Drawer>
  );
}

function AddCrewDialog({ onClose, onCreated }: { onClose: () => void; onCreated: (id: string) => void }) {
  const [person, setPerson] = useState("");
  const [role, setRole] = useState("");
  const [department, setDepartment] = useState("");
  const [contact, setContact] = useState("");
  const [notes, setNotes] = useState("");
  const create = useCommand<ProductionCrewCreateArgs, ProductionCrewMemberDto>("crew.create", {
    onSuccess: (c) => {
      toast.undoable(`Added ${c.personName} to the crew`);
      onCreated(c.id);
      onClose();
    },
  });
  const submit = () =>
    person.trim() &&
    role.trim() &&
    create.mutate({ personName: person.trim(), role: role.trim(), department: department || null, contact: contact.trim() || null, notes: notes.trim() || null });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title="Add Person"
      sub="A crew member with their role. No payroll, no contracts."
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!person.trim() || !role.trim() || create.isPending} onClick={submit}>Add to Crew</Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); submit(); }}>
        <Field label="Person name" required htmlFor="nc-name">
          <TextInput id="nc-name" autoFocus value={person} maxLength={200} onChange={(e) => setPerson(e.target.value)} />
        </Field>
        <Field label="Role" required htmlFor="nc-role">
          <TextInput id="nc-role" value={role} maxLength={120} onChange={(e) => setRole(e.target.value)} placeholder="e.g. Director of Photography" />
        </Field>
        <Field label="Department" htmlFor="nc-dept">
          <Select id="nc-dept" value={department} options={[{ value: "", label: "—" }, ...DEPARTMENTS.map((d) => ({ value: d, label: d }))]} onChange={setDepartment} />
        </Field>
        <Field label="Contact" htmlFor="nc-contact">
          <TextInput id="nc-contact" value={contact} maxLength={500} onChange={(e) => setContact(e.target.value)} />
        </Field>
        <Field label="Notes" htmlFor="nc-notes">
          <TextArea id="nc-notes" rows={2} value={notes} maxLength={20000} onChange={(e) => setNotes(e.target.value)} />
        </Field>
      </form>
    </Dialog>
  );
}
