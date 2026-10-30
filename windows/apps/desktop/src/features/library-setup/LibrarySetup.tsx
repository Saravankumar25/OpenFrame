// "Set up your OpenFrame library": the first start of a new computer, or a library that went
// missing (docs/engineering/40-storage-locations-and-recovery.md section 1). Shown after the team
// password and before Home whenever library.status is not ready. It cannot be dismissed: closing
// the window closes the app. The Rust core refuses everything else until a choice succeeds, so
// this screen is not the only gate.

import { useCallback, useEffect, useState, type ReactNode } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Banner, Button, Segmented } from "../../design-system";
import { call, OpError } from "../../ipc/client";
import { queryClient } from "../../ipc/query";
import type { LibraryInspection } from "../../ipc/generated/LibraryInspection";
import type { LibraryStatus } from "../../ipc/generated/LibraryStatus";
import { BrandLogo } from "../../theme/BrandLogo";
import { joinPath } from "../packages/api";

const LOAD_FAILED = "OpenFrame could not check where your work is kept. Close OpenFrame and open it again.";

type Choice = "default" | "new" | "locate";

export const messageOf = (e: unknown): string =>
  (e as { message?: string } | null)?.message || "That did not work. Nothing was changed. Try again.";

/** Shows `children` once the library is set up. */
export function LibrarySetupGate({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<LibraryStatus | null>(null);
  const [failed, setFailed] = useState(false);
  const [licenceLocked, setLicenceLocked] = useState(false);
  const load = useCallback(() => {
    setFailed(false);
    call<LibraryStatus>("library.status", {})
      .then(setStatus)
      .catch((e) => (e instanceof OpError && e.is("permission.licence_required") ? setLicenceLocked(true) : setFailed(true)));
  }, []);
  useEffect(load, [load]);

  // The licence lock refuses every operation but its own; the app behind this gate shows the lock
  // dialog (buy or restore), which a library question cannot answer.
  if (status?.state === "ready" || licenceLocked) return <>{children}</>;
  if (!status) {
    return (
      <div className="app-root splash" role={failed ? undefined : "status"}>
        <BrandLogo size="lg" />
        {failed ? (
          <div className="col" style={{ alignItems: "center", gap: 10 }}>
            <div className="splash-sub" role="alert">{LOAD_FAILED}</div>
            <Button onClick={load}>Try again</Button>
          </div>
        ) : (
          <div className="splash-sub">Starting OpenFrame Studio…</div>
        )}
      </div>
    );
  }
  return (
    <LibrarySetupScreen
      status={status}
      onReady={(s) => {
        // Everything read before setup was refused; ask again now.
        void queryClient.invalidateQueries();
        setStatus(s);
      }}
    />
  );
}

export function LibrarySetupScreen({ status, onReady }: { status: LibraryStatus; onReady: (s: LibraryStatus) => void }) {
  const [choice, setChoice] = useState<Choice>(status.found.length > 0 || status.state === "missing" ? "locate" : "default");
  const [parent, setParent] = useState<string | null>(null);
  const [found, setFound] = useState<LibraryInspection | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async (op: string, args: object) => {
    setBusy(true);
    setError(null);
    try {
      const s = await call<LibraryStatus>(op, args);
      if (s.state === "ready") onReady(s);
    } catch (e) {
      setError(messageOf(e));
    } finally {
      setBusy(false);
    }
  };

  const pickParent = async () => {
    const dir = await openDialog({ directory: true, title: "Choose the folder that will hold your OpenFrame library" });
    if (typeof dir !== "string") return;
    setError(null);
    setParent(dir);
    try {
      setFound(await call<LibraryInspection>("library.inspect", { dir }));
    } catch {
      setFound(null);
    }
  };

  const pickExisting = async () => {
    const dir = await openDialog({ directory: true, title: "Choose the folder of your OpenFrame library" });
    if (typeof dir !== "string") return;
    setError(null);
    setFound(null);
    try {
      setFound(await call<LibraryInspection>("library.inspect", { dir }));
    } catch (e) {
      setError(messageOf(e));
    }
  };

  const change = (c: Choice) => {
    setChoice(c);
    setParent(null);
    setFound(null);
    setError(null);
  };

  return (
    <div className="app-root" style={{ overflow: "auto", display: "flex", justifyContent: "center", padding: "40px 16px" }}>
      <main className="col" style={{ maxWidth: 680, width: "100%", gap: 16, margin: "0 auto" }} aria-labelledby="library-setup-title">
        <BrandLogo size="lg" />
        <h1 id="library-setup-title" className="h2" style={{ margin: 0 }}>Set up your OpenFrame library</h1>
        <p style={{ margin: 0 }}>
          OpenFrame keeps your projects and your Global Idea Vault in one folder, your OpenFrame library. Choose where it should be. You can move
          it later in Settings &gt; Storage.
        </p>

        {status.state === "missing" && status.root && (
          <Banner
            tone="warn"
            actions={<Button size="sm" disabled={busy} onClick={() => void run("library.connect", { dir: status.root })}>Try Again</Button>}
          >
            Your library was not found at {status.root}. If it is on an external drive, connect the drive and choose Try Again. If you moved the
            folder, locate it below.
          </Banner>
        )}

        {status.found.length > 0 && (
          <section className="card pad col" style={{ gap: 10 }} aria-label="OpenFrame data found on this computer">
            <div className="h4" style={{ margin: 0 }}>OpenFrame data found on this computer</div>
            {status.found.map((f) => (
              <div key={f.path} className="row" style={{ gap: 12, alignItems: "center", justifyContent: "space-between" }}>
                <div className="col" style={{ gap: 2, minWidth: 0 }}>
                  <div className="sm selectable-text" style={{ wordBreak: "break-all" }}>{f.libraryRoot ?? f.path}</div>
                  <div className="xs muted">
                    {f.message}
                    {f.ownerName && ` It belongs to ${f.ownerName}.`}
                    {f.cloudService && ` This folder is synced by ${f.cloudService}.`}
                  </div>
                </div>
                <Button size="sm" variant="primary" disabled={busy} onClick={() => void run("library.connect", { dir: f.libraryRoot ?? f.path })}>
                  Use This Library
                </Button>
              </div>
            ))}
          </section>
        )}

        <Segmented<Choice>
          ariaLabel="Where to keep your library"
          value={choice}
          onChange={change}
          options={[
            { value: "default", label: "Use the default location" },
            { value: "new", label: "Choose a new location" },
            { value: "locate", label: "Locate existing OpenFrame data" },
          ]}
        />

        <section className="card pad col" style={{ gap: 10 }} aria-label="Your choice">
          {choice === "default" && (
            <>
              <div className="sm selectable-text" style={{ wordBreak: "break-all" }}>{status.suggested}</div>
              <div className="xs muted">
                This folder is in your own user folder, outside Documents. Windows does not sync it to OneDrive, so cloud syncing can never lock or
                replace your files while you work. Back up this folder.
              </div>
              <div>
                <Button variant="primary" disabled={busy} onClick={() => void run("library.use_default", {})}>Create Library Here</Button>
              </div>
            </>
          )}

          {choice === "new" && (
            <>
              <div className="xs muted">Choose a folder. OpenFrame makes a folder named OpenFrame inside it for your library.</div>
              <div>
                <Button icon={<FolderOpen size={14} />} disabled={busy} onClick={() => void pickParent()}>Choose Folder…</Button>
              </div>
              {parent && (
                <>
                  <div className="sm">
                    Your library will be <span className="selectable-text" style={{ wordBreak: "break-all" }}>{joinPath(parent, "OpenFrame")}</span>
                  </div>
                  {found?.cloudService && (
                    <Banner tone="warn">
                      That folder is synced by {found.cloudService}. Syncing can lock or replace files while OpenFrame writes to them. A folder
                      that is not synced is safer.
                    </Banner>
                  )}
                  <div>
                    <Button variant="primary" disabled={busy} onClick={() => void run("library.create_at", { parent })}>Create Library</Button>
                  </div>
                </>
              )}
            </>
          )}

          {choice === "locate" && (
            <>
              <div className="xs muted">
                After a reinstall, or when you moved your library or brought it from another drive, choose its folder. OpenFrame shows what it
                found before connecting anything, and never makes an empty library in its place.
              </div>
              <div>
                <Button icon={<FolderOpen size={14} />} disabled={busy} onClick={() => void pickExisting()}>Choose Folder…</Button>
              </div>
              {found && (
                <div className="col" style={{ gap: 8 }} aria-label="What was found">
                  <Banner tone={found.libraryRoot ? "info" : "warn"}>
                    {found.message}
                    {found.ownerName && ` It belongs to ${found.ownerName}.`}
                  </Banner>
                  {found.libraryRoot && (
                    <div>
                      <Button variant="primary" disabled={busy} onClick={() => void run("library.connect", { dir: found.libraryRoot })}>
                        Connect Library
                      </Button>
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </section>

        {error && <Banner tone="err">{error}</Banner>}
      </main>
    </div>
  );
}
