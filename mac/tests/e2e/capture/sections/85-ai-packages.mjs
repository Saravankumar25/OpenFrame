// WindowsHTML capture section: AI Assistant panel (ready, scopes, a few answers, See sources open, proposed changes), Offline AI states, backup, full project export, exchange and review packages, licence and trial screens through entitlement.debug_set.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

import { results } from "../../lib/ai.mjs";

const PANEL = '//aside[@aria-label="AI Assistant"]';
const DIALOG = '//*[@role="dialog"]';
const ANSWER_TIMEOUT = 240_000;

export default async function (t) {
  const b = t.browser;
  const text = (sel) => b.execute((s) => document.querySelector(s)?.innerText ?? "", sel);
  const panelHas = async (s) => (await text('aside[aria-label="AI Assistant"]')).includes(s);

  // ------------------------------------------------------------------ AI Assistant panel

  const openPanel = async () => {
    if (!(await (await b.$(PANEL)).isExisting())) await t.press("Assistant");
    await (await t.el(`${PANEL}//textarea[@aria-label="Ask or command"]`, 15_000)).waitForDisplayed();
    await t.sleep(500);
  };
  const closePanel = async () => {
    if (await (await b.$(PANEL)).isExisting()) {
      await t.press("Close AI Assistant");
      await (await b.$(PANEL)).waitForExist({ reverse: true, timeout: 10_000 });
    }
  };
  const setScope = async (label) => {
    const sel = await t.el(`${PANEL}//select[@id="ai-scope"]`);
    await sel.selectByVisibleText(label);
    await t.sleep(300);
  };
  const newConversation = async () => {
    await t.press("New", PANEL);
    await t.sleep(300);
  };
  const typeQuestion = async (q) => {
    const box = await t.el(`${PANEL}//textarea[@aria-label="Ask or command"]`);
    await box.waitForEnabled({ timeout: 30_000 });
    await box.click();
    await box.setValue(q);
  };
  const waitAnswer = async (q, before) => {
    await b.waitUntil(
      async () =>
        (await results(b)).length > before &&
        (await b.execute((asked) => {
          const aside = document.querySelector('aside[aria-label="AI Assistant"]');
          if (!aside || aside.querySelector('.msg.a.muted[role="status"]')) return false;
          const all = [...aside.querySelectorAll(".msg.a[data-kind]")];
          const prev = all[all.length - 1]?.previousElementSibling;
          return !!prev && prev.matches(".msg.u") && (prev.textContent ?? "").trim() === asked.trim();
        }, q)),
      { timeout: ANSWER_TIMEOUT, interval: 1_000, timeoutMsg: `no answer to "${q}" within ${ANSWER_TIMEOUT / 1000} s` },
    );
    await t.sleep(500);
    const all = await results(b);
    return all[all.length - 1];
  };
  /** Ask in a fresh conversation. Returns the result, or null when no answer came (recorded as a failed state). */
  const ask = async (q, doneName, { workingName, typedName, after } = {}) => {
    try {
      await openPanel();
      await newConversation();
      const before = (await results(b)).length;
      if (typedName) await t.state(typedName, () => typeQuestion(q));
      else await typeQuestion(q);
      await t.press("Send", PANEL);
      if (workingName && (await panelHas("Working on it"))) await t.state(workingName, null);
      const r = await waitAnswer(q, before);
      await t.state(doneName, null, { wait: 800 });
      if (after) await after(r);
      return r;
    } catch (e) {
      await t.state(doneName, async () => {
        throw e;
      });
      return null;
    }
  };
  const openSources = async () => {
    await t.go(`(${PANEL}//details[contains(@class,"ai-details")]/summary)[last()]`);
    await t.sleep(300);
  };

  const aiPart = async () => {
    // Ready, menu, scopes.
    await t.state("ai-panel-ready", async () => {
      await openPanel();
      // The project context is prepared in the first seconds; a ready panel has no notice.
      await b.waitUntil(async () => !(await panelHas("Preparing project context")), { timeout: 90_000, interval: 1_000 }).catch(() => undefined);
    }, { wait: 1500 });
    await t.state("ai-settings-menu", () => t.press("Offline AI settings"), { blur: false });
    await t.closeOverlays();
    await t.state("ai-scope-current-screenplay", () => setScope("Current Screenplay"));
    await t.state("ai-scope-beat-board", () => setScope("Beat Board"));
    await setScope("Whole Project");

    // Offline AI settings, reached from the panel's menu, before any request exists.
    await t.state("ai-offline-settings", async () => {
      await t.press("Offline AI settings");
      await t.menuItem("Offline AI settings");
      await t.el('//section[@aria-label="Offline AI"]', 15_000);
      await t.waitText("No requests yet");
      await t.waitText("AI Ready");
    }, { wait: 1000 });
    await t.state("ai-offline-technical-details", async () => {
      await t.go('//section[@aria-label="Technical details"]//summary');
      await t.el('//table[@aria-label="Offline AI components"]');
    });
    await t.go('//section[@aria-label="Technical details"]//summary');
    await t.state("ai-offline-remove-confirm", () => t.press("Remove OpenFrame AI…"));
    await t.press("Cancel", DIALOG);
    await t.workspace("Home");

    // A list answer from the Whole Project scope.
    await ask("List the characters.", "ai-answer-characters", {
      typedName: "ai-question-typed",
      workingName: "ai-working",
      after: () => t.state("ai-answer-characters-sources", async () => {
        const has = await b.$(`${PANEL}//details[contains(@class,"ai-details")]`);
        if (await has.isExisting()) await openSources();
      }),
    });

    // Questions asked inside the Screenplay.
    await t.workspace("Screenplay");
    await openPanel();
    await t.state("ai-panel-screenplay", async () => {
      await newConversation();
    });
    await setScope("Current Screenplay");
    await ask("Which locations are used in the screenplay?", "ai-answer-locations", {
      after: async () => {
        const has = await b.$(`${PANEL}//details[contains(@class,"ai-details")]`);
        if (await has.isExisting()) await t.state("ai-answer-locations-sources-open", openSources);
      },
    });
    await ask("How many scenes are in the script?", "ai-answer-scene-count");

    // Proposed changes: reject one, apply a harmless one and undo it.
    await setScope("Whole Project");
    const first = await ask("Create a task to scout the railway station.", "ai-proposed-changes");
    if (first?.kind === "Proposal" && (await panelHas("Reject"))) {
      await t.state("ai-proposal-rejected", () => t.press("Reject", PANEL));
    }
    const second = await ask("Create a task to book the bus for the night shoot.", "ai-proposed-changes-second");
    if (second?.kind === "Proposal" && (await panelHas("Apply Changes"))) {
      await t.state("ai-proposal-applied", async () => {
        await t.press("Apply Changes", PANEL);
        await t.waitText("Applied");
      }, { keepToasts: true });
      await t.op("history.undo", { store: "project" }).catch(() => undefined);
      await t.sleep(500);
    }

    // A request to delete: the answer, and when a deleting proposal comes, its confirmation (cancelled, then rejected).
    const del = await ask("Delete all the scenes in the screenplay.", "ai-answer-delete-request");
    if (del?.kind === "Proposal" && (await b.$(`${PANEL}//*[contains(@class,"ai-destructive")]`).isExisting())) {
      await t.state("ai-delete-confirm", () => t.press("Apply Changes", PANEL));
      await t.press("Cancel", DIALOG);
      await t.press("Reject", PANEL);
    } else if (del?.kind === "Proposal" && (await panelHas("Reject"))) {
      await t.press("Reject", PANEL);
    }

    // The panel docked beside other workspaces (a fresh conversation each).
    for (const [label, name] of [
      ["Story", "ai-panel-story"],
      ["Breakdown", "ai-panel-breakdown"],
      ["Production", "ai-panel-production"],
      ["Idea Vault", "ai-panel-vault"],
    ]) {
      await t.state(name, async () => {
        if (label === "Story") await t.storyTab("Story");
        else await t.workspace(label);
        await openPanel();
        await newConversation();
      });
    }

    // Offline AI settings again: the health figures now hold the requests above.
    await t.state("ai-offline-health-filled", async () => {
      await t.press("Offline AI settings");
      await t.menuItem("Offline AI settings");
      await t.el('//section[@aria-label="AI health"]', 15_000);
      await t.waitText("REQUESTS COUNTED");
    }, { wait: 1000 });
    await t.workspace("Home");

    // The panel in the empty project.
    await closePanel();
    await t.inEmptyProject(async () => {
      await t.state("ai-panel-empty-project", openPanel, { wait: 1500 });
      await closePanel();
    });
    await closePanel();
  };

  // ------------------------------------------------------------------ packages

  const dialogOpen = (title) => t.el(t.dialog(title), 15_000);
  const closeDialog = async () => {
    await t.closeOverlays();
  };

  const packagesPart = async () => {
    await t.state("pkg-project-menu", () => t.go('//button[starts-with(@aria-label,"Project: ")]'), { blur: false });
    await t.closeOverlays();

    await t.state("pkg-backup-dialog", async () => {
      await t.projectMenu("Create Backup…");
      await dialogOpen("Create Backup");
      await t.waitText("WHAT THIS BACKUP CONTAINS");
    });
    await closeDialog();

    await t.state("pkg-export-project-dialog", async () => {
      await t.projectMenu("Export Full Project…");
      await dialogOpen("Export Full Project");
      await t.waitText("Private project transfer.");
    });
    await closeDialog();

    const scope = (label) => t.press(label, '//*[@role="radiogroup"][@aria-label="Scope"]');
    await t.state("pkg-export-script-dialog", async () => {
      await t.projectMenu("Export Review Package (script)…");
      await dialogOpen("Script Review Package");
      await t.waitText("THIS PACKAGE WILL CONTAIN");
    });
    await t.state("pkg-export-script-comments", async () => {
      await scope("Script + comments");
      await t.waitText("THIS PACKAGE WILL CONTAIN");
    });
    await t.state("pkg-export-script-no-scene", async () => {
      await scope("Specific scenes only");
      await t.waitText("Choose at least one scene.");
    });
    await t.state("pkg-export-script-scene-chosen", async () => {
      await t.go(`(${DIALOG}//div[contains(@class,"card")]//label[contains(@class,"check")])[1]`);
      await t.waitText("THIS PACKAGE WILL CONTAIN");
    });
    await closeDialog();

    await t.state("pkg-export-beat-board-dialog", async () => {
      await t.projectMenu("Beat Board Package…");
      await dialogOpen("Beat Board Package");
      await t.waitText("THIS PACKAGE WILL CONTAIN");
    });
    await t.state("pkg-export-beat-board-one-act", async () => {
      await scope("One Act");
      await t.waitText("THIS PACKAGE WILL CONTAIN");
    });
    await closeDialog();

    await t.state("pkg-import-exchange-choose", async () => {
      await t.projectMenu("Import Exchange Package…");
      await dialogOpen("Import Exchange Package");
    });
    await closeDialog();

    t.unreachable("pkg-open-review-package", "Open Review Package opens the Windows file picker straight away, with no dialog of its own first; the viewer needs a .ofscriptreview file chosen in that native picker");
    t.unreachable("pkg-backup-save-location", "Browse in Create Backup opens the native save dialog (Save backup to); the location, progress, done and cancelled states need a file chosen there");
    t.unreachable("pkg-export-project-save-location", "Export Project Package opens the native save dialog; the progress and Project package exported states need a file chosen there");
    t.unreachable("pkg-export-package-save-location", "Export Package opens the native save dialog; the Package exported state needs a file chosen there");
    t.unreachable("pkg-import-exchange-review", "Choose Package opens the native open dialog; the preview, stale-package choice and result need a package from a second project");
    t.unreachable("pkg-open-project-choose-file", "Choose Package opens the native open dialog; the package details, collision choice and Back Up and Replace confirmation need a package chosen there");

    await t.state("pkg-review-queue-empty", async () => {
      await t.projectMenu("Review Queue");
      await dialogOpen("Review Queue");
      await t.waitText("Nothing waiting");
    });
    await closeDialog();

    await t.state("pkg-history-imports", async () => {
      await t.projectMenu("Import & Export History");
      await dialogOpen("Import & Export History");
      await t.waitText("No imports yet");
    });
    for (const [label, name, empty] of [
      ["Review records", "pkg-history-review-records", "No review records"],
      ["My reviews", "pkg-history-my-reviews", "No open reviews"],
      ["Package log", "pkg-history-package-log", "No packages yet"],
    ]) {
      await t.state(name, async () => {
        await t.press(label, DIALOG);
        await t.waitText(empty);
      });
    }
    await closeDialog();

    await t.state("pkg-open-project-package-dialog", async () => {
      await t.projectMenu("Open Project Package…");
      await dialogOpen("Open Project Package");
      await t.waitText("Choose a package");
    });
    await closeDialog();
  };

  // ------------------------------------------------------------------ licence and trial screens

  const setLicence = async (channel, state, { resetNotices = false, locked = false } = {}) => {
    const mode = await b.execute(() => document.documentElement.dataset.mode);
    await t.op("entitlement.debug_set", { channel, state, resetNotices });
    await b.refresh();
    await b.waitUntil(async () => (await b.execute(() => document.readyState)) === "complete", { timeout: 30_000 });
    if (!locked) await b.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
    await t.sleep(800);
    if (locked) {
      // The core refuses every operation under the lock, the appearance ones too: the capture only needs the page to switch Light and Dark, so the page answers those two itself.
      // The webview's invoke cannot be replaced, so the two appearance calls are answered at the page's fetch, where the IPC request leaves.
      await b.execute((m) => {
        const root = document.documentElement;
        root.setAttribute("data-mode", m);
        const real = window.fetch.bind(window);
        window.fetch = (input, init) => {
          const url = typeof input === "string" ? input : input.url;
          if (/of_invoke/.test(url) && typeof init?.body === "string" && /"op":"appearance.set_/.test(init.body)) {
            const call = JSON.parse(init.body);
            if (call.op === "appearance.set_mode") root.setAttribute("data-mode", call.args.mode);
            return Promise.resolve(new Response("null", { headers: { "Content-Type": "application/json", "Tauri-Response": "ok" } }));
          }
          return real(input, init);
        };
      }, mode);
    } else if (mode && (await b.execute(() => document.documentElement.dataset.mode)) !== mode) {
      await t.op("appearance.set_mode", { mode });
    }
  };
  const licenceDialog = (title) => t.el(t.dialog(title), 20_000);
  const chip = '//*[@role="group"][@aria-label="Free trial"]';

  const licencePart = async () => {
    const start = await t.op("entitlement.status", {});
    try {
      await t.state("licence-trial-chip", async () => {
        await setLicence("microsoftStore", { state: "trialActive", days_left: 7 }, { resetNotices: true });
        await t.el(chip, 15_000);
      });
      await t.state("licence-buy-unavailable", async () => {
        await t.press("Buy full version");
        await t.waitText("The Microsoft Store connection is not available in this build.");
      }, { keepToasts: true });

      await t.state("licence-trial-reminder", async () => {
        await setLicence("microsoftStore", { state: "trialActive", days_left: 3 }, { resetNotices: true });
        await licenceDialog("Trial reminder");
        await t.waitText("Your free trial ends in 3 days");
      });
      await t.state("licence-trial-ending-chip", async () => {
        await t.press("Continue trial", DIALOG);
        await t.closeOverlays();
        await t.el(chip);
      });

      await t.state("licence-data-note", async () => {
        await setLicence("microsoftStore", { state: "purchased" }, { resetNotices: true });
        await licenceDialog("Your data, your responsibility");
        await t.waitText("Show in File Explorer");
      });
      await t.state("licence-data-note-read", async () => {
        await b.execute(() => {
          const el = document.querySelector('[role="region"][aria-label="Six points about your data"]');
          el.scrollTop = el.scrollHeight;
          el.dispatchEvent(new Event("scroll"));
        });
        const box = await b.$(`${DIALOG}//input[@type="checkbox"]`);
        await b.waitUntil(() => box.isEnabled(), { timeout: 5_000, timeoutMsg: "the data note box stays disabled after reading" });
      });
      await t.state("licence-data-note-confirmed", async () => {
        await t.go(`${DIALOG}//input[@type="checkbox"]`);
        await b.waitUntil(async () => (await b.$(`${DIALOG}//button[normalize-space(.)="OK, I understand"]`)).isEnabled(), { timeout: 5_000 });
      });
      await t.press("OK, I understand", DIALOG);

      await t.state("licence-tutorial-step-1", () => licenceDialog("File safety tutorial"));
      for (const n of [2, 3, 4]) {
        await t.state(`licence-tutorial-step-${n}`, async () => {
          await t.press("Next", DIALOG);
          await t.waitText(`Step ${n} of 4`);
        });
      }
      await t.state("licence-tutorial-step-4-ticked", async () => {
        for (const box of await b.$$(`${DIALOG}//input[@type="checkbox"]`)) await box.click();
        await b.waitUntil(async () => (await b.$(`${DIALOG}//button[normalize-space(.)="Finish"]`)).isEnabled(), { timeout: 5_000 });
      });
      await t.state("licence-welcome-family", async () => {
        await t.press("Finish", DIALOG);
        await licenceDialog("Welcome to the OpenFrame Studio family");
        await t.waitText("Be a part of our community.");
      });
      await t.press("OK", DIALOG);
      await t.sleep(500);

      await t.state("licence-check-needed", async () => {
        await setLicence("microsoftStore", { state: "needsOnlineCheck" });
        await licenceDialog("Checking your purchase");
      });
      await t.state("licence-check-purchase-button", async () => {
        await t.press("Not now", DIALOG);
        await t.el('//*[@role="group"][@aria-label="Licence"]');
      });
      await t.go('//*[@role="group"][@aria-label="Licence"]//button[normalize-space(.)="Check purchase"]');
      await licenceDialog("Checking your purchase");
      await t.keys("Escape");

      await t.state("licence-locked-trial-ended-microsoft-store", async () => {
        await setLicence("microsoftStore", { state: "trialEnded" }, { locked: true });
        await licenceDialog("Your free trial has ended");
      });
      await t.state("licence-locked-buy-unavailable", async () => {
        await t.press("Buy full version", DIALOG);
        await t.waitText("The Microsoft Store connection is not available in this build.");
      }, { keepToasts: true });
      await t.state("licence-locked-revoked", async () => {
        await setLicence("microsoftStore", { state: "revoked" }, { locked: true });
        await licenceDialog("Your purchase is no longer active");
      });
      t.unreachable("licence-mac-first-launch", "The Mac first-launch screen only exists on the Mac App Store channel, which is a Mac shell; it is captured on the Mac track");
    } finally {
      await setLicence(start.channel, start.state, { resetNotices: true }).catch(() => undefined);
    }
  };

  t.unreachable("ai-scope-options-open", "The Scope list is a native select; its drop-down is drawn by Windows outside the page and does not appear in a page capture. Each scope is captured as the chosen value instead");
  t.unreachable("ai-offline-download-states", "The download, paused, failed, not enough space and damaged states of OpenFrame AI need the model host steered through a failed or interrupted download; this tour runs with the models installed");
  t.unreachable("ai-offline-not-downloaded", "Needs a run without the AI models installed; this tour runs with them");
  t.unreachable("ai-export-report", "Export AI report opens the native save dialog; the saved toast needs a file chosen there");
  t.unreachable("ai-runtime-failed", "The runtime failed banner needs the AI engine to crash mid-answer (covered by the crash-safe apply spec, not by a capture)");

  try {
    await aiPart();
  } catch (e) {
    await t.state("ai-part-stopped", async () => {
      throw e;
    });
  }
  await t.reset();
  await closePanel();
  await t.reset();

  try {
    await packagesPart();
  } catch (e) {
    await t.state("pkg-part-stopped", async () => {
      throw e;
    });
  }
  await t.reset();

  try {
    await licencePart();
  } catch (e) {
    await t.state("licence-part-stopped", async () => {
      throw e;
    });
  }
  await t.reset();
}
