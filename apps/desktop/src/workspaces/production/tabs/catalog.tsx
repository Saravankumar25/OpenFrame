// Catalog (FSD §28, §98, §143–144, §163; UX §3.23, mocks 113–114):
// "What things does this film need? Reusable items — not one huge spreadsheet."

import { useMemo, useState } from "react";
import { ImagePlus, Plus, X } from "lucide-react";
import { call } from "../../../ipc/client";
import { reportError, useCommand, useOp } from "../../../ipc/query";
import type { CatalogItemDto } from "../../../ipc/generated/CatalogItemDto";
import type { CatalogStatus } from "../../../ipc/generated/CatalogStatus";
import type { BreakdownCategory } from "../../../ipc/generated/BreakdownCategory";
import type { CatalogReplaceResult } from "../../../ipc/generated/CatalogReplaceResult";
import type { CatalogMatchDto } from "../../../ipc/generated/CatalogMatchDto";
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
  Menu,
  PageHeader,
  Select,
  Skeleton,
  TextArea,
  TextInput,
} from "../../../design-system";
import { CATALOG_STATUSES, CATEGORIES, CATEGORY_SHORT, PROD_TABLES, assetUrl, catalogStatusTone, useCanEdit } from "../../../api/production";
import { useNav } from "../../../app/stores";
import { toast } from "../../../app/toast";
import { BufferedField, SceneChips, pickImages } from "../components/fields";
import { CatalogExportButton } from "../../../features/export/buttons";
import type { CatalogCreateArgs } from "../../../ipc/generated/CatalogCreateArgs";
import type { CatalogReplaceArgs } from "../../../ipc/generated/CatalogReplaceArgs";
import type { CatalogUpdateArgs } from "../../../ipc/generated/CatalogUpdateArgs";

export const tab = { id: "catalog", label: "Catalog", order: 10 };

type Filter = "all" | BreakdownCategory;
const MAIN_FILTERS: { value: Filter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "Props", label: "Props" },
  { value: "Wardrobe", label: "Wardrobe" },
  { value: "Vehicles", label: "Vehicles" },
  { value: "Cast", label: "Cast" },
  { value: "Location / Set", label: "Locations" },
];
const MORE = CATEGORIES.filter((c) => !MAIN_FILTERS.some((f) => f.value === c));
const STATUS_OPTIONS = CATALOG_STATUSES.map((s) => ({ value: s, label: s }));
const CATEGORY_OPTIONS = CATEGORIES.map((c) => ({ value: c, label: c }));

export default function CatalogTab() {
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const canEdit = useCanEdit();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [showArchived, setShowArchived] = useState(false);
  const paramId = route.params?.catalogItemId ?? null;
  const [selected, setSelected] = useState<string | null>(paramId);
  const [lastParam, setLastParam] = useState(paramId);
  if (paramId !== lastParam) {
    setLastParam(paramId);
    if (paramId) {
      setSelected(paramId);
      setShowArchived(true);
    }
  }
  const [adding, setAdding] = useState(false);
  const [archiving, setArchiving] = useState<CatalogItemDto | null>(null);
  const [replacing, setReplacing] = useState<CatalogItemDto | null>(null);
  const [deleting, setDeleting] = useState<CatalogItemDto | null>(null);

  const args = useMemo(
    () => ({ category: filter === "all" ? null : filter, search: search.trim() || null, includeArchived: showArchived }),
    [filter, search, showArchived],
  );
  const items = useOp<CatalogItemDto[]>("catalog.list", args, PROD_TABLES);
  const list = items.data ?? [];
  const current = list.find((i) => i.id === selected) ?? null;
  const detail = useOp<CatalogItemDto>("catalog.get", { id: selected }, PROD_TABLES, { enabled: !!selected && !current });
  const item = current ?? (selected ? detail.data ?? null : null);

  const setArchived = (i: CatalogItemDto, archived: boolean) =>
    void call("catalog.set_archived", { id: i.id, archived })
      .then(() => {
        toast.undoable(archived ? `Archived “${i.name}”` : `Restored “${i.name}” from the archive`);
        setArchiving(null);
      })
      .catch(reportError);

  const menuFor = (i: CatalogItemDto) => [
    { label: "Open", onSelect: () => setSelected(i.id) },
    { label: "Open Scenes", disabled: i.usedIn.length === 0, onSelect: () => go({ workspace: "breakdown", params: { sceneId: i.usedIn[0].sceneId } }) },
    { label: i.archived ? "Restore from Archive" : "Archive…", disabled: !canEdit, onSelect: () => (i.archived ? setArchived(i, false) : setArchiving(i)) },
    { label: "Delete…", danger: true, separatorBefore: true, disabled: !canEdit, onSelect: () => setDeleting(i) },
  ];

  const moreLabel = MORE.includes(filter as BreakdownCategory) ? `${filter} ▾` : "More ▾";

  return (
    <div>
      <PageHeader
        title="Catalog"
        sub="What things does this film need? Reusable items — not one huge spreadsheet."
        actions={
          <>
            <CatalogExportButton />
            {canEdit && <Button variant="primary" icon={<Plus size={15} />} onClick={() => setAdding(true)}>Add Item</Button>}
          </>
        }
      />
      <div className="toolbar">
        <TextInput aria-label="Search catalog" placeholder="Search catalog" value={search} onChange={(e) => setSearch(e.target.value)} style={{ flex: "0 0 230px" }} />
        <div className="seg" role="radiogroup" aria-label="Category">
          {MAIN_FILTERS.map((f) => (
            <button key={f.value} type="button" role="radio" aria-checked={filter === f.value} className={filter === f.value ? "on" : undefined} onClick={() => setFilter(f.value)}>
              {f.label}
            </button>
          ))}
          <Menu
            trigger={
              <button type="button" className={MORE.includes(filter as BreakdownCategory) ? "on" : undefined} aria-label="More categories">
                {moreLabel}
              </button>
            }
            items={MORE.map((c) => ({ label: c, onSelect: () => setFilter(c) }))}
          />
        </div>
        <span className="sp" />
        <Checkbox checked={showArchived} onChange={setShowArchived} label="Show archived" />
      </div>
      {items.isLoading ? (
        <Skeleton h={160} />
      ) : items.error ? (
        <Banner tone="err">{items.error.message}</Banner>
      ) : list.length === 0 ? (
        search || filter !== "all" ? (
          <div className="sm muted" style={{ padding: 12 }}>No catalog items match.</div>
        ) : (
          <EmptyState title="The catalog is empty." actions={canEdit && <Button variant="primary" onClick={() => setAdding(true)}>Add Item</Button>}>
            Confirmed breakdown items will appear here, or add one manually.
          </EmptyState>
        )
      ) : (
        <div className="card" style={{ maxWidth: 760 }}>
          <table className="tbl">
            <thead>
              <tr>
                <th>Name</th>
                <th>Category</th>
                <th>Status</th>
                <th>Used in</th>
              </tr>
            </thead>
            <tbody>
              {list.map((i) => (
                <ContextMenu key={i.id} items={menuFor(i)}>
                  <tr
                    className={i.id === selected ? "sel" : undefined}
                    tabIndex={0}
                    aria-selected={i.id === selected}
                    onClick={() => setSelected(i.id)}
                    onKeyDown={(e) => e.key === "Enter" && setSelected(i.id)}
                    style={{ cursor: "pointer", opacity: i.archived ? 0.55 : 1 }}
                  >
                    <td>
                      <b>{i.name}</b> {i.archived && <Chip>Archived</Chip>}
                    </td>
                    <td>{CATEGORY_SHORT[i.category]}</td>
                    <td>
                      <Chip tone={catalogStatusTone(i.status)}>{i.status}</Chip>
                    </td>
                    <td className="muted">{i.usedInLabel}</td>
                  </tr>
                </ContextMenu>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {item && (
        <CatalogDrawer
          key={item.id}
          item={item}
          canEdit={canEdit}
          onClose={() => setSelected(null)}
          onArchive={() => (item.archived ? setArchived(item, false) : setArchiving(item))}
          onDelete={() => setDeleting(item)}
        />
      )}
      {adding && <AddItemDialog onClose={() => setAdding(false)} onCreated={(id) => setSelected(id)} initialCategory={filter === "all" ? "Props" : filter} />}
      <ConfirmDialog
        open={!!archiving}
        onOpenChange={(v) => !v && setArchiving(null)}
        title={`Archive “${archiving?.name ?? ""}”?`}
        confirmLabel="Archive Item"
        onConfirm={() => archiving && setArchived(archiving, true)}
      >
        {archiving && (
          <>
            <p className="sm" style={{ marginTop: 0 }}>
              {archiving.usedIn.length > 0
                ? `This item is still used in ${archiving.usedIn.length} scene${archiving.usedIn.length === 1 ? "" : "s"} (${archiving.usedIn.map((s) => s.number ?? "removed").join(", ")}). Archiving keeps those associations readable but the item will no longer be offered for new selections.`
                : "The item will no longer be offered for new selections. You can restore it at any time."}
            </p>
            {archiving.usedIn.length > 0 && canEdit && (
              <div className="card pad">
                <div className="h4">Instead</div>
                <div className="sm">You can also replace it in selected scenes with another catalog item without deleting it.</div>
                <div style={{ marginTop: 8 }}>
                  <Button size="sm" onClick={() => { setReplacing(archiving); setArchiving(null); }}>Replace in Scenes…</Button>
                </div>
              </div>
            )}
          </>
        )}
      </ConfirmDialog>
      {replacing && <ReplaceDialog item={replacing} onClose={() => setReplacing(null)} />}
      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(v) => !v && setDeleting(null)}
        title={`Delete “${deleting?.name ?? ""}”?`}
        confirmLabel="Delete"
        danger
        onConfirm={() => {
          const d = deleting;
          if (!d) return;
          void call("catalog.delete", { id: d.id })
            .then(() => {
              toast.undoable(`Deleted “${d.name}”. Restore it from Recently Deleted if needed.`);
              setDeleting(null);
              setSelected(null);
            })
            .catch(reportError);
        }}
      >
        <p className="sm" style={{ marginTop: 0 }}>
          {deleting && deleting.usedIn.length > 0
            ? `It is used in ${deleting.usedIn.length} scene${deleting.usedIn.length === 1 ? "" : "s"}. Those scenes keep the entry, marked “Catalog item removed”, until you restore it. Archiving is usually the better choice.`
            : "The item moves to Recently Deleted, where you can restore it."}
        </p>
      </ConfirmDialog>
    </div>
  );
}

function CatalogDrawer({ item, canEdit, onClose, onArchive, onDelete }: {
  item: CatalogItemDto;
  canEdit: boolean;
  onClose: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  const go = useNav((s) => s.go);
  const [alias, setAlias] = useState("");
  const update = useCommand<CatalogUpdateArgs, CatalogItemDto>("catalog.update");
  const patch = (fields: Record<string, string | null>) => update.mutate({ id: item.id, ...fields });
  const setImage = async (remove: boolean) => {
    try {
      const path = remove ? null : (await pickImages(false))[0];
      if (!remove && !path) return;
      await call("catalog.set_image", { id: item.id, path });
    } catch (e) {
      reportError(e);
    }
  };
  const addAlias = () => {
    const a = alias.trim();
    if (!a) return;
    void call("catalog.add_alias", { id: item.id, alias: a }).then(() => setAlias("")).catch(reportError);
  };
  const img = assetUrl(item.image);
  return (
    <Drawer
      open
      onClose={onClose}
      width="n"
      typeLabel="Catalog item"
      title={item.name}
      footer={
        <>
          {canEdit && <Button variant="ghost" onClick={onDelete}>Delete</Button>}
          <span className="grow" />
          {canEdit && <Button onClick={onArchive}>{item.archived ? "Restore" : "Archive"}</Button>}
          <Button disabled={item.usedIn.length === 0} onClick={() => go({ workspace: "breakdown", params: { sceneId: item.usedIn[0].sceneId } })}>
            Open Scenes
          </Button>
        </>
      }
    >
      <div style={{ overflow: "auto", maxHeight: "100%" }}>
        {item.archived && <div style={{ marginBottom: 8 }}><Banner tone="info">Archived — kept readable in its scenes, not offered for new selections.</Banner></div>}
        <BufferedField id="ci-name" label="Name" required value={item.name} disabled={!canEdit} maxLength={200} onCommit={(v) => patch({ name: v })} />
        <Field label="Category">
          <div className="sm">{item.category}</div>
        </Field>
        <Field label="Status" htmlFor="ci-status">
          <Select<CatalogStatus> id="ci-status" value={item.status} options={STATUS_OPTIONS} onChange={(v) => canEdit && patch({ status: v })} />
        </Field>
        <BufferedField id="ci-desc" label="Description" multiline value={item.description} disabled={!canEdit} maxLength={4000} onCommit={(v) => patch({ description: v })} />
        <BufferedField id="ci-notes" label="Notes" multiline placeholder="Add notes…" value={item.notes} disabled={!canEdit} maxLength={20000} onCommit={(v) => patch({ notes: v })} />
        <BufferedField id="ci-contact" label="Contact / reference" value={item.contact} disabled={!canEdit} maxLength={500} onCommit={(v) => patch({ contact: v })} />
        <Field label="Image">
          {img ? (
            <div style={{ position: "relative" }}>
              <img src={img} alt={item.name} style={{ width: "100%", maxHeight: 180, objectFit: "cover", borderRadius: 8 }} />
              {canEdit && (
                <div style={{ position: "absolute", top: 6, right: 6 }}>
                  <IconButton label="Remove image" onClick={() => void setImage(true)}>
                    <X size={14} />
                  </IconButton>
                </div>
              )}
            </div>
          ) : item.image ? (
            <Chip tone="r">Image file unavailable</Chip>
          ) : (
            canEdit && <Button size="sm" icon={<ImagePlus size={14} />} onClick={() => void setImage(false)}>Add Image</Button>
          )}
        </Field>
        <Field label="Also known as" hint="Other names used in the script. They help match this item in new scenes.">
          <div className="row wrap">
            {item.aliases.map((a) => (
              <Chip key={a.id}>
                {a.alias}
                {canEdit && (
                  <button type="button" aria-label={`Remove name ${a.alias}`} style={{ border: 0, background: "transparent", padding: 0, cursor: "pointer" }} onClick={() => void call("catalog.remove_alias", { id: a.id }).catch(reportError)}>
                    <X size={11} />
                  </button>
                )}
              </Chip>
            ))}
            {item.aliases.length === 0 && <span className="xs muted">None</span>}
          </div>
          {canEdit && (
            <form className="row" style={{ marginTop: 6 }} onSubmit={(e) => { e.preventDefault(); addAlias(); }}>
              <TextInput aria-label="Add another name" placeholder="Add another name" value={alias} maxLength={200} onChange={(e) => setAlias(e.target.value)} />
              <Button size="sm" type="submit" disabled={!alias.trim()}>Add</Button>
            </form>
          )}
        </Field>
        <Field label="Used in scenes">
          <SceneChips scenes={item.usedIn} />
        </Field>
        <div className="xs muted">No financial or procurement fields — this is a production directory.</div>
      </div>
    </Drawer>
  );
}

function AddItemDialog({ onClose, onCreated, initialCategory }: { onClose: () => void; onCreated: (id: string) => void; initialCategory: BreakdownCategory }) {
  const [category, setCategory] = useState<BreakdownCategory>(initialCategory);
  const [name, setName] = useState("");
  const [status, setStatus] = useState<CatalogStatus>("Required");
  const [description, setDescription] = useState("");
  const matches = useOp<CatalogMatchDto[]>("catalog.find_matches", { category, name: name.trim() }, PROD_TABLES, {
    enabled: name.trim().length > 0,
  });
  const exact = (matches.data ?? []).find((m) => m.exact);
  const create = useCommand<CatalogCreateArgs, CatalogItemDto>("catalog.create", {
    onSuccess: (i) => {
      toast.undoable(`Added “${i.name}” to the catalog`);
      onCreated(i.id);
      onClose();
    },
  });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title="Add Item"
      sub="A reusable production item. Scenes that need it are linked from the Breakdown."
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim() || create.isPending} onClick={() => create.mutate({ category, name: name.trim(), status, description: description.trim() || null })}>
            Add Item
          </Button>
        </>
      }
    >
      <Field label="Category" required htmlFor="ai-cat">
        <Select id="ai-cat" value={category} options={CATEGORY_OPTIONS} onChange={setCategory} />
      </Field>
      <Field label="Name" required htmlFor="ai-name">
        <TextInput id="ai-name" autoFocus value={name} maxLength={200} onChange={(e) => setName(e.target.value)} />
      </Field>
      {exact && <Banner tone="warn">The catalog already has “{exact.name}”. Adding creates a separate item; to reuse it, link it from the Breakdown instead.</Banner>}
      <Field label="Status" htmlFor="ai-status">
        <Select id="ai-status" value={status} options={STATUS_OPTIONS} onChange={setStatus} />
      </Field>
      <Field label="Description (optional)" htmlFor="ai-desc">
        <TextArea id="ai-desc" rows={2} value={description} maxLength={4000} onChange={(e) => setDescription(e.target.value)} />
      </Field>
    </Dialog>
  );
}

function ReplaceDialog({ item, onClose }: { item: CatalogItemDto; onClose: () => void }) {
  const candidates = useOp<CatalogItemDto[]>("catalog.list", { category: item.category, search: null, includeArchived: false }, PROD_TABLES);
  const options = (candidates.data ?? []).filter((c) => c.id !== item.id);
  const [target, setTarget] = useState<string>("");
  const inSource = item.usedIn.filter((s) => s.inSource);
  const [scenes, setScenes] = useState<Set<string>>(new Set(inSource.map((s) => s.sceneId)));
  const replace = useCommand<CatalogReplaceArgs, CatalogReplaceResult>("catalog.replace_in_scenes", {
    onSuccess: (r) => {
      toast.undoable(`Replaced in ${r.scenesUpdated} scene${r.scenesUpdated === 1 ? "" : "s"}. “${item.name}” is kept in the catalog.`);
      onClose();
    },
  });
  const chosen = target || options[0]?.id || "";
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title={`Replace “${item.name}” in scenes`}
      sub="This changes production associations only. The screenplay is not changed and the item is not deleted."
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!chosen || scenes.size === 0 || replace.isPending} onClick={() => replace.mutate({ fromId: item.id, toId: chosen, sceneIds: Array.from(scenes) })}>
            Replace in {scenes.size} Scene{scenes.size === 1 ? "" : "s"}
          </Button>
        </>
      }
    >
      {options.length === 0 ? (
        <Banner tone="info">There is no other {CATEGORY_SHORT[item.category].toLowerCase()} item to use. Add one in the Catalog first.</Banner>
      ) : (
        <Field label="Replacement" htmlFor="rep-target">
          <Select id="rep-target" value={chosen} options={options.map((o) => ({ value: o.id, label: `${o.name} (${o.status})` }))} onChange={setTarget} />
        </Field>
      )}
      <Field label="Apply to scenes">
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {inSource.map((s) => (
            <Checkbox
              key={s.sceneId}
              checked={scenes.has(s.sceneId)}
              onChange={(v) => {
                const next = new Set(scenes);
                if (v) next.add(s.sceneId);
                else next.delete(s.sceneId);
                setScenes(next);
              }}
              label={`Scene ${s.number} — ${s.heading}`}
            />
          ))}
        </div>
      </Field>
    </Dialog>
  );
}
