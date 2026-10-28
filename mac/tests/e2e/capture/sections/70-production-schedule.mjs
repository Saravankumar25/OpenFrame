// WindowsHTML capture section: Production: Schedule (board, list, calendar, day detail, drawers, dialogs, menus, settings), Day Out of Days, Daily View, Sides, Reports, Budget; and the Call Sheets workspace.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.
//
// The section builds its own shooting day ("Shoot Day 4", 12 Nov 2026) and an off day, works on
// them, and removes them again, so the seeded days, call sheets and budget lines stay as they were.

export default async function (t) {
  const X = t.xq;
  const DLG = '//*[@role="dialog"]';
  const ANY_DRAWER = '//aside[contains(@class,"drawer")]';
  const drawer = (title) => `//aside[contains(@class,"drawer")][@aria-label=${X(title)}]`;
  const EXPORT = '(//main//button[starts-with(normalize-space(.),"Export")])[1]';
  const EXPORT_DIALOG = '//*[@role="dialog"][.//h2[starts-with(normalize-space(.),"Export")]]';
  const WARN_CHIP = '//button[contains(@class,"chip")][contains(normalize-space(.),"warning")]';
  const dayName = (n) => `//button[contains(@class,"of-dayname")][starts-with(normalize-space(.),"Shoot Day ${n}")]`;
  const strip = (n) => `//div[@role="button"][starts-with(@aria-label,"Scene ${n},")]`;
  const doodCell = (n) => `//table[@aria-label="Day Out of Days"]//tbody//button[contains(@aria-label,"Shoot Day ${n}")]`;
  const paper = '//*[@aria-label="Call sheet document"]';
  const STRICT = `${DLG}//label[contains(normalize-space(.),"Strict validation")]`;
  const SHOW_REPLACED = '//label[contains(normalize-space(.),"Show replaced revisions")]';
  const COVER = '//label[contains(normalize-space(.),"Include call-sheet cover page")]';
  const CAST_CHECK = '//label[contains(@class,"check")][contains(normalize-space(.),"(")][not(contains(normalize-space(.),":"))]';
  const SCENE_CHECK = '//label[contains(@class,"check")][contains(normalize-space(.),":")]';
  const FILTER_ROWS = '//input[@aria-label="Filter report rows"]';
  const SEARCH_SHEETS = '//input[@aria-label="Search call sheets"]';
  const warnDrawer = drawer("Schedule assistance");

  const seen = (text) => t.el(`//*[normalize-space(text())=${X(text)}]`);
  const has = async (xp) => (await t.$(xp)).isExisting();
  const swallow = (p) => p.catch(() => undefined);
  const inDlg = (name) => t.press(name, DLG);
  const dialogOpen = (title) => t.el(t.dialog(title));
  const toastsAway = () => t.browser.execute(() => document.querySelectorAll('.toast button[aria-label="Dismiss"]').forEach((b) => b.click()));
  const seg = (group, label) => t.go(`//*[@role="radiogroup"][@aria-label=${X(group)}]//*[@role="radio"][normalize-space(.)=${X(label)}]`);
  const pickIn = async (selectXp, text) => {
    const sel = await t.el(selectXp);
    await (await sel.$(`.//option[starts-with(normalize-space(.),${X(text)})]`)).click();
    await t.sleep(350);
  };
  const pick = (aria, text) => pickIn(`//select[@aria-label=${X(aria)}]`, text);
  const pickNth = async (aria, n) => {
    const sel = await t.el(`//select[@aria-label=${X(aria)}]`);
    await (await sel.$$(".//option"))[n].click();
    await t.sleep(350);
  };
  const scrollTo = async (xp, block = "center") => {
    const el = await t.el(xp);
    await t.browser.execute((e, b) => e.scrollIntoView({ block: b }), el, block);
    await t.sleep(300);
  };
  const scrollPaper = async (top) => {
    await t.browser.execute((toTop) => {
      const w = document.querySelector(".of-paper-wrap");
      if (w) w.scrollTop = toTop ? 0 : w.scrollHeight;
    }, top);
    await t.sleep(300);
  };
  const showAllWarnings = async () => {
    for (let i = 0; i < 12 && (await has(`${warnDrawer}//button[normalize-space(.)="Show again"]`)); i++) await t.press("Show again", warnDrawer);
  };
  /** A state that only exists when `cond` holds; otherwise it is listed as unreachable. */
  const maybe = async (name, why, cond, fn, opts) => {
    if (await cond()) return t.state(name, fn, opts);
    t.unreachable(name, why);
    return null;
  };
  /** Steps that depend on each other; if one throws, the group is recorded as one failed state and the section goes on. */
  const flow = async (name, fn) => {
    try {
      await fn();
    } catch (e) {
      await t.state(`${name}-flow-stopped`, async () => {
        throw e;
      });
    }
    await t.closeOverlays().catch(() => undefined);
  };

  const sched = () => t.op("schedule.get", {});
  const idOf = (x) => x?.id ?? x;
  const scheduleTab = async () => {
    await t.productionTab("Schedule");
    await t.el('//button[normalize-space(.)="Create Shooting Day"]');
  };
  const openStrip = async (n) => {
    await t.go(strip(n));
    await t.el(drawer(`Scene ${n}`));
  };
  const closeDrawer = async () => {
    await swallow(t.press("Close", ANY_DRAWER));
    await t.sleep(250);
  };
  const dayMenu = async (n, item) => {
    await t.press(`Shoot Day ${n} actions`);
    await t.menuItem(item);
  };

  const base = await sched();
  const scheduleId = base.schedule.id;
  const pool = base.unscheduled.slice(0, 8).map((s) => ({ id: s.id, n: s.number, sceneId: s.sceneId }));
  const castElements = [];
  let day4 = null;

  try {
    // ------------------------------------------------------------------ Schedule: views and filters
    await flow("prod-schedule-views", async () => {
      await t.state("prod-schedule-board", scheduleTab);
      await t.state("prod-schedule-list", () => seg("Schedule view", "List"));
      await t.state("prod-schedule-calendar", () => seg("Schedule view", "Calendar"));
      await t.state("prod-schedule-calendar-next-month", () => t.press("Next month"));
      await t.press("Previous month");
      await seg("Schedule view", "Board");
    });

    await flow("prod-schedule-filters", async () => {
      await t.state("prod-schedule-filter-board", () => pick("Filter by day or night", "Night scenes"));
      await t.state("prod-schedule-filter-list", () => seg("Schedule view", "List"));
      await t.state("prod-schedule-filter-no-match", async () => {
        await pick("Filter by scheduled state", "Unscheduled");
        await pick("Filter by shooting day", "Shoot Day 1");
        await t.waitText("No matching scenes.");
      });
      await t.press("Clear filters");
      await seg("Schedule view", "Board");
    });

    // ------------------------------------------------------------------ Schedule: days
    await flow("prod-schedule-new-day", async () => {
      await t.state("prod-schedule-create-day-dialog", async () => {
        await t.press("Create Shooting Day");
        await dialogOpen("Create Shooting Day");
      });
      await t.state("prod-schedule-create-day-filled", async () => {
        await t.setNative("#of-day-date", "2026-11-12");
        await t.type("#of-day-notes", "Tower climb and rooftop, night shoot");
      });
      await t.state(
        "prod-schedule-day-created",
        async () => {
          await inDlg("Create Day");
          await t.el(dayName(4));
          await t.waitText("Created a shooting day");
        },
        { keepToasts: true },
      );
      day4 = (await sched()).days.find((d) => d.label === "Shoot Day 4");
      await t.state("prod-schedule-day-detail-empty", async () => {
        await t.go(dayName(4));
        await t.waitText("No scenes scheduled.");
      });
      await t.press("Back to schedule");
    });

    await flow("prod-schedule-off-day", async () => {
      await t.state("prod-schedule-off-day-dialog", async () => {
        await t.press("Off Day");
        await dialogOpen("Add Off Day");
      });
      await t.state("prod-schedule-off-day-filled", async () => {
        await t.setNative("#of-day-date", "2026-11-13");
        await t.type("#of-day-notes", "Travel and rest day");
      });
      await t.state(
        "prod-schedule-off-day-created",
        async () => {
          await inDlg("Add Off Day");
          await t.waitText("Added an off day");
        },
        { keepToasts: true },
      );
    });

    await flow("prod-schedule-day-menus", async () => {
      await t.state("prod-schedule-day-menu", () => t.press("Shoot Day 1 actions"));
      await t.state("prod-schedule-edit-date-dialog", async () => {
        await t.menuItem("Edit date");
        await dialogOpen("Edit date of Shoot Day 1");
      });
      await inDlg("Cancel");
      await t.state("prod-schedule-day-menu-empty-day", () => t.press("Shoot Day 4 actions"));
      await t.state("prod-schedule-edit-date-dialog-no-call-sheet", async () => {
        await t.menuItem("Edit date");
        await dialogOpen("Edit date of Shoot Day 4");
      });
      await inDlg("Cancel");
    });

    await flow("prod-schedule-strip-menus", async () => {
      await t.state("prod-schedule-strip-menu-scheduled", () => t.press("Scene 1 actions"));
      await t.keys("Escape");
      await t.state("prod-schedule-strip-menu-unscheduled", () => t.press(`Scene ${pool[0].n} actions`));
      await t.state(
        "prod-schedule-strip-moved",
        async () => {
          await t.closeOverlays();
          await t.press(`Scene ${pool[0].n} actions`);
          await t.menuItem("Shoot Day 4");
          await t.waitText("Moved Scene");
        },
        { keepToasts: true },
      );
      // Three more scenes, estimates and times on the new day, set up directly.
      for (const s of pool.slice(1, 4)) await t.op("schedule.move_strip", { stripId: s.id, dayId: day4.id });
      const minutes = [90, 120, 60, 75];
      for (const [i, s] of pool.slice(0, 4).entries()) await t.op("schedule.set_strip_estimate", { stripId: s.id, minutes: minutes[i] });
      await t.op("schedule.set_day_times", { dayId: day4.id, crewCall: 7 * 60, shootingStart: 8 * 60, targetWrap: 19 * 60 });
      // Confirmed cast on these scenes, so the Day Out of Days, sides and call sheet have people in them.
      for (const s of pool.slice(0, 4)) {
        for (const name of ["LAKSHMI", "VENKAT RAO"]) {
          const el = await t.op("breakdown.add_element", { sceneId: s.sceneId, category: "Cast", name, notes: null, span: null, catalog: { mode: "auto", catalogItemId: null } });
          castElements.push(idOf(el));
        }
      }
      await toastsAway();
      await t.sleep(800);
    });

    // ------------------------------------------------------------------ Schedule: breaks (markers)
    await flow("prod-schedule-markers", async () => {
      await t.state("prod-schedule-marker-dialog", async () => {
        await dayMenu(4, "Meal Break");
        await dialogOpen("Add to this day");
      });
      await t.state("prod-schedule-marker-error-duration", async () => {
        await t.type("#of-mk-dur", "abc");
        await inDlg("Add");
        await t.waitText("Enter a duration such as 45m or 1h 30m.");
      });
      await inDlg("Cancel");
      await t.state("prod-schedule-marker-error-note", async () => {
        await dayMenu(4, "Custom Note");
        await dialogOpen("Add to this day");
        await inDlg("Add");
        await t.waitText("Write the note for this marker.");
      });
      await t.type("#of-mk-label", "Golden hour from 17:40, be ready for the rooftop");
      await inDlg("Add");
      await t.sleep(500);
      await t.state("prod-schedule-marker-dialog-filled", async () => {
        await dayMenu(4, "Meal Break");
        await dialogOpen("Add to this day");
        await t.type("#of-mk-label", "Lunch");
        await t.setNative("#of-mk-time", "13:00");
        await t.type("#of-mk-dur", "45m");
        await t.type("#of-mk-notes", "Packed meals at the bus stand");
      });
      await inDlg("Add");
      await t.state("prod-schedule-marker-added", () => t.el('//div[@role="button"][contains(@aria-label,"LUNCH")]'));
      await t.state("prod-schedule-marker-edit-dialog", async () => {
        await t.go('//div[@role="button"][contains(@aria-label,"LUNCH")]');
        await dialogOpen("Edit break");
      });
      await inDlg("Cancel");
    });

    // ------------------------------------------------------------------ Schedule: strip drawer
    await flow("prod-schedule-strip-drawers", async () => {
      await t.state("prod-schedule-strip-drawer", () => openStrip(pool[0].n));
      await closeDrawer();
      await t.state("prod-schedule-strip-drawer-error-estimate", async () => {
        await openStrip(pool[0].n);
        await t.type("#of-strip-est", "abc");
        await t.keys("Enter");
        await t.waitText("Enter the estimate as minutes");
      });
      await closeDrawer();
      await t.state("prod-schedule-strip-drawer-error-start", async () => {
        await openStrip(pool[0].n);
        await t.type("#of-strip-start", "late");
        await t.keys("Enter");
        await t.waitText("Enter the start time like 14:30");
      });
      await closeDrawer();
      await t.state("prod-schedule-strip-drawer-error-pages", async () => {
        await openStrip(pool[0].n);
        await t.type("#of-strip-pages", "abc");
        await t.keys("Enter");
        await t.waitText("Enter pages like 1 2/8 or 3/8.");
      });
      await closeDrawer();
      await t.state("prod-schedule-strip-drawer-unscheduled", () => openStrip(pool[4].n));
      await closeDrawer();
      await t.state("prod-schedule-strip-drawer-scheduled-day-1", () => openStrip(1));
      await closeDrawer();
    });

    // ------------------------------------------------------------------ Schedule: warnings and suggestions
    await flow("prod-schedule-warnings", async () => {
      const warnings = (await sched()).warnings.filter((w) => !w.decision);
      await t.state("prod-schedule-warnings-drawer", async () => {
        await t.go(WARN_CHIP);
        await t.el(warnDrawer);
      });
      await maybe(
        "prod-schedule-warnings-kept",
        "the schedule has no open warning to keep",
        async () => warnings.length > 0,
        async () => {
          await t.press("Keep anyway", warnDrawer);
          await t.waitText("Kept anyway");
        },
      );
      await maybe(
        "prod-schedule-warnings-dismissed",
        "the schedule has fewer than two warnings",
        async () => warnings.length > 1,
        async () => {
          await t.press("Dismiss", warnDrawer);
          await t.waitText("Dismissed");
        },
      );
      await showAllWarnings();
      await maybe(
        "prod-schedule-warning-open-day",
        "no warning names a day",
        async () => warnings.some((w) => w.dayId),
        async () => {
          await t.go(`(${warnDrawer}//button[normalize-space(.)="Open affected item"])[1]`);
          await t.el('//button[normalize-space(.)="Back to schedule"]');
        },
      );
      await swallow(t.press("Back to schedule"));
    });

    await flow("prod-schedule-suggestions", async () => {
      await scheduleTab();
      await t.state("prod-schedule-suggestions-drawer", async () => {
        await t.press("Suggest Grouping");
        await t.el(drawer("Grouping suggestions"));
        await t.sleep(800);
      });
      await maybe(
        "prod-schedule-suggestions-apply-dialog",
        "the grouping advice finds nothing to suggest for this schedule",
        () => has('//button[normalize-space(.)="Apply manually"]'),
        async () => {
          await t.press("Apply manually");
          await dialogOpen("Group these scenes?");
        },
      );
      await swallow(inDlg("Cancel"));
      await closeDrawer();
    });

    // ------------------------------------------------------------------ Schedule: settings, finalized, strict
    await flow("prod-schedule-settings", async () => {
      await t.state("prod-schedule-settings-dialog", async () => {
        await t.press("Schedule settings");
        await dialogOpen("Schedule settings");
      });
      await t.state("prod-schedule-settings-error", async () => {
        await t.type("#of-sch-len", "abc");
        await inDlg("Save");
        await t.waitText("Enter the target day length");
      });
      await inDlg("Cancel");
      await t.state("prod-schedule-settings-finalized-selected", async () => {
        await t.press("Schedule settings");
        await dialogOpen("Schedule settings");
        await pick("Schedule status", "Finalized");
      });
      await inDlg("Save");
      await t.state("prod-schedule-finalized-banner", () => t.waitText("This schedule is finalized"));
      await t.state("prod-schedule-settings-reopen", async () => {
        await t.press("Reopen…");
        await dialogOpen("Schedule settings");
      });
      await pick("Schedule status", "Draft");
      await inDlg("Save");
      await t.waitText("This schedule is finalized", { present: false });
    });

    // Strict validation: a chip, and "Keep Anyway" when a move creates a warning on a fresh day.
    await flow("prod-schedule-strict", async () => {
      await t.press("Schedule settings");
      await dialogOpen("Schedule settings");
      await t.go(STRICT);
      await t.state("prod-schedule-settings-strict", () => t.waitText("Strict validation"));
      await inDlg("Save");
      await t.state("prod-schedule-strict-chip", () => t.el('//*[contains(@class,"chip")][normalize-space(.)="Strict validation"]'));
      const tmpId = idOf(await t.op("schedule.create_day", { scheduleId, date: "2026-11-14", notes: null, offDay: false, afterDayId: null }));
      try {
        const tmp = (await sched()).days.find((d) => d.id === tmpId);
        await scheduleTab();
        await t.state("prod-schedule-keep-anyway-dialog", async () => {
          await t.press(`Scene ${pool[5].n} actions`);
          await t.menuItem(tmp.label);
          await dialogOpen("This change creates a schedule warning");
        });
        await swallow(inDlg("Change the schedule instead"));
      } finally {
        await swallow(t.op("schedule.delete_day", { dayId: tmpId }));
      }
      await t.press("Schedule settings");
      await dialogOpen("Schedule settings");
      await t.go(STRICT);
      await inDlg("Save");
      await t.waitText("Strict validation", { present: false });
    });

    t.unreachable(
      "prod-schedule-script-changes",
      "the banner and its dialog appear only after the Production Source script is edited; editing it would change the shared Night Bus script for every other section",
    );
    t.unreachable("prod-schedule-with-source-no-schedule", "needs a project that has a Production Source but no shooting schedule; both demo projects have one or none");

    // ------------------------------------------------------------------ Schedule: day detail
    await flow("prod-schedule-day-detail", async () => {
      await scheduleTab();
      await t.state("prod-schedule-day-detail", async () => {
        await t.go(dayName(1));
        await t.waitText("What exactly are we doing on this day?");
      });
      await t.state("prod-schedule-day-detail-add-break-menu", () => t.press("Add break"));
      await t.keys("Escape");
      await t.state("prod-schedule-day-detail-actions-menu", () => t.press("More day actions"));
      await t.keys("Escape");
      await t.state("prod-schedule-day-export-dialog", async () => {
        await t.go(EXPORT);
        await t.el(t.dialog("Export Shooting Schedule"));
      });
      await t.keys("Escape");
      await t.press("Back to schedule");

      await t.state("prod-schedule-day-detail-timeline", async () => {
        await t.go(dayName(4));
        await t.el('//ol[@aria-label="Timeline of Shoot Day 4"]');
      });
      await t.state("prod-schedule-day-detail-over-target", async () => {
        await t.type('//input[@aria-label="Day target length"]', "3h");
        await t.keys("Tab");
        await t.waitText("Over target");
      });
      await t.type('//input[@aria-label="Day target length"]', "");
      await t.keys("Tab");
      await t.state(
        "prod-schedule-day-detail-time-error",
        async () => {
          await t.type('//input[@aria-label="Crew call for Shoot Day 4"]', "soon");
          await t.keys("Tab");
          await t.waitText("Enter the crew call as a time");
        },
        { keepToasts: true },
      );
      await t.type('//input[@aria-label="Crew call for Shoot Day 4"]', "07:00");
      await t.keys("Tab");
      await toastsAway();
      await t.press("Back to schedule");

      await t.state("prod-schedule-day-detail-off", async () => {
        await t.go('//button[contains(@class,"of-dayname")][starts-with(normalize-space(.),"Off")]');
        await t.waitText("Off day: no scenes, notes allowed.");
      });
      await t.press("Back to schedule");

      await t.state("prod-schedule-export-dialog", async () => {
        await t.go(EXPORT);
        await t.el(t.dialog("Export Shooting Schedule"));
      });
      await t.keys("Escape");
    });

    // ------------------------------------------------------------------ Day Out of Days
    await flow("prod-dood", async () => {
      await t.state("prod-dood-table", async () => {
        await t.productionTab("Day Out of Days");
        await t.el('//table[@aria-label="Day Out of Days"]');
      });
      const label = await (await t.el(doodCell(4))).getAttribute("aria-label");
      const code = label.includes(": T") ? "H " : "T ";
      await t.state("prod-dood-cell-menu", () => t.go(doodCell(4)));
      await t.state(
        "prod-dood-corrected",
        async () => {
          await t.menuItem(code);
          await t.waitText("1 corrected by hand");
        },
        { keepToasts: true },
      );
      await t.state("prod-dood-note-dialog", async () => {
        await t.go(doodCell(4));
        await t.menuItem("Add a note");
        await dialogOpen("Correction note");
        await t.type("#of-dood-note", "Flies in the night before");
      });
      await inDlg("Save");
      await t.sleep(400);
      await t.go(doodCell(4));
      await t.menuItem("Use the schedule value");
      await t.browser.waitUntil(async () => !(await has('//*[contains(@class,"chip")][contains(normalize-space(.),"corrected by hand")]')), { timeout: 10_000, timeoutMsg: "correction still shown" });
      await t.state("prod-dood-simple-codes", () => pick("Code style", "Codes: W and -"));
      await pick("Code style", "Codes: SW");
      await t.state("prod-dood-one-cast", () => pickNth("Cast to show", 1));
      await pick("Cast to show", "All cast");
      await t.state("prod-dood-day-range", async () => {
        await pick("First day", "Shoot Day 2");
        await pick("Last day", "Shoot Day 3");
      });
      await pick("First day", "From the first day");
      await pick("Last day", "To the last day");
      await t.state("prod-dood-export-dialog", async () => {
        await t.go(EXPORT);
        await t.el(t.dialog("Export Day Out of Days"));
      });
      await t.keys("Escape");
    });

    // ------------------------------------------------------------------ Daily View
    await flow("prod-daily", async () => {
      await t.state("prod-daily-day-1", async () => {
        await t.productionTab("Daily View");
        await pick("Shooting day", "Shoot Day 1");
        await t.waitText("Open Call Sheet");
      });
      await t.state("prod-daily-no-call-sheet", async () => {
        await pick("Shooting day", "Shoot Day 3");
        await t.waitText("No call sheet yet for this day.");
      });
      await t.state("prod-daily-day-4", async () => {
        await pick("Shooting day", "Shoot Day 4");
        await t.waitText("Tower climb and rooftop");
      });
      await t.state("prod-daily-off-day", async () => {
        await pickNth("Shooting day", 4);
        await t.waitText("Off days have no call sheet.");
      });
      await pick("Shooting day", "Shoot Day 1");
      await t.state("prod-daily-export-dialog", async () => {
        await t.go(EXPORT);
        await t.el(t.dialog("Export Daily View"));
      });
      await t.keys("Escape");
    });

    // ------------------------------------------------------------------ Sides
    await flow("prod-sides", async () => {
      await t.state("prod-sides-setup", async () => {
        await t.productionTab("Sides");
        await t.el('//*[@aria-label="Sides preview"]');
      });
      await t.state("prod-sides-day-4", () => pick("Shooting day", "Shoot Day 4"));
      await t.state("prod-sides-selected-cast-none", async () => {
        await seg("Sides for", "Selected cast");
        await t.waitText("Choose a shooting day and at least one scene and cast member");
      });
      await maybe(
        "prod-sides-selected-cast",
        "no confirmed cast on the first shooting day",
        () => has(CAST_CHECK),
        async () => {
          await t.go(`(${CAST_CHECK})[1]`);
          await t.el('//*[@aria-label="Sides preview"]');
        },
      );
      await seg("Sides for", "All cast");
      await t.state("prod-sides-cover-page", async () => {
        await t.go(COVER);
        await seen("Call sheet cover");
      });
      await t.go(COVER);
      await t.state("prod-sides-scene-unchecked", async () => {
        await t.go(`(${SCENE_CHECK})[1]`);
        await t.sleep(400);
      });
      await t.go(`(${SCENE_CHECK})[1]`);
      const emptyDayId = idOf(await t.op("schedule.create_day", { scheduleId, date: "2026-11-15", notes: null, offDay: false, afterDayId: null }));
      try {
        await t.productionTab("Schedule");
        await t.productionTab("Sides");
        await t.state("prod-sides-no-scenes", async () => {
          const e = (await sched()).days.find((d) => d.id === emptyDayId);
          await pick("Shooting day", e.label);
          await t.waitText("No scenes scheduled on this day.");
        });
      } finally {
        await swallow(t.op("schedule.delete_day", { dayId: emptyDayId }));
      }
      await t.productionTab("Schedule");
      await t.productionTab("Sides");
      await t.state(
        "prod-sides-saved",
        async () => {
          await t.press("Save Sides");
          await t.waitText("Saved snapshot");
        },
        { keepToasts: true },
      );
      await t.state("prod-sides-live-with-saved", async () => {
        await t.press("Back to live preview");
        await seen("Saved sides");
      });
      await t.state("prod-sides-export-dialog", async () => {
        await t.go(EXPORT);
        await t.el(EXPORT_DIALOG);
      });
      await t.keys("Escape");
    });

    // ------------------------------------------------------------------ Reports
    await flow("prod-reports", async () => {
      await t.productionTab("Reports");
      for (const [key, label] of [
        ["scene", "Scene Report"],
        ["location", "Location Report"],
        ["cast-scene", "Cast Scene Report"],
        ["prop", "Prop Report"],
        ["schedule", "Schedule Report"],
        ["breakdown-completeness", "Breakdown Completeness"],
      ]) {
        await t.state(`prod-reports-${key}`, async () => {
          await t.press(label);
          await t.sleep(700);
        });
      }
      await t.press("Location Report");
      await t.state("prod-reports-filtered", async () => {
        await t.type(FILTER_ROWS, "bus");
        await t.sleep(700);
      });
      await t.state("prod-reports-filter-no-match", async () => {
        await t.type(FILTER_ROWS, "zzzz");
        await t.sleep(700);
      });
      await t.type(FILTER_ROWS, "");
      await t.press("Scene Report");
      await t.state(
        "prod-reports-saved-snapshot",
        async () => {
          await t.press("Save snapshot");
          await t.waitText("Back to live reports");
        },
        { keepToasts: true },
      );
      await t.state("prod-reports-live-with-snapshots", async () => {
        await t.press("Back to live reports");
        await seen("Saved snapshots");
      });
      await t.state("prod-reports-export-dialog", async () => {
        await t.go(EXPORT);
        await t.el(EXPORT_DIALOG);
      });
      await t.keys("Escape");
    });

    // ------------------------------------------------------------------ Budget
    await flow("prod-budget", async () => {
      const categories = ((await t.op("budget.get", {}))?.categories ?? []).filter((c) => c !== "Contingency");
      await t.state("prod-budget-filled", async () => {
        await t.productionTab("Budget");
        await t.el('//table[@aria-label="Budget lines"]');
      });
      await maybe(
        "prod-budget-review-banner",
        "the budget showed no review reminder after the schedule changes in this section",
        () => has('//*[contains(@class,"banner")][contains(normalize-space(.),"Review budget.")]'),
        () => t.sleep(200),
      );
      await t.state("prod-budget-add-line-dialog", async () => {
        await t.press("Add Line");
        await dialogOpen("Add Line");
      });
      await t.state("prod-budget-add-line-error-description", async () => {
        await inDlg("Add Line");
        await t.waitText("A line needs a description.");
      });
      await t.state("prod-budget-add-line-error-planned", async () => {
        await t.type("#of-bl-desc", "Colour grade and sound mix");
        await t.type("#of-bl-amt", "lots");
        await inDlg("Add Line");
        await t.waitText("Enter the planned amount as a non-negative number");
      });
      await t.state("prod-budget-add-line-filled", async () => {
        await pickIn("//select[@id='of-bl-cat']", categories.at(-1) ?? "Post/Other");
        await t.type("#of-bl-amt", "180000");
        await t.type("#of-bl-notes", "Two weeks at an outside studio");
      });
      await inDlg("Add Line");
      await t.state("prod-budget-line-added", () => t.waitText("Colour grade and sound mix"), { keepToasts: true });
      await toastsAway();
      await t.press("Delete Colour grade and sound mix");
      await t.waitText("Colour grade and sound mix", { present: false, sel: "main" });
      await toastsAway();
      await t.state("prod-budget-edit-line-dialog", async () => {
        await t.press("Edit Lead and supporting cast fees");
        await dialogOpen("Edit line");
      });
      await inDlg("Cancel");
      await t.state("prod-budget-edit-plan-dialog", async () => {
        await t.press("Edit plan");
        await dialogOpen("Budget plan");
      });
      await t.state("prod-budget-edit-plan-error", async () => {
        await t.type('//input[@aria-label="Contingency value"]', "150");
        await inDlg("Save");
        await t.waitText("Contingency must be between 0% and 100%.");
      });
      await inDlg("Cancel");
      await t.state("prod-budget-snapshot-name-dialog", async () => {
        await t.press("Save snapshot");
        await dialogOpen("Save budget snapshot");
        await t.type("#of-b-snap", "Pre-production estimate");
      });
      await inDlg("Save snapshot");
      await t.state("prod-budget-snapshots-saved", () => seen("Saved snapshots"));
      await t.state("prod-budget-snapshot-view", async () => {
        await t.press("View");
        await t.el(`//*[@role="dialog"][.//h2[normalize-space(.)="Pre-production estimate"]]`);
      });
      await t.keys("Escape");
      await t.state("prod-budget-export-dialog", async () => {
        await t.go(EXPORT);
        await t.el(EXPORT_DIALOG);
      });
      await t.keys("Escape");
      await swallow(t.press("Delete snapshot Pre-production estimate"));
    });

    // ------------------------------------------------------------------ Call Sheets
    await flow("cs-list", async () => {
      await t.state("cs-list", () => t.workspace("Call Sheets"), { home: "Call Sheets" });
      await t.state("cs-row-context-menu", () => t.rightClick("(//tbody//tr)[1]"));
      await t.keys("Escape");
      await t.state("cs-row-actions-menu", () => t.go('(//tbody//button[contains(@aria-label," actions")])[1]'));
      await t.keys("Escape");
      await t.state("cs-search", async () => {
        await t.type(SEARCH_SHEETS, "Day 2");
        await t.sleep(300);
      });
      await t.state("cs-search-no-match", async () => {
        await t.type(SEARCH_SHEETS, "zzzz");
        await t.waitText("No matching call sheets.");
      });
      await t.type(SEARCH_SHEETS, "");
      await t.state("cs-filter-draft", () => seg("Filter call sheets", "Draft"));
      await t.state("cs-filter-final-none", async () => {
        await seg("Filter call sheets", "Final");
        await t.waitText("No matching call sheets.");
      });
      await seg("Filter call sheets", "All");
    });

    await flow("cs-create", async () => {
      await t.state("cs-create-dialog", async () => {
        await t.press("Create Call Sheet");
        await dialogOpen("Create Call Sheet");
      });
      await t.state("cs-create-dialog-day-chosen", () => pick("Source shooting day", "Shoot Day 4"));
      // Day 4 starts with no day times, so the sheet shows Needs input for the crew call.
      await t.op("schedule.set_day_times", { dayId: day4.id, crewCall: null, shootingStart: null, targetWrap: null });
      await inDlg("Create Call Sheet");
      await t.el(paper);
      await t.sleep(600);
      await t.state("cs-editor-draft", () => scrollPaper(true));
      await t.state("cs-editor-draft-lower", () => scrollPaper(false));
      await scrollPaper(true);
    });

    // Add Section: the menu, then every section in turn.
    await flow("cs-sections", async () => {
      await t.state("cs-add-section-menu", () => t.press("Add Section"));
      await t.keys("Escape");
      for (const [key, item] of [
        ["weather", "Weather"],
        ["extras", "Extras"],
        ["departments", "Department notes"],
        ["advance", "Advance schedule"],
        ["crew", "Crew"],
        ["communication", "Walkie channels and contacts"],
        ["special-notes", "Special notes"],
        ["attachments", "Attachments"],
        ["reference-images", "Reference images"],
      ]) {
        await t.state(`cs-section-${key}`, async () => {
          await t.closeOverlays();
          // Some sections come with the sheet when the project has the data (crew, for one); only add the others.
          if (!(await has(`//section[@aria-label=${X(item)}]`))) {
            await t.press("Add Section");
            await t.menuItem(item);
          }
          await scrollTo(`//section[@aria-label=${X(item)}]`, "start");
          if (key === "weather") await t.type('//input[@aria-label="Forecast"]', "Clear after dusk, light wind from the east");
        });
      }
      await t.state("cs-section-custom-field", async () => {
        await t.press("Add Section");
        await t.menuItem("Custom field");
        await scrollPaper(false);
      });
      await scrollPaper(true);
    });

    // Finalize needs a crew call: the dialog, then the refusal.
    await flow("cs-finalize", async () => {
      await t.state("cs-finalize-dialog-missing-crew-call", async () => {
        await t.press("Finalize…");
        await dialogOpen("Finalize call sheet");
      });
      await t.state(
        "cs-finalize-crew-call-required",
        async () => {
          await inDlg("Finalize");
          await t.waitText("crew call");
          await t.sleep(300);
        },
        { keepToasts: true, blur: false },
      );
      await toastsAway();
      await t.type('//input[@aria-label="Crew call"]', "07:00");
      await t.keys("Tab");
      await t.sleep(900);
    });

    // A schedule change after the sheet was made: Needs refresh, the refresh dialog, finalize while stale.
    await flow("cs-refresh", async () => {
      await t.op("schedule.move_strip", { stripId: pool[4].id, dayId: day4.id });
      await t.state("cs-needs-refresh-banner", async () => {
        await t.waitText("Needs refresh.");
        await scrollPaper(true);
      });
      await t.state("cs-finalize-stale-dialog", async () => {
        await t.press("Finalize…");
        await dialogOpen("Finalize call sheet");
        await inDlg("Finalize");
        await t.waitText("Finalize this version anyway");
      });
      await inDlg("Cancel");
      await t.state("cs-refresh-dialog", async () => {
        await t.press("Review Update");
        await dialogOpen("Update call sheet: review changes");
      });
      await t.state(
        "cs-refreshed",
        async () => {
          await inDlg("Apply Update");
          await t.waitText("Needs refresh.", { present: false });
        },
        { keepToasts: true },
      );
      await toastsAway();
      await t.state("cs-refresh-dialog-no-changes", async () => {
        await t.press("Refresh from schedule");
        await dialogOpen("Update call sheet: review changes");
        await t.waitText("This call sheet already matches the schedule.");
      });
      await inDlg("Cancel");
      await t.state("cs-editor-export-dialog", async () => {
        await t.press("Export PDF");
        await t.el(t.dialog("Export Call Sheet"));
      });
      await t.keys("Escape");
    });

    await flow("cs-final", async () => {
      await t.state("cs-finalize-dialog", async () => {
        await t.press("Finalize…");
        await dialogOpen("Finalize call sheet");
      });
      await t.state(
        "cs-final",
        async () => {
          await inDlg("Finalize");
          await t.waitText("Finalized ");
          await scrollPaper(true);
        },
        { keepToasts: true },
      );
      await toastsAway();
    });

    // The schedule moves on after the sheet is final: a new revision.
    await flow("cs-revisions", async () => {
      await t.op("schedule.set_day_times", { dayId: day4.id, crewCall: 7 * 60 + 30, shootingStart: 8 * 60, targetWrap: 19 * 60 });
      await t.state("cs-final-schedule-changed", () => t.waitText("The schedule changed after this call sheet was finalized"));
      await t.state("cs-new-revision-dialog", async () => {
        await t.press("Start New Revision");
        await dialogOpen("Update call sheet: review changes");
      });
      await t.state(
        "cs-revision-v2",
        async () => {
          await inDlg("Create New Revision");
          await seen("Revisions");
        },
        { keepToasts: true },
      );
      await toastsAway();
      await t.state("cs-replaced-by-newer-revision", async () => {
        await t.go('//button[contains(@class,"li")][starts-with(normalize-space(.),"v1")]');
        await t.waitText("A newer revision replaced this one.");
      });
      await t.press("Open newer revision");
      await seen("Revisions");
    });

    // The list again, with revisions, and filters that now have rows.
    await flow("cs-list-revisions", async () => {
      await t.op("schedule.move_strip", { stripId: pool[5].id, dayId: day4.id });
      await t.state("cs-list-with-revisions", async () => {
        await t.press("Call Sheets", "//main");
        await t.el(SEARCH_SHEETS);
      });
      await t.state("cs-list-show-replaced", async () => {
        await t.go(SHOW_REPLACED);
        await t.waitText("replaced by a newer revision");
      });
      await t.state("cs-filter-needs-refresh", () => seg("Filter call sheets", "Needs refresh"));
      await t.state("cs-filter-final", () => seg("Filter call sheets", "Final"));
      await seg("Filter call sheets", "All");
      await t.go(SHOW_REPLACED);
      await t.state("cs-list-export-dialog", async () => {
        await t.go('(//tbody//button[normalize-space(.)="Export"])[1]');
        await t.el(t.dialog("Export Call Sheet"));
      });
      await t.keys("Escape");
    });

    // Every day with a sheet: the create dialog has nothing to offer.
    await flow("cs-every-day", async () => {
      const without = (await sched()).days.filter((d) => !d.isOffDay && !d.callSheet);
      const made = [];
      try {
        for (const d of without) made.push(idOf(await t.op("callsheets.create", { dayId: d.id })));
        await t.state("cs-create-dialog-every-day-done", async () => {
          await t.workspace("Call Sheets");
          await t.press("Create Call Sheet");
          await dialogOpen("Create Call Sheet");
          await t.waitText("Every shooting day already has a call sheet.");
        });
        await t.keys("Escape");
      } finally {
        for (const id of made) await swallow(t.op("callsheets.delete", { id }));
      }
    });

    // The day of a call sheet is deleted.
    await flow("cs-day-deleted", async () => {
      await t.state(
        "prod-schedule-day-deleted",
        async () => {
          await scheduleTab();
          await dayMenu(4, "Delete day");
          await t.waitText("returned to Unscheduled");
        },
        { keepToasts: true },
      );
      await toastsAway();
      await t.state("cs-list-day-deleted", async () => {
        await t.workspace("Call Sheets");
        await t.waitText("(day deleted)");
      });
      await t.state("cs-day-deleted-banner", async () => {
        await t.go('(//tbody//tr[contains(normalize-space(.),"day deleted")]//button[normalize-space(.)="Open"])[1]');
        await t.waitText("The shooting day of this call sheet was deleted.");
      });
      // The delete button has no confirmation; its toast offers Undo.
      await t.state(
        "cs-deleted-toast",
        async () => {
          await t.press("Delete call sheet");
          await t.el(SEARCH_SHEETS);
          await t.waitText("Deleted ");
        },
        { keepToasts: true },
      );
      await toastsAway();
    });
  } finally {
    // Leave the seeded project as the seed made it.
    await t.closeOverlays().catch(() => undefined);
    for (const id of castElements) await swallow(t.op("breakdown.remove_element", { id }));
    for (const s of (await swallow(t.op("sides.list", {}))) ?? []) await swallow(t.op("sides.delete", { id: s.id }));
    for (const r of (await swallow(t.op("reports.list", {}))) ?? []) await swallow(t.op("reports.delete", { id: r.id }));
    for (const r of (await swallow(t.op("callsheets.list", {}))) ?? []) {
      if (!r.dayExists || r.dayLabel === "Shoot Day 4") await swallow(t.op("callsheets.delete", { id: r.id }));
    }
    const left = await swallow(sched());
    for (const d of left?.days ?? []) {
      if (!base.days.some((b) => b.id === d.id)) await swallow(t.op("schedule.delete_day", { dayId: d.id }));
    }
    const sc = (await swallow(sched()))?.schedule;
    if (sc && (sc.strictValidation !== base.schedule.strictValidation || sc.status !== base.schedule.status)) {
      await swallow(
        t.op("schedule.update_settings", { scheduleId, name: sc.name, strictValidation: base.schedule.strictValidation, dayDurationMinutes: sc.dayDurationMinutes, expectedRev: null }),
      );
      if (sc.status !== base.schedule.status) await swallow(t.op("schedule.set_status", { scheduleId, status: base.schedule.status }));
    }
    const snaps = (await swallow(t.op("budget.get", {})))?.snapshots ?? [];
    for (const s of snaps) await swallow(t.op("budget.delete_snapshot", { id: s.id }));
  }

  // ------------------------------------------------------------------ Empty states, in the empty project
  await t.inEmptyProject(async () => {
    await t.state("prod-schedule-empty-no-source", async () => {
      await t.productionTab("Schedule");
      await t.waitText("Create a shooting schedule from your screenplay.");
    });
    await t.state("prod-dood-empty", async () => {
      await t.productionTab("Day Out of Days");
      await t.waitText("Day Out of Days comes from your shooting schedule.");
    });
    await t.state("prod-daily-empty", async () => {
      await t.productionTab("Daily View");
      await t.waitText("Select or create a shooting day.");
    });
    await t.state("prod-sides-empty", async () => {
      await t.productionTab("Sides");
      await t.waitText("Sides need a shooting schedule.");
    });
    await t.state("prod-reports-empty", async () => {
      await t.productionTab("Reports");
      await t.sleep(600);
    });
    await t.state("prod-budget-start", async () => {
      await t.productionTab("Budget");
      await seen("Start a budget");
    });
    await t.state("cs-list-empty", async () => {
      await t.workspace("Call Sheets");
      await t.waitText("Create a call sheet from a shooting day.");
    });
    await t.state("cs-create-dialog-no-schedule", async () => {
      await t.press("Create Call Sheet");
      await dialogOpen("Create Call Sheet");
      await t.waitText("Create a shooting schedule and a shooting day first.");
    });
  });
  await t.reset();
}
