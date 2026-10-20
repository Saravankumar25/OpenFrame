// WindowsHTML capture section: Screenplay: editor, zoom, Scenes panel and menus, element menu, suggestions, find and replace, view menu, Writing Room and Focus modes, every panel, drafts, compare, lock and revisions, title page, page setup, custom elements, scene number, scene hub, picker dialogs, export and print preview, import failure.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

export default async function (t) {
  const b = () => t.browser;
  const xq = t.xq;
  const TOOLBAR = '//*[@role="toolbar"][@aria-label="Screenplay"]';
  const WR_TABS = '//*[@role="toolbar"][@aria-label="Writing Room panels"]';
  const DOC = '//div[contains(@class,"spx-doc")]';
  const PANELS = '//aside[@aria-label="Screenplay panels"]';
  const DIALOG = '//*[@role="dialog"]';
  const DRAWER = '//aside[contains(@class,"drawer")]';
  const VIEW = '//button[@aria-label="View and more"]';
  const DRAFT_BTN = '//button[starts-with(@aria-label,"Draft: ")]';
  const PICKER_BTN = '//button[starts-with(@aria-label,"Screenplay: ")]';
  const ACTION = `${DOC}//div[@data-type="action"]`;
  const CHARACTER = `${DOC}//div[@data-type="character"]`;
  const jump = (n) => `(//nav[@aria-label="Scenes"]//button[contains(@class,"spx-sni-main")])[${n}]`;
  const row = (n) => `(//nav[@aria-label="Scenes"]//div[contains(@class,"spx-sni")])[${n}]`;
  const tb = (prefix) => `${TOOLBAR}//button[starts-with(normalize-space(.),${xq(prefix)})]`;
  const mi = (text) => `//*[@role="menuitem" or @role="menuitemcheckbox" or @role="menuitemradio"][contains(normalize-space(.),${xq(text)})]`;
  const radio = (prefix) => `//*[@role="radio"][starts-with(normalize-space(.),${xq(prefix)})]`;
  const closeX = `${DRAWER}//button[@aria-label="Close"]`;

  /** Click without noting the click as the link to the next screen (set-up steps). */
  async function q(sel) {
    const el = await t.el(sel);
    await el.scrollIntoView({ block: "center" }).catch(() => undefined);
    await el.click();
    await t.sleep(250);
  }
  const esc = async () => {
    await b().keys("Escape");
    await t.sleep(300);
  };
  const cancel = (within = DIALOG) => q(t.btn("Cancel", within));
  const saved = () => b().execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");
  async function waitSaved() {
    if (!(await b().$(".sp-top").isExisting())) return;
    await b().waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000 }).catch(() => undefined);
  }
  async function ready() {
    await b().$(`${DOC}//div[@data-type="scene_heading"]`).waitForExist({ timeout: 25_000, timeoutMsg: "the screenplay editor did not open" });
    await t.sleep(1500);
  }
  async function selectWords(el) {
    await b().execute((node) => {
      const w = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
      let tn = w.nextNode();
      while (tn && !tn.textContent.trim()) tn = w.nextNode();
      if (!tn) return;
      const s = tn.textContent;
      const m = /^(\s*\S+\s+)(\S+\s+\S+)/.exec(s);
      const r = document.createRange();
      r.setStart(tn, m ? m[1].length : 0);
      r.setEnd(tn, m ? m[0].length : Math.min(s.length, 8));
      const sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(r);
    }, el);
  }
  async function phase(name, fn) {
    try {
      await fn();
    } catch (e) {
      await t.state(`sp-${name}-phase-stopped`, async () => {
        throw e;
      });
    }
    await t.closeOverlays().catch(() => undefined);
  }

  const ov = await t.op("screenplay.overview", {});
  const spId = ov.screenplay.id;
  const spTitle = ov.screenplay.title;
  const second = ov.drafts.find((d) => d.name === "Second Draft") ?? ov.drafts.find((d) => d.isCurrent);
  const first = ov.drafts.find((d) => d.id !== second.id);
  const scene1Id = (await t.op("screenplay.document", { draftId: second.id })).scenes[0].id;

  /** Standard layout on the Second Draft at 100 percent, read fresh from the saved layout. */
  async function freshLayout(open = true) {
    await t.closeOverlays().catch(() => undefined);
    await t.sleep(800);
    await waitSaved();
    await t.op("screenplay.set_view_state", {
      state: { mode: "standard", panels: [], showNotes: true, draftId: second.id, screenplayId: spId, navColumns: [], navHidden: false },
    });
    await b().execute(() => {
      try {
        localStorage.setItem("openframe.screenplay.zoom", "1");
      } catch {
        // View preference only.
      }
    });
    await b().refresh();
    await b().$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 30_000 });
    await t.sleep(800);
    if (!open) return;
    if (!(await b().$('//main[@aria-label="Screenplay"]').isExisting())) await q('//nav[@aria-label="Workspaces"]//button[normalize-space(.)="Screenplay" or @aria-label="Screenplay"]');
    await ready();
  }
  async function switchDraft(name) {
    await q(DRAFT_BTN);
    await q(`//*[@role="menuitem"][starts-with(normalize-space(.),${xq(name)})]`);
    await b().waitUntil(async () => ((await (await b().$(DRAFT_BTN)).getAttribute("aria-label")) ?? "").startsWith(`Draft: ${name}.`), { timeout: 10_000 });
    await t.sleep(1500);
  }
  async function viewItem(text) {
    await q(VIEW);
    await t.menuItem(text);
  }

  // ---------------------------------------------------------------- the editor
  await phase("editor", async () => {
    await freshLayout(false);
    await t.workspace("Screenplay");
    await ready();
    await t.state("sp-editor-standard", null, { home: "Screenplay" });
    await t.state("sp-editor-scrolled-mid-script", () => t.go(jump(18)), { wait: 1000 });
    await q(jump(1));
    await t.sleep(600);

    await t.state("sp-zoom-50", async () => {
      await t.press("Zoom out", TOOLBAR);
      for (let i = 0; i < 4; i++) await q(t.btn("Zoom out", TOOLBAR));
    });
    await q(t.btn("Reset the page zoom", TOOLBAR));
    await t.state("sp-zoom-200", async () => {
      await t.press("Zoom in", TOOLBAR);
      for (let i = 0; i < 9; i++) await q(t.btn("Zoom in", TOOLBAR));
    });
    await q(t.btn("Reset the page zoom", TOOLBAR));
    await q(jump(1));

    // Scene navigator: columns menu, the columns, and hiding the list.
    const colBtn = '//button[@aria-label="Navigator columns"]';
    await t.state("sp-navigator-columns-menu", () => t.go(colBtn));
    await t.go(mi("Page"));
    await t.sleep(900);
    await q(colBtn);
    await q(mi("Notes"));
    await t.sleep(900);
    await t.state("sp-navigator-columns-menu-checked", () => t.go(colBtn));
    await esc();
    await t.state("sp-navigator-columns-shown", null, { wait: 800 });
    await q(colBtn);
    await q(mi("Page"));
    await q(colBtn);
    await q(mi("Notes"));
    await t.sleep(600);
    await t.state("sp-navigator-hidden", () => t.press("Scenes", TOOLBAR));
    await q(t.btn("Scenes", TOOLBAR));
    await t.sleep(400);

    // Scene context menu, Scene Hub, scene number and the reorder warning.
    await t.state("sp-scene-context-menu", () => t.rightClick(row(1)));
    await t.state("sp-scene-hub-drawer", async () => {
      await t.menuItem("Scene Hub");
      await t.el(`${DRAWER}//*[contains(normalize-space(.),"What exists around this scene?")]`);
      await t.sleep(900);
    }, { wait: 800 });
    await q(closeX);
    await t.rightClick(row(1));
    await t.menuItem("Scene number…");
    await t.state("sp-scene-number-dialog", () => t.el("#sn-value"));
    await t.state("sp-scene-number-warning", async () => {
      await t.type("#sn-value", "1A");
      await t.press("Save", t.dialog("Scene number"));
      await t.el(t.btn("Change the number", t.dialog("Scene number")));
    });
    if (await b().$(DIALOG).isExisting()) await cancel();
    else await t.op("screenplay.set_scene_number", { sceneId: scene1Id, acknowledgeDependencies: true }).catch(() => undefined);
    await t.sleep(300);
    await t.state("sp-reorder-warning-dialog", async () => {
      await t.rightClick(row(1));
      await t.menuItem("Move down");
      await t.el(t.dialog("This scene is already used in production planning"));
    });
    if (await b().$(DIALOG).isExisting()) await cancel();
    else await t.op("screenplay.move_scene", { sceneId: scene1Id, index: 0 }).catch(() => undefined);
    await t.sleep(300);

    // Element type menu.
    await q(ACTION);
    await t.state("sp-element-type-menu", () => t.go('//button[starts-with(@aria-label,"Element type: ")]'));
    await esc();
  });

  // ------------------------------------------------- suggestions and comments on text
  await phase("typing", async () => {
    const el = await t.el(CHARACTER);
    const orig = (await el.getText()).trim();
    await t.state("sp-suggestion-list", async () => {
      await q(CHARACTER);
      await t.keys("End");
      await t.keys(["Shift", "Home"]);
      await b().keys(["L", "A"]);
      await t.el('//ul[@id="sp-suggest-list"]');
      await t.sleep(300);
    }, { blur: false });
    await t.state("sp-suggestion-list-create-new", async () => {
      await b().keys(["Z", "Q"]);
      await t.sleep(400);
      await t.el('//ul[@id="sp-suggest-list"]');
    }, { blur: false });
    for (let i = 0; i < 6; i++) {
      if (((await (await t.el(CHARACTER)).getText()) ?? "").trim() === orig) break;
      await b().keys(["Control", "z"]);
      await t.sleep(400);
    }
    await waitSaved();

    await t.state("sp-comment-float-button", async () => {
      await q(ACTION);
      await selectWords(await t.el(ACTION));
      await t.el('//button[contains(@class,"spx-cmt-btn")]');
    }, { blur: false });
    await t.state("sp-comment-composer-text", async () => {
      await t.go('//button[contains(@class,"spx-cmt-btn")]');
      await t.type('//textarea[@aria-label="Comment"]', "Is this the same dog we see at the bus stand later?");
    });
    await q(t.btn("Cancel", PANELS));
    await t.state("sp-standard-comments-panel", null, { wait: 800 });
    await q(`${PANELS}//button[@aria-label="Close panel"]`);
  });

  // ------------------------------------------------------------------ find and replace
  await phase("find", async () => {
    await t.state("sp-find-empty", () => t.press("Find", TOOLBAR));
    await t.state("sp-find-results", async () => {
      await t.type('//input[@aria-label="Find"]', "radio");
      await b().waitUntil(async () => /\d+ of \d+/.test(await t.text(".spx-count")), { timeout: 8000, timeoutMsg: "no find count" });
    });
    await t.state("sp-find-replace-filled", () => t.type('//input[@aria-label="Replace with"]', "receiver"));
    await t.state("sp-find-no-results", async () => {
      await t.type('//input[@aria-label="Find"]', "zzqx");
      await t.waitText("No matches found");
    });
    await t.state("sp-find-this-scene", async () => {
      await t.type('//input[@aria-label="Find"]', "the");
      await t.press("This scene");
      await t.sleep(500);
    });
    await t.press("Close find (Esc)");
    await t.sleep(300);
  });

  // ---------------------------------------------------- view menu and its dialogs
  await phase("dialogs", async () => {
    await t.state("sp-view-menu", () => t.go(VIEW));
    await t.state("sp-title-page-dialog", async () => {
      await t.menuItem("Title page…");
      await t.el("#tp-title");
      await t.el(`${DIALOG}//*[@aria-label="Title page preview"]//img | ${DIALOG}//*[@aria-label="Title page preview"]//*[contains(@class,"ix-page")]`).catch(() => undefined);
      await t.sleep(900);
    }, { wait: 900 });
    await t.state("sp-title-page-empty-title", () => t.type("#tp-title", ""));
    await cancel();

    await viewItem("Page setup…");
    await t.state("sp-page-setup-dialog", () => t.el("#ps-paper"));
    await t.state("sp-page-setup-header-filled", async () => {
      await q(t.btn("Insert {title} into the header"));
      await q(t.btn("Insert {draft} into the header"));
      await q(t.btn("Insert {page} into the footer"));
    });
    await cancel();

    await viewItem("Custom elements…");
    await t.state("sp-custom-elements-empty", () => t.waitText("No custom elements yet."));
    await t.state("sp-custom-elements-add-form", () => t.press("Add custom element"));
    await cancel(t.dialog("Custom elements"));
    await q(t.btn("Done", DIALOG));
    await t.sleep(300);

    await viewItem("Page view");
    await t.state("sp-page-view-drawer", async () => {
      await t.el(`${DRAWER}//*[@aria-label="Pages"]//*[@role="img"]`);
      await t.sleep(900);
    }, { wait: 800 });
    await q(closeX);

    // F7 finds the next spelling mistake; with none it says so.
    await q(ACTION);
    await b().keys("");
    await t.sleep(700);
    if (await b().$('//*[@role="menu"][starts-with(@aria-label,"Spelling:")]').isExisting()) {
      await t.state("sp-spell-menu", null, { blur: false, keepToasts: true });
      await esc();
    } else {
      await t.state("sp-spelling-none-found", null, { blur: false, keepToasts: true, wait: 200 });
      t.unreachable("sp-spell-menu", "The spelling menu needs an underlined word and an installed dictionary; the demo script has no mistake, so F7 only reports that none were found.");
    }
  });

  // -------------------------------------------- Writing Room, every panel, Focus mode
  await phase("writing-room", async () => {
    await freshLayout();
    await q(jump(1));
    await t.sleep(500);
    await viewItem("Writing Room Mode");
    await t.sleep(900);
    await t.state("sp-writing-room-beat-board", null, { wait: 800 });
    await t.state("sp-writing-room-beat-board-all-cards", () => t.go('//button[starts-with(normalize-space(.),"Show all Beat Board cards")]'));
    await t.state("sp-writing-room-no-panel", () => t.press("Beat Board", WR_TABS));

    await t.state("sp-panel-scene-notes", () => t.press("Scene Notes", WR_TABS), { wait: 800 });
    await t.state("sp-panel-private-note-menu", () => t.go('//button[@aria-label="Private note actions"]'));
    await esc();
    await t.state("sp-panel-private-note-composer", async () => {
      await t.press("Add private note", PANELS);
      await t.type('//textarea[@aria-label="Private note"]', "Keep the horn on the sound track only until the picture cuts in.");
    });
    await q(t.btn("Cancel", PANELS));
    await t.state("sp-panel-scene-note-editing", async () => {
      await t.press("Add note", PANELS);
      await t.type('//textarea[@aria-label="Scene note"]', "Night exterior. Sodium light from the bus stand, no practicals on the road.");
    });
    await q(t.btn("Cancel", PANELS));

    await q(t.btn("Scene Notes", WR_TABS));
    await t.state("sp-panel-characters", () => t.press("Characters", WR_TABS), { wait: 800 });
    await t.state("sp-panel-characters-match-menu", () => t.go('(//button[starts-with(@aria-label,"Correct match for ")])[1]'));
    await esc();

    await q(t.btn("Characters", WR_TABS));
    await t.state("sp-panel-comments-open", () => t.press("Comments", WR_TABS), { wait: 800 });
    await t.state("sp-panel-comments-actions-menu", () => t.go('(//button[@aria-label="Comment actions"])[1]'));
    await esc();
    await t.state("sp-panel-comments-resolved", () => t.go(radio("Resolved")));
    await q(radio("Open"));
    await t.state("sp-panel-comments-reply", async () => {
      await q(`(${PANELS}//button[normalize-space(.)="Reply"])[1]`);
      await t.type('//textarea[@aria-label="Reply"]', "Agreed, I will add the insert to the shot list.");
    });
    await q(t.btn("Cancel", PANELS));
    await t.state("sp-panel-comments-composer-scene", async () => {
      await t.go('//button[starts-with(normalize-space(.),"Comment on Scene")]');
      await t.type('//textarea[@aria-label="Comment"]', "Should the bus horn come before the first picture?");
    });
    await q(t.btn("Cancel", PANELS));
    await t.state("sp-panel-comments-composer-draft", async () => {
      await t.go(t.btn("Comment on draft", PANELS));
      await t.type('//textarea[@aria-label="Comment"]', "Overall the opening is stronger. Check the rooftop beat for length.");
    });
    await q(t.btn("Cancel", PANELS));

    await q(t.btn("Comments", WR_TABS));
    await t.state("sp-panel-draft-info", () => t.press("Draft information", WR_TABS), { wait: 800 });
    await t.state("sp-writing-room-two-panels", () => t.press("Scene Notes", WR_TABS), { wait: 800 });

    await q(VIEW);
    await t.menuItem("Focus Mode");
    await t.sleep(900);
    await t.state("sp-focus-mode", null, { wait: 800 });
    await q(t.btn("Exit Focus"));
    await t.sleep(600);
  });

  // ----------------------------------------------------------------------- drafts
  await phase("drafts", async () => {
    await freshLayout();
    await t.state("sp-draft-switch-menu", () => t.go(DRAFT_BTN));
    await t.state("sp-editor-older-draft", async () => {
      await t.menuItem(first.name);
      await b().waitUntil(async () => ((await (await b().$(DRAFT_BTN)).getAttribute("aria-label")) ?? "").startsWith(`Draft: ${first.name}.`), { timeout: 10_000 });
      await t.sleep(1500);
    }, { wait: 800 });
    await t.state("sp-panel-comments-empty", () => t.go(tb("Comments")));
    await t.state("sp-panel-comments-resolved-empty", () => t.go(radio("Resolved")));
    await q(`${PANELS}//button[@aria-label="Close panel"]`);
    await switchDraft(second.name);

    await t.state("sp-drafts-drawer", () => t.press("Drafts", TOOLBAR), { wait: 900 });
    await t.state("sp-draft-actions-menu", () => t.go(`//button[@aria-label=${xq(`Actions for ${second.name}`)}]`));
    await esc();
    await t.state("sp-draft-actions-menu-other", () => t.go(`//button[@aria-label=${xq(`Actions for ${first.name}`)}]`));
    await esc();

    await q(`//button[@aria-label=${xq(`Actions for ${second.name}`)}]`);
    await t.menuItem("Rename…");
    await t.state("sp-rename-draft-dialog", () => t.el("#rd-name"));
    await cancel();
    await q(`//button[@aria-label=${xq(`Actions for ${second.name}`)}]`);
    await t.menuItem("Delete…");
    await t.state("sp-delete-draft-dialog-current", () => t.waitText("This is the current draft. Make another draft current before deleting it."));
    await cancel();
    await q(`//button[@aria-label=${xq(`Actions for ${first.name}`)}]`);
    await t.menuItem("Delete…");
    await t.state("sp-delete-draft-dialog", () => t.el(t.btn("Delete Draft", DIALOG)));
    await cancel();

    await t.state("sp-new-draft-dialog", async () => {
      await t.press("New Draft", `${DRAWER}`);
      await t.el("#nd-name");
    });
    await t.state("sp-new-draft-previous-source", () => t.press("Previous named draft…", DIALOG));
    await t.state("sp-new-draft-duplicate-name", async () => {
      await t.type("#nd-name", first.name);
      await t.press("Create Draft", DIALOG);
      await t.waitText("Another draft already has this name");
    });
    await cancel();
  });

  // ---------------------------------------------------------------------- compare
  await phase("compare", async () => {
    await t.state("sp-compare-text-only", () => t.press("Compare", TOOLBAR), { wait: 1500 });
    await t.state("sp-compare-scene-only", () => t.press("Scene-only"), { wait: 800 });
    await t.state("sp-compare-second-change", async () => {
      await q(radio("Text-only"));
      await t.press("Next change");
    }, { wait: 800 });
    await t.state("sp-compare-same-draft", async () => {
      const before = await t.el('//select[@aria-label="Before"]');
      await before.selectByVisibleText(second.name);
      await t.waitText("Choose two different drafts to compare.");
    });
    await t.press("Back to script");
    await t.sleep(1200);
    t.unreachable("sp-compare-no-differences", "Needs two identical drafts; the demo drafts differ, and making a copy would leave an extra deleted draft in Recently Deleted.");
  });

  // ------------------------------------------- lock, revisions, unlock (always undone)
  await phase("lock", async () => {
    await freshLayout();
    await t.state("sp-lock-dialog", async () => {
      await t.press("Lock", TOOLBAR);
      await t.el(t.dialog("Lock this draft as the Shooting Draft?"));
      await t.sleep(900);
    }, { wait: 800 });
    await t.press("Lock as Shooting Draft", DIALOG);
    await t.state("sp-locked-draft", async () => {
      await t.waitText("Locked shooting draft.");
      await t.sleep(700);
    }, { wait: 800 });

    await t.state("sp-locked-edit-dialog", async () => {
      await q(ACTION);
      await b().keys("x");
      await t.el(t.dialog("This is your locked shooting draft. Create a revision?"));
    });
    await q(t.btn("Start Revision", DIALOG));
    await t.state("sp-start-revision-dialog", () => t.el("#rv-label"));
    await t.state("sp-start-revision-filled", async () => {
      await q(radio("Pink"));
      await t.type("#rv-reason", "Dog added to the opening, rooftop radio line.");
    });
    await t.state("sp-start-revision-custom-colour", () => t.press("Custom…", DIALOG));
    await cancel();
    await t.state("sp-revisions-drawer-locked", () => t.press("Revisions", TOOLBAR), { wait: 1200 });

    // Start a real revision on the way, edit it, show its screens, then delete it again.
    await q(t.btn("Start Revision", DRAWER));
    await q(radio("Pink"));
    await q(t.btn("Start Revision", DIALOG));
    await b().waitUntil(async () => ((await (await b().$(DRAFT_BTN)).getAttribute("aria-label")) ?? "").startsWith("Draft: Revision"), { timeout: 15_000 });
    await t.sleep(1800);
    await t.state("sp-revision-draft-editor", null, { wait: 800 });
    await q(ACTION);
    await b().execute((node) => {
      const w = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
      let last = null;
      for (let n = w.nextNode(); n; n = w.nextNode()) last = n;
      if (!last) return;
      const r = document.createRange();
      r.setStart(last, last.length);
      r.collapse(true);
      const sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(r);
    }, await t.el(ACTION));
    await t.sleep(300);
    await b().keys(" Again.".split(""));
    await t.sleep(600);
    await waitSaved();
    await q('//button[@aria-label="Navigator columns"]');
    await q(mi("Revision"));
    await t.sleep(1500);
    await t.state("sp-navigator-revision-column", null, { wait: 800 });
    await t.state("sp-revisions-drawer-revision", () => t.press("Revisions", TOOLBAR), { wait: 1500 });
    await q(closeX);
    await viewItem("Page view");
    await t.state("sp-page-view-revision-marks", async () => {
      await t.go('//label[.//span[normalize-space(.)="Show revision marks"]]');
      await t.sleep(1500);
    }, { wait: 800 });
    await q(closeX);
    await q(t.btn("Export", TOOLBAR));
    await t.state("sp-export-revision-marked", async () => {
      await t.go('//label[contains(@class,"ix-radio")][.//span[normalize-space(.)="Revision-marked version"]]');
    }, { wait: 800 });
    await cancel();

    // Back to the locked draft, then delete the revision and unlock.
    await switchDraft(second.name);
    await q(t.btn("Drafts", TOOLBAR));
    const nowRevision = (await t.op("screenplay.overview", {})).drafts.find((d) => d.name.startsWith("Revision"));
    if (nowRevision?.isCurrent) {
      await q(`//button[@aria-label=${xq(`Actions for ${second.name}`)}]`);
      await t.menuItem("Mark current");
      await t.sleep(600);
    }
    if (nowRevision) {
      await q(`//button[@aria-label=${xq(`Actions for ${nowRevision.name}`)}]`);
      await t.menuItem("Delete…");
      await q(t.btn("Delete Draft", DIALOG));
      await t.sleep(900);
    }
    await q(closeX);
    await q(t.btn("Revisions", TOOLBAR));
    await q(t.btn("Unlock draft", DRAWER));
    await t.sleep(900);
    await q(closeX);
  });
  // Whatever happened above, leave the Second Draft current and unlocked and no revision draft behind.
  try {
    let o = await t.op("screenplay.overview", {});
    for (const d of o.drafts) if (d.status === "Locked") await t.op("screenplay.unlock_draft", { draftId: d.id }).catch(() => undefined);
    o = await t.op("screenplay.overview", {});
    if (!o.drafts.find((d) => d.isCurrent && d.id === second.id)) await t.op("screenplay.set_current_draft", { draftId: second.id }).catch(() => undefined);
    for (const d of o.drafts) if (d.name.startsWith("Revision")) await t.op("screenplay.delete_draft", { draftId: d.id }).catch(() => undefined);
  } catch {
    // The run log shows what stopped; the checks below stay best effort.
  }

  // --------------------------------------------------- export, print preview, import
  await phase("export", async () => {
    await freshLayout();
    await t.state("sp-export-dialog", async () => {
      await t.press("Export", TOOLBAR);
      await t.el('//label[contains(@class,"ix-radio")]');
      await t.sleep(600);
    }, { wait: 800 });
    await t.state("sp-export-more-formats", () => t.press("Additional formats…", DIALOG));
    await t.state("sp-export-selected-none", () => t.go('//label[contains(@class,"ix-radio")][.//span[normalize-space(.)="Selected scenes"]]'), { wait: 800 });
    await t.state("sp-export-selected-scenes", async () => {
      await t.go('(//*[@role="group"][@aria-label="Scenes to export"]//label)[1]');
      await q('(//*[@role="group"][@aria-label="Scenes to export"]//label)[2]');
      await q('(//*[@role="group"][@aria-label="Scenes to export"]//label)[3]');
    });
    await t.state("sp-print-preview", async () => {
      await t.press("Preview…", DIALOG);
      await t.el(`${DIALOG}//*[@role="img"]`);
      await t.sleep(1200);
    }, { wait: 1200 });
    await t.state("sp-print-preview-zoomed", () => t.press("Zoom in", DIALOG), { wait: 900 });
    await t.state("sp-print-preview-no-title-page", async () => {
      await t.go('//label[.//span[normalize-space(.)="Title page"]]');
      await t.sleep(900);
    }, { wait: 900 });
    t.unreachable("sp-print-os-dialog", "Print opens the Windows print dialog, which is outside the app window and cannot be captured.");
    t.unreachable("sp-export-save-dialog", "Export… opens the native Save dialog (Export Screenplay), which is outside the web page.");
    t.unreachable("sp-export-complete", "The Export complete screen comes only after the native Save dialog is answered.");
    t.unreachable("sp-import-file-picker", "Browse… in Import Screenplay opens the native file dialog (Choose a screenplay to import).");
    await t.closeOverlays();

    await viewItem("Import screenplay");
    await t.state("sp-import-pasted-text", async () => {
      await t.type(
        "#ix-paste",
        "Shopping list for the weekend\n\nMilk, bread and eggs from the corner shop.\nRemember to call the plumber about the kitchen sink.\nThe meeting moved to Thursday afternoon at the usual place.",
      );
    });
    await t.state("sp-import-failed", async () => {
      await t.press("Preview", DIALOG);
      await t.waitText("Import failed. Your current project was not changed.");
    });
    await q(t.btn("Close", DIALOG));
    await t.sleep(400);
  });

  // ------------------------------------------------ screenplay picker and its dialogs
  await phase("picker", async () => {
    await t.state("sp-screenplay-picker-menu", () => t.go(PICKER_BTN));
    await t.menuItem("New Screenplay…");
    await t.state("sp-new-screenplay-dialog", () => t.el("#ns-title"));
    await t.state("sp-new-screenplay-duplicate-title", async () => {
      await t.type("#ns-title", spTitle);
      await t.waitText("Another screenplay here already has this title.");
    });
    await cancel();
    await q(PICKER_BTN);
    await t.menuItem("Rename Screenplay…");
    await t.state("sp-rename-screenplay-dialog", () => t.el("#rs-title"));
    await cancel();
    await q(PICKER_BTN);
    await t.menuItem("Delete Screenplay…");
    await t.state("sp-delete-screenplay-dialog", () => t.el(t.btn("Delete Screenplay", DIALOG)));
    await cancel();
  });

  // ------------------------------------------------------ final layout, empty screenplay
  await phase("finish", async () => {
    const o = await t.op("screenplay.overview", {});
    const cur = o.drafts.find((d) => d.isCurrent);
    if (cur?.id !== second.id) await t.op("screenplay.set_current_draft", { draftId: second.id }).catch(() => undefined);
    const sc = (await t.op("screenplay.document", { draftId: second.id })).scenes;
    if (sc[0]?.id !== scene1Id) await t.op("screenplay.move_scene", { sceneId: scene1Id, index: 0 }).catch(() => undefined);
    await freshLayout();
  });
  await phase("empty", async () => {
    await t.inEmptyProject(async () => {
      await t.workspace("Screenplay");
      await t.state("sp-empty-workspace", () => t.waitText("Write your screenplay or import one you already have."));
    });
  });
  await t.reset();
}
