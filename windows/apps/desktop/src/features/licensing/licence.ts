// The licence state and the screen due now, read from the Rust core, and the "Buy full version"
// action for every channel. The core decides; this file only asks and routes.

import { useQuery, type QueryClient } from "@tanstack/react-query";
import { create } from "zustand";
import { call, OpError } from "../../ipc/client";
import type { EntitlementNotice } from "../../ipc/generated/EntitlementNotice";
import type { BuyOutcome } from "../../ipc/generated/BuyOutcome";
import type { NoticeKind } from "../../ipc/generated/NoticeKind";
import type { EntitlementStatus } from "../../ipc/generated/EntitlementStatus";
import { localDay } from "./rules";

export const NOTICE_OP = "entitlement.notice";

/** The licence state and the due screen. Asked at launch, when the window comes back to the front
 * and every 5 minutes while the window is in front. Each ask only reads the state the core already
 * holds: the core works it out at startup, on Restore purchase and, on the Mac, at each online
 * check. On Windows a trial that ends while OpenFrame stays open therefore shows at the next
 * launch. */
export function useLicenceNotice() {
  const today = localDay();
  return useQuery<EntitlementNotice, OpError>({
    queryKey: [NOTICE_OP, today],
    queryFn: () => call<EntitlementNotice>(NOTICE_OP, { today }),
    staleTime: 60_000,
    refetchOnWindowFocus: true,
    refetchInterval: 5 * 60_000,
  });
}

/** Ask the core again after a finished screen. */
export function refreshLicence(client: QueryClient): Promise<void> {
  return client.invalidateQueries({ queryKey: [NOTICE_OP] });
}

/** After a purchase, a restore or a trial start everything is asked again: on the Mac the lock
 * (ADR-0022) may have lifted, and the operations it refused now answer. */
function afterStoreChange(client: QueryClient): Promise<void> {
  return client.invalidateQueries();
}

/** The "Checking your purchase" screen (NeedsOnlineCheck, either store) can be closed: a failed
 * check keeps opening and exporting (ADR-0018 decision 6). Closed, it stays closed for this
 * session while the core still names it, and the top bar offers "Check purchase" to open it
 * again. The lock after the trial or a refund (ADR-0022, ADR-0023) has no such state: it cannot be
 * closed. */
interface CheckScreenState {
  closed: boolean;
  close: () => void;
  reopen: () => void;
}

export const useCheckScreen = create<CheckScreenState>((set) => ({
  closed: false,
  close: () => set({ closed: true }),
  reopen: () => set({ closed: false }),
}));

/** Remember a reminder day or a finished screen, then show what comes next. */
export async function recordNotice(client: QueryClient, notice: NoticeKind): Promise<void> {
  const today = localDay();
  const next = await call<EntitlementNotice>("entitlement.record_notice", { notice, today });
  client.setQueryData([NOTICE_OP, today], next);
}

/** "Buy full version": hands off to the store's own purchase window, which shows the price
 * (ADR-0023). `null` when the store took over and finished or was closed, or plain words. */
export async function buyFullVersion(client: QueryClient): Promise<string | null> {
  const r = await call<BuyOutcome>("entitlement.buy", {});
  await afterStoreChange(client);
  return r.message;
}

/** "Start free trial" (Mac first launch). */
export async function startTrial(client: QueryClient): Promise<string | null> {
  const r = await call<BuyOutcome>("entitlement.start_trial", {});
  await afterStoreChange(client);
  return r.message;
}

/** "Restore purchase" or "Try again": asks the store again. */
export async function restorePurchase(client: QueryClient): Promise<EntitlementStatus> {
  const s = await call<EntitlementStatus>("entitlement.restore_purchase", {});
  await afterStoreChange(client);
  return s;
}

/** Plain words for a failed call: the core's message, or a calm fallback. */
export function failureText(e: unknown, fallback: string): string {
  return e instanceof OpError ? e.message : fallback;
}
