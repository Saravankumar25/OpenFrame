// Toasts (UX §2.9, mock 021): dark pill, bottom centre, human text, optional
// action such as "Undo". There is no notification centre.

import { create } from "zustand";
import { call } from "../ipc/client";

export interface ToastItem {
  id: number;
  kind: "info" | "error" | "success";
  text: string;
  action?: { label: string; run: () => void };
}

interface ToastState {
  items: ToastItem[];
  push: (t: Omit<ToastItem, "id">, ms?: number) => void;
  dismiss: (id: number) => void;
}

let seq = 0;
export const useToasts = create<ToastState>((set) => ({
  items: [],
  push: (t, ms = 5000) => {
    const id = ++seq;
    set((s) => ({ items: [...s.items.slice(-2), { ...t, id }] }));
    window.setTimeout(() => set((s) => ({ items: s.items.filter((x) => x.id !== id) })), ms);
  },
  dismiss: (id) => set((s) => ({ items: s.items.filter((x) => x.id !== id) })),
}));

export const toast = {
  info: (text: string, action?: ToastItem["action"]) => useToasts.getState().push({ kind: "info", text, action }),
  success: (text: string) => useToasts.getState().push({ kind: "success", text }),
  error: (text: string) => useToasts.getState().push({ kind: "error", text }, 8000),
  /** Confirmation of a completed change with an Undo link (FSD §51). */
  undoable: (text: string, store: "project" | "global" = "project") =>
    useToasts.getState().push({
      kind: "info",
      text,
      action: {
        label: "Undo",
        run: () => {
          void call("history.undo", { store }).catch((e: { message?: string }) => toast.error(e.message ?? "Undo failed."));
        },
      },
    }),
};

export function Toaster() {
  const items = useToasts((s) => s.items);
  const dismiss = useToasts((s) => s.dismiss);
  if (items.length === 0) return null;
  return (
    <div aria-live="polite" role="status">
      {items.map((t, i) => (
        <div
          key={t.id}
          className="toast"
          style={{ bottom: 40 + i * 52, background: t.kind === "error" ? "#8f2b22" : undefined }}
        >
          <span>{t.text}</span>
          {t.action && (
            <a
              href="#"
              onClick={(e) => {
                e.preventDefault();
                t.action?.run();
                dismiss(t.id);
              }}
            >
              {t.action.label}
            </a>
          )}
          <button
            aria-label="Dismiss"
            onClick={() => dismiss(t.id)}
            style={{ background: "transparent", border: 0, color: "#fff9", fontSize: 16, lineHeight: 1 }}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
