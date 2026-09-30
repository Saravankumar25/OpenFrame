// Templates (FSD §59, §113; Domain §20; UX §3.37; mock 153).
// "Would a saved starting point help me begin faster?"
//
// Built-in (read-only), Yours (kept on this computer for every project) and
// This project. "Use in This Project" copies a template into the project; the
// copy is independent — later template edits never change what was made from it.
// Document workspaces (call sheets, shot lists, moodboards, reports) can read
// them with templates.list { templateType }. The blank path is always available.

import { useEffect, useState } from "react";
import { Copy, Eye, MoreHorizontal, Pencil, Plus, Save, Trash2 } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { TemplateDto } from "../../ipc/generated/TemplateDto";
import type { TemplateScope } from "../../ipc/generated/TemplateScope";
import type { TemplatesListing } from "../../ipc/generated/TemplatesListing";
import { Button, ConfirmDialog, Dialog, EmptyState, Field, Menu, PageHeader, Segmented, Select, Skeleton, TextArea, TextInput, type MenuItemSpec } from "../../design-system";
import { useNav } from "../../app/stores";
import { toast } from "../../app/toast";

export const TEMPLATE_TYPES: { value: string; label: string; listLabel: string; key: string }[] = [
  { value: "call_sheet", label: "Call sheet", listLabel: "Sections", key: "sections" },
  { value: "shot_list", label: "Shot list", listLabel: "Columns", key: "columns" },
  { value: "moodboard", label: "Moodboard", listLabel: "Sections", key: "sections" },
  { value: "report", label: "Report", listLabel: "Sections", key: "sections" },
  { value: "story_board", label: "Story Board starter", listLabel: "Acts", key: "acts" },
  { value: "breakdown", label: "Breakdown document", listLabel: "Categories", key: "categories" },
  { value: "budget", label: "Simple budget", listLabel: "Categories", key: "categories" },
  { value: "project_starter", label: "Project starter", listLabel: "Acts", key: "acts" },
];

const typeDef = (t: string) => TEMPLATE_TYPES.find((x) => x.value === t) ?? TEMPLATE_TYPES[0];

/** Lines of text → template content for a type (`{ description, <key>: [...] }`), keeping other keys. */
export function contentFrom(type: string, description: string, linesText: string, base: Record<string, unknown> = {}): Record<string, unknown> {
  const items = linesText.split("\n").map((l) => l.trim()).filter(Boolean);
  const next: Record<string, unknown> = { ...base, [typeDef(type).key]: items };
  if (description.trim()) next.description = description.trim();
  else delete next.description;
  return next;
}

function listOf(content: Record<string, unknown>, key: string): string[] {
  const v = content[key];
  return Array.isArray(v) ? v.map(String) : [];
}

const SCOPE_TABLES = ["template"];

export function Templates() {
  const listing = useOp<TemplatesListing>("templates.list", {}, SCOPE_TABLES, { refetchOnMount: "always" });
  const route = useNav((s) => s.route);
  const [editing, setEditing] = useState<{ template?: TemplateDto } | null>(null);
  const [preview, setPreview] = useState<TemplateDto | null>(null);
  const [deleting, setDeleting] = useState<TemplateDto | null>(null);
  const reload = () => void listing.refetch();

  // Search hits open a project template.
  useEffect(() => {
    const id = route.params?.templateId;
    const t = id ? listing.data?.project.find((x) => x.id === id) : undefined;
    if (t) setPreview(t);
  }, [route.params?.templateId, listing.data]);

  const copyIntoProject = async (t: TemplateDto) => {
    try {
      const copy = await call<TemplateDto>("templates.copy", { scope: t.scope, id: t.id, toScope: "project" });
      toast.undoable(`Added template “${copy.name}” to this project`);
      reload();
    } catch (e) {
      reportError(e);
    }
  };
  const keepForAll = async (t: TemplateDto) => {
    try {
      const copy = await call<TemplateDto>("templates.copy", { scope: t.scope, id: t.id, toScope: "global" });
      toast.success(`“${copy.name}” is now in your templates for every project.`);
      reload();
    } catch (e) {
      reportError(e);
    }
  };
  const remove = async (t: TemplateDto) => {
    setDeleting(null);
    try {
      await call("templates.delete", { scope: t.scope, id: t.id });
      if (t.scope === "project") toast.undoable(`Moved template “${t.name}” to Recently Deleted`);
      else toast.info(`Deleted template “${t.name}”. Projects that already used it are not affected.`);
      reload();
    } catch (e) {
      reportError(e);
    }
  };

  const menuFor = (t: TemplateDto): MenuItemSpec[] => [
    { label: "Preview", icon: <Eye size={14} />, onSelect: () => setPreview(t) },
    ...(t.scope !== "project" ? [{ label: "Use in This Project", icon: <Copy size={14} />, onSelect: () => void copyIntoProject(t) }] : []),
    ...(t.scope === "project" ? [{ label: "Save to My Templates", icon: <Save size={14} />, onSelect: () => void keepForAll(t) }] : []),
    ...(t.scope !== "builtin"
      ? [
          { label: "Edit", icon: <Pencil size={14} />, onSelect: () => setEditing({ template: t }) },
          { label: "Delete…", icon: <Trash2 size={14} />, danger: true, separatorBefore: true, onSelect: () => (t.scope === "project" ? void remove(t) : setDeleting(t)) },
        ]
      : [{ label: "Copy to My Templates", icon: <Save size={14} />, onSelect: () => void keepForAll(t) }]),
  ];

  const section = (title: string, sub: string, items: TemplateDto[], empty: string) => (
    <section aria-label={title} style={{ marginBottom: 18 }}>
      <div className="h4">{title}</div>
      <div className="xs muted" style={{ margin: "-2px 0 8px" }}>{sub}</div>
      {items.length === 0 ? (
        <div className="card muted sm" style={{ padding: 14 }}>{empty}</div>
      ) : (
        <div className="grid g4">
          {items.map((t) => (
            <div key={`${t.scope}:${t.id}`} className="card pad" style={{ position: "relative", display: "flex", flexDirection: "column", gap: 6 }}>
              <button className="clickable" style={{ border: 0, background: "transparent", padding: 0, textAlign: "left", display: "flex", flexDirection: "column", gap: 6 }} onClick={() => setPreview(t)} aria-label={`Preview ${t.name}, ${t.typeLabel}`}>
                <div className="photo cool" style={{ height: 64, borderRadius: 6 }} aria-hidden />
                <b>{t.name}</b>
                <span className="xs muted">{t.typeLabel}{t.description ? ` · ${t.description}` : ""}</span>
              </button>
              <div style={{ position: "absolute", top: 16, right: 18 }}>
                <Menu align="end" items={menuFor(t)} trigger={<button className="iconbtn" aria-label={`Actions for ${t.name}`} style={{ background: "#ffffffd9" }}><MoreHorizontal size={15} /></button>} />
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );

  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader
        title="Templates"
        sub="Would a saved starting point help me begin faster?"
        actions={<Button size="sm" variant="primary" icon={<Plus size={14} />} onClick={() => setEditing({})}>Create Template</Button>}
      />
      {listing.isLoading ? (
        <div className="grid g4">{[0, 1, 2, 3].map((i) => <Skeleton key={i} h={120} />)}</div>
      ) : !listing.data ? (
        <EmptyState title="Templates couldn't be loaded." actions={<Button onClick={reload}>Try Again</Button>}>{listing.error?.message}</EmptyState>
      ) : (
        <>
          {section("Built-in", "Starting points that come with OpenFrame. Use one to copy it into this project.", listing.data.builtin, "")}
          {section("Yours", "Kept on this computer and available in every project.", listing.data.global, "Save a project template to My Templates, or create one, to reuse it in every project.")}
          {section("This project", "Copies kept inside this project. Changing a template never changes anything already made from it.", listing.data.project, "No templates in this project yet. Use a built-in template or one of yours, or create one.")}
          <p className="xs muted">Templates are optional — every document can also start blank.</p>
        </>
      )}
      {editing && <TemplateDialog template={editing.template} onClose={() => setEditing(null)} onSaved={reload} />}
      {preview && (
        <PreviewDialog
          template={preview}
          onClose={() => setPreview(null)}
          onUse={() => { setPreview(null); void copyIntoProject(preview); }}
          onKeep={() => { setPreview(null); void keepForAll(preview); }}
          onEdit={() => { setPreview(null); setEditing({ template: preview }); }}
        />
      )}
      {deleting && (
        <ConfirmDialog open onOpenChange={(v) => !v && setDeleting(null)} title={`Delete template “${deleting.name}”?`} confirmLabel="Delete Template" danger onConfirm={() => void remove(deleting)}>
          <p style={{ marginTop: 0 }}>This template is removed from this computer. Projects and documents that already used it are not affected.</p>
        </ConfirmDialog>
      )}
    </div>
  );
}

function PreviewDialog({ template, onClose, onUse, onKeep, onEdit }: { template: TemplateDto; onClose: () => void; onUse: () => void; onKeep: () => void; onEdit: () => void }) {
  const lists = Object.entries(template.content).filter(([, v]) => Array.isArray(v)) as [string, unknown[]][];
  const scopeLabel = template.scope === "builtin" ? "Built-in" : template.scope === "global" ? "Yours · every project" : "This project";
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={template.name}
      sub={`${template.typeLabel} · ${scopeLabel}`}
      size="md"
      footer={
        <>
          {template.scope !== "builtin" && <Button onClick={onEdit} icon={<Pencil size={14} />}>Edit</Button>}
          {template.scope === "project" ? (
            <Button variant="primary" icon={<Save size={14} />} onClick={onKeep}>Save to My Templates</Button>
          ) : (
            <Button variant="primary" icon={<Copy size={14} />} onClick={onUse}>Use in This Project</Button>
          )}
        </>
      }
    >
      {template.description && <p style={{ marginTop: 0 }}>{template.description}</p>}
      {lists.length === 0 ? (
        <p className="muted">This template has no preset content yet.</p>
      ) : (
        <div className="grid g2">
          {lists.map(([k, items]) => (
            <div key={k}>
              <div className="h4">{k.charAt(0).toUpperCase() + k.slice(1)}</div>
              <ol className="sm" style={{ margin: 0, paddingLeft: 18 }}>
                {items.map((it, i) => <li key={i}>{String(it)}</li>)}
              </ol>
            </div>
          ))}
        </div>
      )}
    </Dialog>
  );
}

function TemplateDialog({ template, onClose, onSaved }: { template?: TemplateDto; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState(template?.name ?? "");
  const [type, setType] = useState(template?.templateType ?? "call_sheet");
  const [scope, setScope] = useState<Exclude<TemplateScope, "builtin">>(template?.scope === "global" ? "global" : "project");
  const [description, setDescription] = useState(template?.description ?? "");
  const [lines, setLines] = useState(template ? listOf(template.content, typeDef(template.templateType).key).join("\n") : "");
  const [touched, setTouched] = useState(false);
  const [busy, setBusy] = useState(false);
  const invalid = name.trim().length === 0;
  const def = typeDef(type);
  const save = async () => {
    setTouched(true);
    if (invalid || busy) return;
    setBusy(true);
    const content = contentFrom(type, description, lines, template?.content ?? {});
    try {
      if (template) {
        await call("templates.update", { scope: template.scope, id: template.id, name: name.trim(), content });
        toast.success(`Saved template “${name.trim()}”.`);
      } else {
        await call("templates.create", { scope, templateType: type, name: name.trim(), content });
        toast.success(scope === "global" ? `Created “${name.trim()}” in your templates.` : `Created “${name.trim()}” in this project.`);
      }
      onSaved();
      onClose();
    } catch (e) {
      reportError(e);
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={template ? "Edit Template" : "Create Template"}
      sub={template ? "Changes apply to future use only." : "A saved starting point for new documents."}
      size="md"
      footer={<><Button onClick={onClose}>Cancel</Button><Button variant="primary" type="submit" form="template-form" disabled={busy}>{template ? "Save" : "Create"}</Button></>}
    >
      <form id="template-form" onSubmit={(e) => { e.preventDefault(); void save(); }}>
        <Field label="Name" required htmlFor="tp-name" error={touched && invalid ? "Give the template a name." : null}>
          <TextInput id="tp-name" autoFocus value={name} maxLength={120} invalid={touched && invalid} placeholder="e.g. My short-film call sheet" onChange={(e) => setName(e.target.value)} />
        </Field>
        <div className="grid g2" style={{ gap: "0 12px" }}>
          <Field label="Type" required htmlFor="tp-type">
            {template ? <TextInput id="tp-type" value={def.label} readOnly /> : <Select id="tp-type" value={type} onChange={setType} options={TEMPLATE_TYPES.map((t) => ({ value: t.value, label: t.label }))} />}
          </Field>
          <Field label="Keep in">
            {template ? (
              <span className="sm muted" style={{ paddingTop: 6 }}>{template.scope === "global" ? "Your templates (every project)" : "This project"}</span>
            ) : (
              <Segmented ariaLabel="Keep template in" value={scope} onChange={setScope} options={[{ value: "project", label: "This project" }, { value: "global", label: "Every project" }]} />
            )}
          </Field>
        </div>
        <Field label="Description" htmlFor="tp-desc">
          <TextInput id="tp-desc" value={description} maxLength={200} placeholder="e.g. Standard one-page" onChange={(e) => setDescription(e.target.value)} />
        </Field>
        <Field label={def.listLabel} htmlFor="tp-lines" hint={`One per line. These become the ${def.listLabel.toLowerCase()} of new documents made from this template.`}>
          <TextArea id="tp-lines" rows={7} value={lines} onChange={(e) => setLines(e.target.value)} />
        </Field>
      </form>
    </Dialog>
  );
}
