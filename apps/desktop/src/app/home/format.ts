// Human date/time wording shared by Home, Project Home and the project support
// pages ("Last modified 2 hours ago", "Today 10:41", "Yesterday, 11:58 PM").

const DAY = 86_400_000;

function startOfDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

export function timeOfDay(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** Short date like "12 Sep 2026". */
export function shortDate(ms: number): string {
  return new Date(ms).toLocaleDateString([], { day: "numeric", month: "short", year: "numeric" });
}

/** Relative wording for recent moments: "just now", "5 minutes ago", "Yesterday", "12 Sep 2026". */
export function ago(ms: number, now: number = Date.now()): string {
  const d = now - ms;
  if (d < 60_000) return "just now";
  if (d < 3_600_000) {
    const m = Math.round(d / 60_000);
    return `${m} minute${m === 1 ? "" : "s"} ago`;
  }
  const today = startOfDay(now);
  if (ms >= today) {
    const h = Math.round(d / 3_600_000);
    return `${h} hour${h === 1 ? "" : "s"} ago`;
  }
  if (ms >= today - DAY) return "Yesterday";
  return shortDate(ms);
}

/** Day + time for lists: "Today 10:41", "Yesterday 09:12", "12 Sep 10:02". */
export function dayAndTime(ms: number, now: number = Date.now()): string {
  const today = startOfDay(now);
  if (ms >= today) return `Today ${timeOfDay(ms)}`;
  if (ms >= today - DAY) return `Yesterday ${timeOfDay(ms)}`;
  return `${new Date(ms).toLocaleDateString([], { day: "numeric", month: "short" })} ${timeOfDay(ms)}`;
}

/** Coarse day wording for tables: "Today", "Yesterday", "3 days ago", "12 Sep 2026". */
export function dayWord(ms: number, now: number = Date.now()): string {
  const today = startOfDay(now);
  if (ms >= today) return "Today";
  if (ms >= today - DAY) return "Yesterday";
  const days = Math.floor((today - startOfDay(ms)) / DAY);
  if (days < 7) return `${days} days ago`;
  return shortDate(ms);
}

export function fileSize(bytes: number | null | undefined): string {
  if (bytes == null) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** Epoch ms of a local calendar day <-> "YYYY-MM-DD" for <input type="date">. */
export function toDateInput(ms: number | null | undefined): string {
  if (ms == null) return "";
  const d = new Date(ms);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

export function fromDateInput(v: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(v);
  if (!m) return null;
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), 12, 0, 0, 0).getTime();
}
