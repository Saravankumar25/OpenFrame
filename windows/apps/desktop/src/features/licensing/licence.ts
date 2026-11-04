// The licence state and the screen due now, read from the Rust core, and the "Buy full version"
// action for every channel. The core decides; this file only asks and routes.

import { useEffect, useState } from "react";
import { useQuery, type QueryClient } from "@tanstack/react-query";
import { create } from "zustand";
import { call, OpError, openUrl } from "../../ipc/client";
import { toast } from "../../app/toast";
import type { EntitlementNotice } from "../../ipc/generated/EntitlementNotice";
import type { BuyOutcome } from "../../ipc/generated/BuyOutcome";
import type { NoticeKind } from "../../ipc/generated/NoticeKind";
import type { EntitlementStatus } from "../../ipc/generated/EntitlementStatus";
import { CONTACT_URL, localDay } from "./rules";

export const NOTICE_OP = "entitlement.notice";

/** The local calendar day, kept current: checked every minute, so it changes soon after local
 * midnight even when nothing else re-renders and even when the clock or time zone was moved back,
 * and the day's reminder and the days left are asked for again. */
function useToday(): string {
  const [today, setToday] = useState(localDay);
  useEffect(() => {
    const timer = setInterval(() => setToday(localDay()), 60_000);
    return () => clearInterval(timer);
  }, []);
  return today;
}

/** The licence state and the due screen. Asked at launch, when the window comes back to the front,
 * every 5 minutes while the window is in front, and when the core says the licence changed (the
 * store refresh or a purchase made elsewhere; see `LicenceScreens`). Each ask only reads the state
 * the core already holds, so a trial that ends while Straft stays open shows the lock after the
 * core's next store refresh. */
export function useLicenceNotice() {
  const today = useToday();
  return useQuery<EntitlementNotice, OpError>({
    queryKey: [NOTICE_OP, today],
    queryFn: () => call<EntitlementNotice>(NOTICE_OP, { today }),
    // Midnight changes the key: keep the screens up while the new day's answer arrives.
    placeholderData: (previous) => previous,
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

/** "Contact us": opens the contact page in the browser. No form, no request from the app; a failed
 * open shows the same toast as the other links. */
export function openContact(): void {
  void openUrl(CONTACT_URL).catch((e: unknown) => toast.error(failureText(e, "Straft couldn't open that link.")));
}

/** Plain words for a failed call: the core's message, or a calm fallback. */
export function failureText(e: unknown, fallback: string): string {
  return e instanceof OpError ? e.message : fallback;
}
