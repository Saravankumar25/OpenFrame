// Test helpers for component tests (vitest + Testing Library). Tests mock
// `ipc/client` so operations are answered by a per-test table of responses.

import type { ReactElement } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";

/** Render inside a fresh React Query client (no retries, no shared cache). */
export function renderWithClient(ui: ReactElement) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity }, mutations: { retry: false } } });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

export type OpTable = Record<string, unknown | ((args: Record<string, unknown>) => unknown)>;

/** Build a `call` implementation answering from `table` (functions receive the args). */
export function answerFrom(table: OpTable) {
  return async (op: string, args: object = {}) => {
    if (!(op in table)) throw new Error(`unexpected operation in test: ${op}`);
    const v = table[op];
    return typeof v === "function" ? (v as (a: Record<string, unknown>) => unknown)(args as Record<string, unknown>) : v;
  };
}
