// Notes & Tasks (FSD §106, §158, §159; UX §3.35; mock 151).
// "Small things that do not belong anywhere else." Project notes (pinned
// first) and lightweight tasks (Open / Done, optional due date, owner and
// related object). No Gantt, dependencies or time tracking.

import { useEffect, useMemo, useState } from "react";
import { ListTodo, NotebookPen, Pin, Plus, Search } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { Member } from "../../ipc/generated/Member";
import type { NoteDto } from "../../ipc/generated/NoteDto";
import type { TaskDto } from "../../ipc/generated/TaskDto";
import { Button, Chip, EmptyState, PageHeader, Segmented, Select, Skeleton } from "../../design-system";
import { navigateTo } from "../../app/navigate";
import { useIntent, useNav } from "../../app/stores";
import { toast } from "../../app/toast";
import { dayWord } from "../../app/home/format";
import { NoteEditor, noteLabel } from "./NoteEditor";
import { TaskEditor } from "./TaskEditor";

type TaskFilter = "open" | "done" | "all";

function preview(n: NoteDto): string {
  const lines = n.body.split("\n").map((l) => l.trim()).filter(Boolean);
  const rest = n.title?.trim() ? lines : lines.slice(1);
  return rest.join(" ").slice(0, 120);
}

function dueLabel(ms: number): { text: string; overdue: boolean } {
  const d = new Date(ms);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const overdue = d.getTime() < today.getTime();
  return { text: `due ${d.toLocaleDateString([], { day: "numeric", month: "short" })}`, overdue };
}

export default function NotesAndTasks() {
  const route = useNav((s) => s.route);
  const consume = useIntent((s) => s.consume);
  const intent = useIntent((s) => s.intent);
  const [noteText, setNoteText] = useState("");
  const [openNote, setOpenNote] = useState<string | null>(route.params?.noteId ?? null);
  const [openTask, setOpenTask] = useState<string | "new" | null>(route.params?.taskId ?? null);
  const [filter, setFilter] = useState<TaskFilter>("open");
  const [owner, setOwner] = useState("");
  const notes = useOp<NoteDto[]>("notes.list", { text: noteText.trim() || null }, ["project_note", "project_member"]);
  const tasks = useOp<TaskDto[]>("tasks.list", { ownerUserId: owner || null }, ["task", "project_member", "search_doc", "*"]);
  const members = useOp<Member[]>("project.members", {}, ["project_member"]);

  // Search hits and "Continue" can open a specific note/task.
  useEffect(() => {
    if (route.params?.noteId) setOpenNote(route.params.noteId);
    if (route.params?.taskId) setOpenTask(route.params.taskId);
  }, [route.params?.noteId, route.params?.taskId]);

  // Quick actions: "+ > New Note / New Task".
  useEffect(() => {
    if (consume("notes.new_note")) void newNote();
    if (consume("notes.new_task")) setOpenTask("new");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [intent]);

  const newNote = async () => {
    try {
      const n = await call<NoteDto>("notes.create", { title: null, body: "" });
      setOpenTask(null);
      setOpenNote(n.id);
    } catch (e) {
      reportError(e);
    }
  };

  const toggleDone = async (t: TaskDto) => {
    const done = t.status !== "Done";
    try {
      await call("tasks.set_done", { id: t.id, done });
      toast.undoable(done ? `Completed task “${t.title}”` : `Reopened task “${t.title}”`);
    } catch (e) {
      reportError(e);
    }
  };

  const allTasks = useMemo(() => tasks.data ?? [], [tasks.data]);
  const openCount = allTasks.filter((t) => t.status === "Open").length;
  const shownTasks = allTasks.filter((t) => (filter === "all" ? true : filter === "open" ? t.status === "Open" : t.status === "Done"));
  const editingTask = openTask && openTask !== "new" ? allTasks.find((t) => t.id === openTask) ?? null : null;
  const noteList = notes.data ?? [];

  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader
        title="Notes & Tasks"
        sub="Small things that do not belong anywhere else."
        actions={
          <>
            <Button size="sm" icon={<Plus size={14} />} onClick={() => void newNote()}>New Note</Button>
            <Button size="sm" variant="primary" icon={<Plus size={14} />} onClick={() => { setOpenNote(null); setOpenTask("new"); }}>New Task</Button>
          </>
        }
      />
      <div className="row" style={{ alignItems: "flex-start", gap: 14 }}>
        <section className="card grow" aria-label="Project Notes" style={{ minWidth: 0 }}>
          <div className="li">
            <b className="grow">Project Notes</b>
            <div className="input" style={{ width: 200, padding: "0 8px", minHeight: 28 }}>
              <Search size={13} aria-hidden style={{ color: "var(--muted)" }} />
              <input type="search" aria-label="Search notes" placeholder="Search notes" value={noteText} onChange={(e) => setNoteText(e.target.value)} style={{ border: 0, outline: "none", flex: 1, background: "transparent", minHeight: 26, fontSize: 12.5 }} />
            </div>
          </div>
          {notes.isLoading ? (
            <div className="li"><Skeleton h={30} /></div>
          ) : noteList.length === 0 ? (
            noteText.trim() ? (
              <div className="li muted sm">No notes match “{noteText}”.</div>
            ) : (
              <EmptyState icon={<NotebookPen size={26} />} title="No notes yet." actions={<Button size="sm" icon={<Plus size={14} />} onClick={() => void newNote()}>New Note</Button>}>
                Add a note for something that doesn't naturally belong in Idea Vault or another workspace.
              </EmptyState>
            )
          ) : (
            <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
              {noteList.map((n) => (
                <li key={n.id}>
                  <button
                    className={`li clickable${openNote === n.id ? " sel" : ""}`}
                    style={{ width: "100%", border: 0, borderBottom: "1px solid var(--line)", background: openNote === n.id ? undefined : "transparent", textAlign: "left" }}
                    aria-current={openNote === n.id ? "true" : undefined}
                    onClick={() => { setOpenTask(null); setOpenNote(n.id); }}
                  >
                    <span className="grow" style={{ minWidth: 0 }}>
                      <b style={{ display: "flex", alignItems: "center", gap: 5 }}>
                        {n.pinned && <Pin size={12} aria-label="Pinned" />}
                        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{noteLabel(n)}</span>
                      </b>
                      {preview(n) && <span className="xs muted" style={{ display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{preview(n)}</span>}
                    </span>
                    <span className="xs muted">{dayWord(n.updatedAt)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="card grow" aria-label="Tasks" style={{ minWidth: 0 }}>
          <div className="li wrap" style={{ gap: 8 }}>
            <b>Tasks</b>
            <Chip tone="out">{openCount} open</Chip>
            <span className="grow" />
            <Select
              ariaLabel="Owner"
              value={owner}
              onChange={setOwner}
              options={[{ value: "", label: "Anyone" }, ...(members.data ?? []).map((m) => ({ value: m.userId, label: m.isYou ? "Me" : m.displayName }))]}
            />
            <Segmented ariaLabel="Show tasks" value={filter} onChange={setFilter} options={[{ value: "open", label: "Open" }, { value: "done", label: "Done" }, { value: "all", label: "All" }]} />
          </div>
          {tasks.isLoading ? (
            <div className="li"><Skeleton h={30} /></div>
          ) : shownTasks.length === 0 ? (
            allTasks.length === 0 && !owner ? (
              <EmptyState icon={<ListTodo size={26} />} title="No tasks yet." actions={<Button size="sm" icon={<Plus size={14} />} onClick={() => setOpenTask("new")}>New Task</Button>}>
                Add a small task only when you need to remember an action.
              </EmptyState>
            ) : (
              <div className="li muted sm">{filter === "open" ? "Nothing open. Well done." : filter === "done" ? "No completed tasks yet." : "No tasks for this owner."}</div>
            )
          ) : (
            <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
              {shownTasks.map((t) => {
                const done = t.status === "Done";
                const due = t.dueAt != null ? dueLabel(t.dueAt) : null;
                return (
                  <li key={t.id} className={`li${openTask === t.id ? " sel" : ""}`}>
                    <input
                      type="checkbox"
                      checked={done}
                      onChange={() => void toggleDone(t)}
                      aria-label={done ? `Reopen “${t.title}”` : `Mark “${t.title}” done`}
                      style={{ accentColor: "var(--accent)", width: 16, height: 16, flex: "none" }}
                    />
                    <button className="grow sm clickable" style={{ border: 0, background: "transparent", textAlign: "left", padding: 0, minWidth: 0 }} onClick={() => { setOpenNote(null); setOpenTask(t.id); }}>
                      {done ? <s className="muted">{t.title}</s> : t.title}
                      {t.ownerName && <span className="xs muted"> · {t.ownerName}</span>}
                    </button>
                    {t.related && (
                      t.related.nav != null ? (
                        <button className="chip" style={{ border: 0, cursor: "pointer" }} onClick={() => navigateTo(t.related?.nav)} title={`Open ${t.related.typeLabel}`}>
                          {t.related.typeLabel}{t.related.title ? `: ${t.related.title}` : ""}
                        </button>
                      ) : (
                        <Chip title={t.related.available ? undefined : "This item was deleted"}>{t.related.typeLabel}{t.related.available ? "" : " (deleted)"}</Chip>
                      )
                    )}
                    {due && !done && <span className={`xs ${due.overdue ? "errtxt" : "muted"}`}>{due.text}{due.overdue ? " · overdue" : ""}</span>}
                    {done && <Chip tone="g">Done</Chip>}
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      </div>
      {openNote && <NoteEditor key={openNote} noteId={openNote} onClose={() => setOpenNote(null)} />}
      {openTask === "new" && <TaskEditor key="new" task={null} onClose={() => setOpenTask(null)} />}
      {editingTask && <TaskEditor key={editingTask.id} task={editingTask} onClose={() => setOpenTask(null)} />}
    </div>
  );
}
