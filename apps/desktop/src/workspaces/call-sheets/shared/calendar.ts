// Pure month-grid helpers for the schedule calendar (UX §3.29, mock 132).

export interface CalCell {
  /** YYYY-MM-DD */
  iso: string;
  day: number;
  inMonth: boolean;
}

const pad = (n: number) => String(n).padStart(2, "0");

export function isoDate(y: number, m0: number, d: number): string {
  return `${y}-${pad(m0 + 1)}-${pad(d)}`;
}

/** Six Monday-first weeks covering the month `m0` (0-based) of `year`. */
export function monthGrid(year: number, m0: number): CalCell[] {
  const first = new Date(Date.UTC(year, m0, 1));
  const offset = (first.getUTCDay() + 6) % 7; // Monday = 0
  const start = Date.UTC(year, m0, 1 - offset);
  const cells: CalCell[] = [];
  for (let i = 0; i < 42; i++) {
    const d = new Date(start + i * 86_400_000);
    cells.push({ iso: isoDate(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()), day: d.getUTCDate(), inMonth: d.getUTCMonth() === m0 });
  }
  return cells;
}

export function monthLabel(year: number, m0: number): string {
  return new Date(Date.UTC(year, m0, 1)).toLocaleString(undefined, { month: "long", year: "numeric", timeZone: "UTC" });
}

export function shiftMonth(year: number, m0: number, delta: number): [number, number] {
  const t = year * 12 + m0 + delta;
  return [Math.floor(t / 12), ((t % 12) + 12) % 12];
}
