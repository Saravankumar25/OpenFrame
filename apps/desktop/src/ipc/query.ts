// Query/command hooks. React Query is only a cache of Rust-owned state:
// every query declares the tables it depends on, and Rust `dataChanged`
// events invalidate exactly those queries. No polling, no client-side truth.

import {
  QueryClient,
  useMutation,
  useQuery,
  type UseQueryOptions,
} from "@tanstack/react-query";
import { call, onAppEvent, OpError } from "./client";
import { toast } from "../app/toast";

export type StoreScope = "project" | "global";

export interface OpQueryMeta extends Record<string, unknown> {
  /** Tables this query reads; `"*"` = refetch on any change in the store. */
  tables: string[];
  store?: StoreScope;
}

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: Infinity, // Rust events drive freshness
      retry: (count, err) => (err instanceof OpError ? err.retryable && count < 2 : count < 1),
      refetchOnWindowFocus: false,
    },
  },
});

/** Read from the application core. */
export function useOp<T>(
  op: string,
  args: object | undefined,
  tables: string[],
  options: Omit<UseQueryOptions<T, OpError>, "queryKey" | "queryFn" | "meta"> & { store?: StoreScope } = {},
) {
  const { store = "project", ...rest } = options;
  return useQuery<T, OpError>({
    queryKey: [op, args ?? {}],
    queryFn: () => call<T>(op, args ?? {}),
    meta: { tables, store } satisfies OpQueryMeta,
    ...rest,
  });
}

/** Run a command. Errors are shown as human messages unless `silent`. */
export function useCommand<A extends object, R = unknown>(
  op: string,
  opts: { silent?: boolean; onSuccess?: (r: R, a: A) => void; onError?: (e: OpError, a: A) => void } = {},
) {
  return useMutation<R, OpError, A>({
    mutationFn: (args: A) => call<R>(op, args),
    onSuccess: (r, a) => opts.onSuccess?.(r, a),
    onError: (e, a) => {
      if (!opts.silent) reportError(e);
      opts.onError?.(e, a);
    },
  });
}

/** Show a failure the way the UX spec asks: human message, details kept for diagnostics. */
export function reportError(e: unknown): void {
  const err = e instanceof OpError ? e : new OpError("unknown", {
    code: "internal.unexpected",
    message: "Something went wrong inside OpenFrame. Your project was not changed.",
    retryable: false,
  });
  if (err.code === "internal.cancelled") return;
  toast.error(err.message);
  console.warn(`[${err.op}] ${err.code}`, err.detail ?? "");
}

let installed = false;
/** Wire Rust change events to query invalidation. Call once at startup. */
export function installInvalidation(client: QueryClient = queryClient): void {
  if (installed) return;
  installed = true;
  void onAppEvent((ev) => {
    if (ev.type === "dataChanged") {
      const changed = new Set(ev.tables);
      const store: StoreScope = ev.store === "global" ? "global" : "project";
      void client.invalidateQueries({
        predicate: (q) => {
          const meta = q.meta as OpQueryMeta | undefined;
          if (!meta) return false;
          if ((meta.store ?? "project") !== store && !meta.tables.includes("**")) return false;
          return meta.tables.includes("*") || meta.tables.includes("**") || meta.tables.some((t) => changed.has(t));
        },
      });
    } else if (ev.type === "projectOpened" || ev.type === "projectClosed") {
      void client.invalidateQueries();
    }
  });
}
