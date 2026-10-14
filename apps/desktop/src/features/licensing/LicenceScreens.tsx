// Trial reminders, the store screens and the once-only screens after a purchase (owner mocks 184
// to 198, docs/engineering/36-entitlement-and-trial.md section 10). The Rust core decides which
// screen is due (`entitlement.notice`); this component shows one at a time.
//
// Mandatory screens cannot be closed: no close button, Escape and clicks outside do nothing, focus
// stays inside, and the app's own keyboard shortcuts are held back while they are open (copy, cut,
// paste and select all still work, so the root folder path can be copied). They are the lock
// after the trial or a refund in either store (owner decisions 2026-10-07, ADR-0022 for the Mac,
// ADR-0023 for the Microsoft Store; the Rust core refuses every project operation underneath it,
// so the screen is not the only gate), the Mac first launch, the data note and the tutorial. The
// "Checking your purchase" screen can be closed: a failed check keeps opening and exporting
// (ADR-0018 decision 6). The internal installer build shows none of these screens (ADR-0023).
// No screen names a price, a currency or a pricing model: the store's own purchase window does.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import { FolderOpen, Info, RefreshCw, TriangleAlert, Zap } from "lucide-react";
import { Banner, Button, Checkbox, Dialog } from "../../design-system";
import { call, openUrl, revealLocation } from "../../ipc/client";
import { reportQuietly, useOp } from "../../ipc/query";
import type { AppInfo } from "../../ipc/generated/AppInfo";
import type { Channel } from "../../ipc/generated/Channel";
import type { EntitlementState } from "../../ipc/generated/EntitlementState";
import { toast } from "../../app/toast";
import { BrandLogo } from "../../theme/BrandLogo";
import {
  buyFullVersion,
  failureText,
  recordNotice,
  refreshLicence,
  restorePurchase,
  startTrial,
  useCheckScreen,
  useLicenceNotice,
} from "./licence";
import {
  SOCIAL_LINKS,
  isAppShortcut,
  isFixedSocialUrl,
  localDay,
  newLaptopInstallStep,
  reminderHeadline,
  storeName,
  revealLabel,
  type SocialLink,
} from "./rules";
import "./licensing.css";

const BUY_FAILED = "OpenFrame could not start the purchase right now. Everything keeps working.";
const SAVE_FAILED = "OpenFrame could not save this step. Try again.";

/** Holds back the app's own keyboard shortcuts (Ctrl, Alt or Cmd combinations) while a mandatory
 * screen is open, so nothing opens on top of it or behind it. Copy, cut, paste and select all go
 * through, so the root folder path can be copied. */
export function useHoldShortcuts(active: boolean): void {
  useEffect(() => {
    if (!active) return;
    const hold = (e: KeyboardEvent) => {
      if (isAppShortcut(e)) {
        e.preventDefault();
        e.stopImmediatePropagation();
      }
    };
    window.addEventListener("keydown", hold, true);
    return () => window.removeEventListener("keydown", hold, true);
  }, [active]);
}

/** The product logo, the same mark and wordmark as the top bar. Decorative: the dialog title
 * names the screen. */
function Brand() {
  return (
    <div className="lic-brand" aria-hidden>
      <BrandLogo />
    </div>
  );
}

/** One screen at a time. `projectOpen` is false on Application Home. */
export function LicenceScreens({ projectOpen }: { projectOpen: boolean }) {
  const q = useLicenceNotice();
  const client = useQueryClient();
  const info = useOp<AppInfo>("app.info", {}, []);
  const [reminder, setReminder] = useState<number | null>(null);
  const [dismissedOn, setDismissedOn] = useState<string | null>(null);
  const checkClosed = useCheckScreen((s) => s.closed);
  const closeCheck = useCheckScreen((s) => s.close);
  const reopenCheck = useCheckScreen((s) => s.reopen);
  const notice = q.data?.notice;
  const today = localDay();

  // A reminder is remembered as soon as it is shown, so it appears at most once that day even
  // if OpenFrame closes while it is open. A failed save only means it may show once more, so it
  // goes to the diagnostics and not on screen.
  useEffect(() => {
    if (notice?.kind !== "reminder" || !projectOpen || reminder !== null || dismissedOn === today) return;
    setReminder(notice.daysLeft);
    call("entitlement.record_notice", { notice: "reminder", today }).catch(reportQuietly);
  }, [notice, projectOpen, reminder, dismissedOn, today]);

  // A closed check screen stays closed only while the core still names it.
  useEffect(() => {
    if (checkClosed && notice && notice.kind !== "checkNeeded") reopenCheck();
  }, [checkClosed, notice, reopenCheck]);

  if (!q.data?.status || !notice) return null;
  const platform = info.data?.platform;
  const channel = q.data.status.channel;
  const dataRoot = q.data.dataRoot;

  if (reminder !== null) {
    return (
      <ReminderDialog
        daysLeft={reminder}
        onClose={() => {
          setReminder(null);
          setDismissedOn(today);
          void refreshLicence(client);
        }}
      />
    );
  }
  switch (notice.kind) {
    case "dataNote":
      return <DataNoteDialog dataRoot={dataRoot} platform={platform} />;
    case "tutorial":
      return <TutorialDialog dataRoot={dataRoot} platform={platform} channel={channel} />;
    case "welcome":
      return <WelcomeDialog />;
    case "macFirstLaunch":
      return <MacFirstLaunchDialog />;
    case "locked":
      return <LockedDialog state={q.data.status.state} channel={channel} />;
    case "checkNeeded":
      return checkClosed ? null : <CheckDialog channel={channel} onClose={closeCheck} />;
    default:
      return null;
  }
}

// ------------------------------------------------------------------ trial reminder (184, 185, 189)

export function ReminderDialog({ daysLeft, onClose }: { daysLeft: number; onClose: () => void }) {
  const client = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const buy = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const m = await buyFullVersion(client);
      if (m) setMessage(m);
      else onClose();
    } catch (e) {
      setMessage(failureText(e, BUY_FAILED));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Trial reminder"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Continue trial</Button>
          <Button variant="primary" icon={<Zap size={14} aria-hidden />} disabled={busy} onClick={() => void buy()}>
            Buy full version
          </Button>
        </>
      }
    >
      <Brand />
      <h3 style={{ margin: "10px 0 6px", textAlign: "center", fontSize: 19 }}>{reminderHeadline(daysLeft)}</h3>
      <p style={{ textAlign: "center", margin: "0 0 8px" }}>Buy the full version to keep using OpenFrame Studio.</p>
      <Banner tone="info">
        <Info size={14} aria-hidden /> Your projects stay on your computer either way. Nothing is deleted when the trial ends.
      </Banner>
      {message && (
        <div style={{ marginTop: 8 }}>
          <Banner tone="warn">{message}</Banner>
        </div>
      )}
    </Dialog>
  );
}

// ------------------------------------------------------------------ data note (193)

const DATA_POINTS: { title: string; body: string }[] = [
  {
    title: "Your projects live only on this computer.",
    body: "OpenFrame has no cloud. We do not hold a copy, so we cannot recover your work if the computer is lost, stolen, damaged or reset.",
  },
  {
    title: "Back up the whole root folder.",
    body: "Pick the one folder where your OpenFrame projects are saved and copy it to Google Drive (or another cloud drive) and, if you can, to an external disk. Repeat it often.",
  },
  {
    title: "Moving to a new laptop is your job, too.",
    body: "Copy that same root folder to the new computer, install OpenFrame, then use Open Project and pick the folder.",
  },
  {
    title: "Keep at least two copies.",
    body: "One on this computer and one somewhere else. A copy inside the same computer does not protect you.",
  },
  {
    title: "Never delete or rename the root folder by hand.",
    body: "Use OpenFrame to rename, archive or delete projects.",
  },
  {
    title: "Backups inside OpenFrame are separate copies.",
    body: "Create Backup saves a copy; still keep it off this computer.",
  },
];

/** True when the list is scrolled to its end (or is short enough to show whole). */
function atEnd(el: HTMLElement): boolean {
  return el.scrollHeight - el.scrollTop - el.clientHeight <= 4;
}

function RootFolder({ dataRoot, platform, heading }: { dataRoot: string; platform?: string; heading: string }) {
  const reveal = () =>
    void revealLocation("dataRoot").catch((e: unknown) => toast.error(failureText(e, "OpenFrame couldn't open that folder.")));
  return (
    <div className="card pad" style={{ marginTop: 10 }}>
      <div className="h4">{heading}</div>
      <div className="row wrap" style={{ gap: 8 }}>
        <span className="chip selectable-text" style={{ whiteSpace: "normal", wordBreak: "break-all" }}>{dataRoot}</span>
        <span className="grow" />
        <Button size="sm" icon={<FolderOpen size={13} aria-hidden />} onClick={reveal}>
          {revealLabel(platform)}
        </Button>
      </div>
    </div>
  );
}

export function DataNoteDialog({ dataRoot, platform }: { dataRoot: string; platform?: string }) {
  const client = useQueryClient();
  const [readToEnd, setReadToEnd] = useState(false);
  const [ticked, setTicked] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useHoldShortcuts(true);

  // A list that fits without scrolling counts as read. The dialog content mounts in a portal after
  // this component, so the check runs when the list itself arrives.
  const listRef = useCallback((el: HTMLDivElement | null) => {
    if (el && atEnd(el)) setReadToEnd(true);
  }, []);

  const ok = async () => {
    setBusy(true);
    setError(null);
    try {
      await recordNotice(client, "dataNote");
    } catch (e) {
      setError(failureText(e, SAVE_FAILED));
    } finally {
      setBusy(false);
    }
  };
  const canContinue = readToEnd && ticked && !busy;
  const hint = !readToEnd
    ? "Scroll to the end of the list to continue."
    : !ticked
      ? "Tick the box to continue."
      : "You cannot continue until this is acknowledged.";

  return (
    <Dialog
      open
      onOpenChange={() => undefined}
      dismissible={false}
      title="Your data, your responsibility"
      sub="Shown once, right after you buy the full version."
      size="lg"
      footerLeft={<span className="muted xs" aria-live="polite">{hint}</span>}
      footer={
        <Button variant="primary" disabled={!canContinue} onClick={() => void ok()}>
          OK, I understand
        </Button>
      }
    >
      <Banner tone="warn">
        <TriangleAlert size={14} aria-hidden /> <b>Important. Please read before you continue.</b>
      </Banner>
      <div
        ref={listRef}
        role="region"
        aria-label="Six points about your data"
        tabIndex={0}
        className="col"
        style={{ gap: 8, marginTop: 10, maxHeight: 260, overflowY: "auto", paddingRight: 4 }}
        onScroll={(e) => atEnd(e.currentTarget) && setReadToEnd(true)}
      >
        <ol style={{ margin: 0, paddingLeft: 0, listStyle: "none", display: "grid", gap: 8 }}>
          {DATA_POINTS.map((p, i) => (
            <li key={p.title} className="row" style={{ alignItems: "flex-start", gap: 10 }}>
              <span className="chip dark" aria-hidden>{i + 1}</span>
              <div>
                <b>{p.title}</b>
                <div className="sm muted">{p.body}</div>
              </div>
            </li>
          ))}
        </ol>
      </div>
      <RootFolder dataRoot={dataRoot} platform={platform} heading="Your root folder" />
      <div style={{ marginTop: 10 }}>
        <Checkbox
          checked={ticked}
          onChange={setTicked}
          disabled={!readToEnd}
          label={<b>I have read this and I understand that backing up my data is my responsibility.</b>}
        />
      </div>
      {error && (
        <div style={{ marginTop: 8 }}>
          <Banner tone="err">{error}</Banner>
        </div>
      )}
    </Dialog>
  );
}

// ------------------------------------------------------------------ file safety tutorial (194 to 197)

const STEPS = ["Where it lives", "Back up", "New laptop", "Check"] as const;
const CHECKS = [
  "I know where my root folder is.",
  "I will back it up to Google Drive or an external disk regularly.",
  "I know I must copy the folder myself when I change laptops.",
] as const;

export function TutorialDialog({ dataRoot, platform, channel }: { dataRoot: string; platform?: string; channel: Channel }) {
  const client = useQueryClient();
  const [step, setStep] = useState(0);
  const [checks, setChecks] = useState<boolean[]>([false, false, false]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  useHoldShortcuts(true);

  // Each new step moves focus to its heading, so a keyboard or screen reader user starts there.
  useEffect(() => {
    if (step > 0) headingRef.current?.focus();
  }, [step]);

  const finish = async () => {
    setBusy(true);
    setError(null);
    try {
      await recordNotice(client, "tutorial");
    } catch (e) {
      setError(failureText(e, SAVE_FAILED));
    } finally {
      setBusy(false);
    }
  };
  const last = step === STEPS.length - 1;
  const allTicked = checks.every(Boolean);

  let body: ReactNode;
  if (step === 0) {
    body = (
      <>
        <RootFolder dataRoot={dataRoot} platform={platform} heading="Your root folder" />
        <div className="card pad" style={{ marginTop: 10 }}>
          <div className="h4">Inside it</div>
          <div className="sm" style={{ lineHeight: 1.7 }}>
            Projects: one folder for each project, with its data, drafts and attachments
            <br />
            Projects, Backups: copies you make with Create Backup
            <br />
            Global Idea Vault: ideas you keep across projects
          </div>
        </div>
        <p className="sm" style={{ margin: "10px 0 0" }}>Everything you create is inside this one folder. Remember this location.</p>
      </>
    );
  } else if (step === 1) {
    body = (
      <>
        <ol style={{ margin: "0 0 6px 18px", padding: 0, lineHeight: 1.7 }}>
          <li>Install Google Drive for desktop (or your preferred cloud or external-disk tool).</li>
          <li>Choose the root folder shown in step 1 as a folder to sync, or copy it into your Drive folder.</li>
          <li>Check in the Drive website that your projects appear there.</li>
          <li>Repeat after every working session, or turn on automatic sync.</li>
        </ol>
        <Banner tone="info">
          <Info size={14} aria-hidden /> Close OpenFrame (or Create Backup) before copying so files are complete. An external drive works as well as Google Drive.
        </Banner>
      </>
    );
  } else if (step === 2) {
    body = (
      <>
        <ol style={{ margin: "0 0 6px 18px", padding: 0, lineHeight: 1.7 }}>
          <li>On the old laptop copy the whole root folder to a USB disk or Google Drive.</li>
          <li>{newLaptopInstallStep(channel)}</li>
          <li>Copy the folder to the new laptop.</li>
          <li>Open OpenFrame, choose Open Project, and pick the project inside the folder.</li>
        </ol>
        <Banner tone="warn">
          <TriangleAlert size={14} aria-hidden /> Linked external files (for example a reference photo on another drive) must be copied too. OpenFrame lists missing ones after opening.
        </Banner>
      </>
    );
  } else {
    body = (
      <>
        <p className="sm" style={{ margin: "0 0 8px" }}>Confirm that you understand each point.</p>
        <fieldset className="col" style={{ gap: 8, border: 0, padding: 0, margin: 0 }}>
          <legend className="sr-only">Final check</legend>
          {CHECKS.map((label, i) => (
            <Checkbox key={label} checked={checks[i]} onChange={(v) => setChecks((c) => c.map((x, j) => (j === i ? v : x)))} label={label} />
          ))}
        </fieldset>
      </>
    );
  }

  const titles = ["Where your data lives", "Back up the root folder to Google Drive", "Moving to a new laptop", "Final check"];
  return (
    <Dialog
      open
      onOpenChange={() => undefined}
      dismissible={false}
      title="File safety tutorial"
      size="lg"
      footerLeft={<span className="muted xs">Step {step + 1} of 4 · mandatory</span>}
      footer={
        <>
          {step > 0 && <Button onClick={() => setStep(step - 1)}>Back</Button>}
          {last ? (
            <Button variant="primary" disabled={!allTicked || busy} onClick={() => void finish()}>
              Finish
            </Button>
          ) : (
            <Button variant="primary" onClick={() => setStep(step + 1)}>
              Next
            </Button>
          )}
        </>
      }
    >
      <ol className="row" aria-label="Tutorial steps" style={{ marginBottom: 10, listStyle: "none", padding: 0, gap: 6 }}>
        {STEPS.map((s, i) => (
          <li key={s} className={`chip${i === step ? " dark" : i < step ? " g" : ""}`} aria-current={i === step ? "step" : undefined} style={{ padding: "3px 10px" }}>
            {i + 1} {s}
            {i < step && <span className="sr-only"> (done)</span>}
          </li>
        ))}
      </ol>
      <h3 ref={headingRef} tabIndex={-1} style={{ margin: "0 0 8px", fontSize: 18 }}>{titles[step]}</h3>
      {body}
      {error && (
        <div style={{ marginTop: 8 }}>
          <Banner tone="err">{error}</Banner>
        </div>
      )}
    </Dialog>
  );
}

// ------------------------------------------------------------------ welcome (198)

export function WelcomeDialog({ links = SOCIAL_LINKS }: { links?: readonly SocialLink[] }) {
  const client = useQueryClient();
  const [busy, setBusy] = useState(false);
  const close = async () => {
    setBusy(true);
    try {
      await recordNotice(client, "welcome");
    } catch (e) {
      // The welcome is not mandatory: close it for this session either way.
      toast.error(failureText(e, SAVE_FAILED));
      void refreshLicence(client);
    } finally {
      setBusy(false);
    }
  };
  const open = (link: SocialLink) => {
    if (!isFixedSocialUrl(link.url)) return;
    void openUrl(link.url).catch((e: unknown) => toast.error(failureText(e, "OpenFrame couldn't open that link.")));
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !busy && void close()}
      title="Welcome to the OpenFrame Studio family"
      size="lg"
      footer={
        <Button variant="primary" disabled={busy} onClick={() => void close()}>
          OK
        </Button>
      }
    >
      <div style={{ textAlign: "center" }}>
        <Brand />
        <p className="muted" style={{ margin: "8px 0 12px" }}>
          Thank you for backing independent filmmaking. Be a part of our community.
        </p>
      </div>
      <ul className="grid g4" aria-label="Our community" style={{ gap: 12, listStyle: "none", padding: 0, margin: 0 }}>
        {links.map((l) => {
          const content = (
            <>
              <i aria-hidden>{l.short}</i>
              {l.name}
              <span className="muted" style={{ fontWeight: 500 }}>{l.handle}</span>
            </>
          );
          return (
            <li key={l.name}>
              {isFixedSocialUrl(l.url) ? (
                <button type="button" className="soc" aria-label={`${l.name} ${l.handle}, opens in your browser`} onClick={() => open(l)}>
                  {content}
                </button>
              ) : (
                <div className="soc" aria-label={`${l.name} ${l.handle}`}>{content}</div>
              )}
            </li>
          );
        })}
      </ul>
    </Dialog>
  );
}

// ------------------------------------------------------------------ store screens (187, 190, 192)

function useStoreAction() {
  const client = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const run = async (action: (client: QueryClient) => Promise<string | null>) => {
    setBusy(true);
    setMessage(null);
    try {
      const m = await action(client);
      if (m) setMessage(m);
    } catch (e) {
      setMessage(failureText(e, BUY_FAILED));
    } finally {
      setBusy(false);
    }
  };
  return { busy, message, run };
}

const restore = async (client: QueryClient) => {
  await restorePurchase(client);
  return null;
};
const buyHere = (client: QueryClient) => buyFullVersion(client);

function StoreMessage({ message }: { message: string | null }) {
  if (!message) return null;
  return (
    <div style={{ marginTop: 8 }}>
      <Banner tone="warn">{message}</Banner>
    </div>
  );
}

export function MacFirstLaunchDialog() {
  const { busy, message, run } = useStoreAction();
  useHoldShortcuts(true);
  return (
    <Dialog open onOpenChange={() => undefined} dismissible={false} title="Welcome to OpenFrame Studio" sub="How would you like to begin?" size="lg">
      <Brand />
      <div className="grid g2" style={{ gap: 12 }}>
        <section className="card pad" aria-labelledby="mac-trial-h">
          <div id="mac-trial-h" className="b" style={{ fontSize: 16 }}>Free trial</div>
          <div className="sm muted" style={{ margin: "4px 0 10px" }}>7 days. Everything included. No credit card.</div>
          <Button disabled={busy} onClick={() => void run(startTrial)}>Start free trial</Button>
        </section>
        <section className="card pad" aria-labelledby="mac-buy-h" style={{ borderColor: "var(--accent)" }}>
          <div id="mac-buy-h" className="b" style={{ fontSize: 16 }}>Buy the full version</div>
          <div className="sm muted" style={{ margin: "4px 0 10px" }}>Through the App Store, which shows you the details before you buy.</div>
          <Button variant="primary" icon={<Zap size={14} aria-hidden />} disabled={busy} onClick={() => void run(buyHere)}>
            Buy full version
          </Button>
        </section>
      </div>
      <div className="row" style={{ justifyContent: "center", marginTop: 10 }}>
        <Button variant="ghost" size="sm" icon={<RefreshCw size={13} aria-hidden />} disabled={busy} onClick={() => void run(restore)}>
          Restore purchase
        </Button>
      </div>
      <StoreMessage message={message} />
    </Dialog>
  );
}

/** The lock after the trial or a refund in either store (ADR-0022, ADR-0023): only Buy and
 * Restore, and it cannot be closed. The core refuses every project operation while it shows. */
export function LockedDialog({ state, channel }: { state: EntitlementState; channel: Channel }) {
  const { busy, message, run } = useStoreAction();
  useHoldShortcuts(true);
  const revoked = state.state === "revoked";
  const store = storeName(channel);
  const where = channel === "macAppStore" ? "this Mac" : "this computer";
  return (
    <Dialog
      open
      onOpenChange={() => undefined}
      dismissible={false}
      title={revoked ? "Your purchase is no longer active" : "Your free trial has ended"}
      size="md"
      footer={
        <div className="col" style={{ alignItems: "center", gap: 8, width: "100%" }}>
          <Button variant="primary" icon={<Zap size={14} aria-hidden />} disabled={busy} onClick={() => void run(buyHere)}>
            Buy full version
          </Button>
          <Button variant="ghost" size="sm" icon={<RefreshCw size={13} aria-hidden />} disabled={busy} onClick={() => void run(restore)}>
            Restore purchase
          </Button>
        </div>
      }
    >
      <Brand />
      <p style={{ margin: "0 0 8px", textAlign: "center" }}>
        {revoked
          ? `${store.charAt(0).toUpperCase()}${store.slice(1)} reports that this purchase was refunded or revoked. Buy the full version to open your projects again.`
          : "Buy the full version to open your projects again."}
      </p>
      <Banner tone="info">
        <Info size={14} aria-hidden /> Your projects are safe on {where}. Nothing is deleted. Buying or restoring opens them again exactly as you left them.
      </Banner>
      <StoreMessage message={message} />
    </Dialog>
  );
}

/** The licence could not be confirmed online (either store). Closable: opening and exporting keep
 * working (ADR-0018 decision 6). */
export function CheckDialog({ channel, onClose }: { channel: Channel; onClose: () => void }) {
  const { busy, message, run } = useStoreAction();
  const store = storeName(channel);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && !busy && onClose()}
      title="Checking your purchase"
      sub={busy ? `Talking to ${store}…` : undefined}
      size="md"
      footer={
        <>
          <Button disabled={busy} onClick={onClose}>Not now</Button>
          <Button disabled={busy} onClick={() => void run(buyHere)}>Buy full version</Button>
          <Button variant="primary" disabled={busy} onClick={() => void run(restore)}>Try again</Button>
        </>
      }
    >
      {busy ? (
        <div className="bar" role="progressbar" aria-label={`Checking with ${store}`}>
          <i style={{ width: "60%" }} />
        </div>
      ) : (
        <Banner tone="err">
          <b>We could not reach {store}.</b> Connect to the internet to check your purchase. If you bought before, your purchase will be restored as soon as the check succeeds.
        </Banner>
      )}
      <div style={{ marginTop: 8 }}>
        <Banner tone="info">
          <Info size={14} aria-hidden /> Your projects stay open and can be exported. Nothing is deleted.
        </Banner>
      </div>
      <StoreMessage message={message} />
    </Dialog>
  );
}
