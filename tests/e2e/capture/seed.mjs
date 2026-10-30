// Seeds the WindowsHTML capture data root and captures the first-run screens on the way.
//
// Through the UI: the first-launch name dialog, empty Application Home, the New Project dialog
// (empty, error, filled), the project "Empty Short" (kept empty for the empty states of every
// workspace), then "Night Bus" with the bundled example screenplay imported through the Import
// Screenplay dialog. The rest of Night Bus is seeded through the app's own operations so every
// section starts from the same filled project: Beat Board acts, sequences, cards and beats,
// characters, Story page text, Idea Vault items, a second draft with comments, notes, tasks, the
// production source with a breakdown, catalog items, locations, cast, crew, shots, a storyboard,
// a moodboard, a schedule with days, call sheets, a budget and project files. The draft lock is
// left to the screenplay section (it locks and unlocks).
//
// Every name is invented (owner rule for demo data).

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { encodePng } from "../lib/png.mjs";

/** A small picture (a dusk gradient with a horizon) as base64 PNG, for vault, storyboard and moodboard items. */
export function picture(w = 320, h = 200, hue = 0) {
  const data = Buffer.alloc(w * h * 4);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const o = (y * w + x) * 4;
      const sky = y < h * 0.62;
      const t = y / h;
      data[o] = sky ? 240 - t * 120 + hue : 40 + hue / 2;
      data[o + 1] = sky ? 150 - t * 60 : 52;
      data[o + 2] = sky ? 90 + t * 120 - hue : 70;
      data[o + 3] = 255;
    }
  }
  return encodePng({ width: w, height: h, data }).toString("base64");
}

/** The id inside an operation result (a plain id, `{ id }`, or an object that wraps one). */
export function idOf(x) {
  if (typeof x === "string") return x;
  if (!x || typeof x !== "object") return null;
  if (typeof x.id === "string") return x.id;
  for (const v of Object.values(x)) {
    const id = idOf(v);
    if (id) return id;
  }
  return null;
}

/** Run a seed step; a failure is logged and the seed goes on (sections note what they miss). */
async function soft(label, fn) {
  try {
    return await fn();
  } catch (e) {
    console.log(`  seed step failed: ${label}: ${(e instanceof Error ? e.message : String(e)).split("\n")[0].slice(0, 300)}`);
    return null;
  }
}

export async function seed(t, { dataRoot }) {
  const b = t.browser;

  // First start: the library setup screen when the run sets OPENFRAME_LIBRARY_SETUP=1 (parallel.mjs does).
  const SETUP = "Set up your OpenFrame library";
  await b
    .waitUntil(async () => /Set up your OpenFrame library|Welcome to OpenFrame Studio/.test(await t.text("body")), { timeout: 30_000 })
    .catch(async () => {
      throw new Error(`neither the library setup screen nor the welcome dialog showed; the window shows: ${(await t.text("body")).slice(0, 300)}`);
    });
  if ((await t.text("body")).includes(SETUP)) {
    await t.state("app-library-setup-default");
    await t.state("app-library-setup-new-location", () => t.press("Choose a new location"));
    await t.state("app-library-setup-locate", () => t.press("Locate existing OpenFrame data"));
    await t.press("Use the default location");
    await t.press("Create Library Here");
    await t.waitText(SETUP, { timeout: 30_000, present: false });
  }

  // First launch.
  await t.waitText("Welcome to OpenFrame Studio", { timeout: 30_000 });
  await t.state("app-first-launch-welcome");
  await t.state("app-first-launch-name-required", async () => {
    await t.type("#dn-name", "");
    await t.waitText("Enter your name.");
  });
  await t.type("#dn-name", "Maya Iyer");
  await t.press("Continue", '//*[@role="dialog"]');
  await b.$('//main[@aria-label="Your projects"]').waitForExist({ timeout: 15_000 });
  await t.state("app-home-empty");

  // The empty project.
  await t.state("app-new-project-dialog", () => t.press("Create Project"));
  await t.state("app-new-project-title-required", async () => {
    await (await t.el("#np-title")).click();
    await t.keys("Tab");
    await t.waitText("A title is required.");
  });
  await t.state("app-new-project-filled", async () => {
    await t.type("#np-title", "Empty Short");
    await t.press("Short Film", '//*[@role="dialog"]');
  }, { blur: false });
  await t.state("app-new-project-optional-details", async () => {
    await t.go('//*[@role="dialog"]//summary');
  });
  await t.press("Create", '//*[@role="dialog"]');
  await b.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
  await t.state("home-new-project");
  await t.projectMenu("Close project");
  await b.$('//main[@aria-label="Your projects"]').waitForExist({ timeout: 20_000 });

  // Night Bus.
  await t.press("New Project");
  await t.type("#np-title", "Night Bus");
  await t.press("Create", '//*[@role="dialog"]');
  await b.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
  await t.sleep(800);

  await t.state("sp-import-source", async () => {
    await t.press("Create");
    await t.menuItem("Import Screenplay");
    await t.el("#ix-paste");
  });
  await t.state("sp-import-sample-chosen", () => t.go('//*[@role="radiogroup"][@aria-label="Sample script"]//*[@role="radio"][1]'));
  await t.state("sp-import-preview", async () => {
    await t.press("Preview", '//*[@role="dialog"]');
    await t.waitText("Import Screenplay · Preview");
  });
  await t.state("sp-import-complete", async () => {
    await t.press("Import", '//*[@role="dialog"]');
    await t.waitText("Import complete", { timeout: 30_000 });
  });
  await t.press("Open Screenplay", '//*[@role="dialog"]');
  await b.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 15_000 });
  await t.sleep(1000);

  // Data through operations.
  const ov = await t.op("screenplay.overview", {});
  const draft1 = ov.drafts.find((d) => d.isCurrent) ?? ov.drafts[0];
  const doc1 = await t.op("screenplay.document", { draftId: draft1.id });
  const scenes1 = doc1.scenes;

  const draft2 = await soft("second draft", async () => {
    const d = await t.op("screenplay.new_draft", { sourceDraftId: draft1.id, name: "Second Draft", note: "Tighter opening, new rooftop beat", makeCurrent: true });
    const id = idOf(d);
    const doc = await t.op("screenplay.document", { draftId: id });
    await t.op("screenplay.apply_edits", {
      draftId: id,
      ops: [
        { op: "insertElement", sceneId: doc.scenes[0].id, index: 1, elementType: "action", text: "A stray dog trots after the bus, then gives up at the bridge." },
        { op: "insertElement", sceneId: doc.scenes[2].id, index: 1, elementType: "action", text: "Wind rattles the antenna. Somewhere below, a radio plays an old film song." },
      ],
    });
    return { id, scenes: (await t.op("screenplay.document", { draftId: id })).scenes };
  });
  const cur = draft2 ?? { id: draft1.id, scenes: scenes1 };

  await soft("comments", async () => {
    const c1 = await t.op("comment.create", { targetType: "screenplay_scene", targetId: cur.scenes[1].id, body: "Can we see the shop sign before she walks in?", sceneId: cur.scenes[1].id });
    await t.op("comment.reply", { parentId: idOf(c1), body: "Yes, an insert of the faded board works." });
    await t.op("comment.create", { targetType: "screenplay_scene", targetId: cur.scenes[3].id, body: "Venkat's line about the receiver could be shorter.", sceneId: cur.scenes[3].id });
    const c3 = await t.op("comment.create", { targetType: "screenplay_scene", targetId: cur.scenes[0].id, body: "Check the bus number against the art department list.", sceneId: cur.scenes[0].id });
    await t.op("comment.resolve", { id: idOf(c3) });
  });
  await soft("private note", () => t.op("private_note.create", { targetType: "screenplay_scene", targetId: cur.scenes[0].id, body: "Open on sound only: the bus horn before the picture." }));

  // Beat Board.
  await soft("beat board", async () => {
    const act = async (title) => (await t.op("story.create_act", { episodeId: null, title, note: null, beforeId: null })).id;
    const a1 = await act("Act 1: The Return");
    const a2 = await act("Act 2: The Tower");
    const a3 = await act("Act 3: The Signal");
    const seq = async (actId, title) => (await t.op("story.create_sequence", { actId, title, index: null })).id;
    const s1 = await seq(a1, "Arrival in Rayapuram");
    const s2 = await seq(a2, "Climbing the tower");
    const card = (parentType, parentId, shortDescription, sceneHeading, longDescription = null) =>
      t.op("story.create_card", { parent: { parentType, parentId }, episodeId: null, shortDescription, sceneHeading, longDescription, notes: null, index: null });
    await card("sequence", s1, "Lakshmi steps off the night bus", "EXT. RAYAPURAM BUS STAND - NIGHT", "The last bus drops her at the empty stand. The town she left is smaller than she remembers.");
    await card("sequence", s1, "Grandfather at the workbench", "INT. VENKAT RADIO WORKS - NIGHT");
    await card("act", a1, "The district office refuses the repair", "INT. DISTRICT OFFICE - DAY");
    await card("sequence", s2, "The rusted ladder", "EXT. TOWER - DUSK");
    await card("sequence", s2, "Farooq brings the spare valve", "INT. FAROOQ'S BUS - NIGHT");
    await card("act", a3, "The first broadcast", "INT. COMMUNITY HALL - NIGHT", "The whole village listens as the signal comes through.");
    await t.op("story.create_beat", { parent: { parentType: "act", parentId: a1 }, episodeId: null, text: "She decides to stay one more night", color: "orange", index: null });
    await t.op("story.create_beat", { parent: { parentType: "act", parentId: a2 }, episodeId: null, text: "Midpoint: the tower can be fixed", color: "blue", index: null });
    await t.op("story.create_beat", { parent: { parentType: "act", parentId: a3 }, episodeId: null, text: "The storm tests the repair", color: null, index: null });
    await card("parking", null, "A dream about the old cinema", null);
  });

  await soft("characters", async () => {
    const ch = async (name, roleLabel, description) =>
      (await t.op("story.create_character", { name, roleLabel, description, notes: null, episodeId: null, imagePath: null })).id;
    const l = await ch("LAKSHMI", "Lead", "An engineering graduate who comes home to Rayapuram for one night and stays.");
    const v = await ch("VENKAT RAO", "Supporting", "Her grandfather, who still repairs radios nobody brings in.");
    const f = await ch("FAROOQ", "Supporting", "Drives the night bus and knows every rumour on the coast road.");
    await ch("PADMA", "Minor", "Runs the community hall and the only telephone that works.");
    await t.op("story.create_relationship", { fromCharacterId: l, toCharacterId: v, relationshipType: "Granddaughter", note: "She learned soldering at his bench." });
    await t.op("story.create_relationship", { fromCharacterId: f, toCharacterId: v, relationshipType: "Old friend", note: null });
  });

  await soft("story page", () =>
    t.op("story.page.save", {
      text:
        "Night Bus\n\nLakshmi returns to Rayapuram on the last bus of the night to sell her grandfather's radio shop. " +
        "The coastal village has lost its only broadcast tower to a storm, and with it the warnings fishermen depend on.\n\n" +
        "Over one monsoon week she and Venkat Rao, helped by Farooq the bus driver, bring the old tower back to life. " +
        "The repair is small. What it gives the village is not.",
      expectedRev: 0,
    }),
  );

  // Idea Vault.
  await soft("idea vault", async () => {
    const folder = await t.op("vault.create_folder", { store: "project", name: "Locations", parentId: null });
    const coll = await t.op("vault.create_collection", { store: "project", name: "Look and feel", note: "Colour and light references" });
    const folderId = idOf(folder);
    const collectionId = idOf(coll);
    const add = (args) => t.op("vault.create", { store: "project", title: null, body: null, caption: null, url: null, sourceText: null, folderId: null, collectionId: null, tags: [], ...args });
    const n1 = await add({ itemType: "note", title: "Opening image", body: "The bus headlights sweep over the shuttered shops before we see anyone.", tags: ["opening", "visual"] });
    await add({ itemType: "note", title: "Radio sounds", body: "Record real shortwave static at the harbour for the tower scenes.", tags: ["sound"] });
    await add({ itemType: "quote", body: "A town is only as far away as its last bus.", sourceText: "Overheard at the Rayapuram stand", tags: ["theme"] });
    await add({ itemType: "url", title: "Coastal weather archive", url: "https://example.com/coastal-weather", body: "Storm dates for the 1990s.", collectionId });
    const img = async (title, caption, hue) =>
      t.op("vault.ingest_bytes", { store: "project", itemType: "image", fileName: `${title.toLowerCase().replace(/\W+/g, "-")}.png`, mediaType: "image/png", dataBase64: picture(320, 200, hue), durationMs: null, title, caption, folderId, collectionId });
    await img("Harbour at dusk", "Location scout photo, east jetty", 0);
    await img("Tower silhouette", "Reference for the Act 2 climb", 40);
    const gone = await add({ itemType: "note", title: "Old title ideas", body: "The Last Bus. Signal. Tower Song." });
    await t.op("vault.set_pinned", { store: "project", ids: [idOf(n1)], pinned: true });
    await t.op("vault.set_research_flag", { store: "project", ids: [idOf(n1)], flag: "needsExpertCheck" });
    await t.op("vault.delete", { store: "project", ids: [idOf(gone)] });
    await t.op("vault.create", { store: "global", itemType: "note", title: "Ideas for the next film", body: "A night ferry story.", caption: null, url: null, sourceText: null, folderId: null, collectionId: null, tags: ["later"] });
  });

  await soft("notes", async () => {
    await t.op("notes.create", { title: "Monsoon schedule risk", body: "Keep two cover sets ready for rain days in week 2.", pinned: true });
    await t.op("notes.create", { title: "Music", body: "Ask the composer for a theme built on radio static." });
    await t.op("notes.create", { title: "Permits", body: "Tower climb needs the panchayat letter and a safety officer." });
  });
  await soft("tasks", async () => {
    const day = 24 * 3600 * 1000;
    await t.op("tasks.create", { title: "Scout the east jetty at dusk", dueAt: Date.now() + 3 * day, ownerUserId: null, notes: "Bring the light meter.", related: null });
    await t.op("tasks.create", { title: "Confirm the bus with the depot", dueAt: Date.now() + 7 * day, ownerUserId: null, notes: null, related: null });
    await t.op("tasks.create", { title: "Book the safety officer", dueAt: null, ownerUserId: null, notes: null, related: null });
    const done = await t.op("tasks.create", { title: "Lock the shooting script", dueAt: null, ownerUserId: null, notes: null, related: null });
    await t.op("tasks.set_done", { id: idOf(done), done: true });
  });

  // Production.
  await soft("production source", () => t.op("production.set_source", { draftId: cur.id, reason: "Approved for prep" }));
  await soft("breakdown", async () => {
    const list = await t.op("breakdown.scenes", { filter: null, category: null, catalogItemId: null });
    const scenes = list?.scenes ?? list ?? [];
    for (const s of scenes.slice(0, 4)) await t.op("breakdown.suggest", { sceneId: s.id ?? s.sceneId });
    const first = scenes[0]?.id ?? scenes[0]?.sceneId;
    await t.op("breakdown.add_element", { sceneId: first, category: "Props", name: "Soldering iron", notes: "Working, period model", span: null, catalog: { mode: "auto", catalogItemId: null } });
    await t.op("breakdown.add_element", { sceneId: first, category: "Vehicles", name: "Night bus", notes: null, span: null, catalog: { mode: "auto", catalogItemId: null } });
    await t.op("breakdown.set_complete", { sceneId: first, complete: true });
  });
  await soft("catalog", async () => {
    await t.op("catalog.create", { category: "Props", name: "Marine radio receiver", description: "Valve set, 1970s", notes: null });
    await t.op("catalog.create", { category: "Wardrobe", name: "Lakshmi's travel kurta", description: "Faded indigo" });
  });
  await soft("locations", async () => {
    await t.op("locations.create", { name: "Rayapuram bus stand", address: "Coast road, Rayapuram", contact: "Panchayat office", status: "Confirmed" });
    await t.op("locations.create", { name: "Venkat Radio Works", address: "Market street", status: "Shortlisted" });
    await t.op("locations.create", { name: "Pedda Revu tower", address: "North headland", nearestHospital: "District hospital, 14 km", emergencyContact: "Coast guard station", status: "Idea" });
    await t.op("locations.create", { name: "Old cinema hall", address: "Station road", status: "Rejected" });
  });
  await soft("cast and crew", async () => {
    await t.op("cast.create", { personName: "Ananya Rao", characterName: "LAKSHMI", primary: true, contact: "agent@example.com" });
    await t.op("cast.create", { personName: "Suresh Babu", characterName: "VENKAT RAO", primary: true });
    await t.op("cast.create", { personName: "Imran Khan", characterName: "FAROOQ", primary: true });
    await t.op("crew.create", { personName: "Divya Menon", role: "Director of Photography", department: "Camera" });
    await t.op("crew.create", { personName: "Karthik S", role: "Production Designer", department: "Art" });
    await t.op("crew.create", { personName: "Neha Joshi", role: "Sound Recordist", department: "Sound" });
    await t.op("crew.create", { personName: "Ravi Teja", role: "Line Producer", department: "Production" });
  });
  await soft("shots", async () => {
    const shot = (sceneId, description, size, movement) => t.op("shot.create", { sceneId, description, size, movement });
    await shot(cur.scenes[0].id, "Wide: the bus arrives at the empty stand", "Wide", "Static");
    await shot(cur.scenes[0].id, "Lakshmi steps down, bag first", "Medium", "Handheld");
    await shot(cur.scenes[0].id, "Close on the faded route board", "Close-up", "Static");
    await shot(cur.scenes[1].id, "Over the shoulder: Venkat at the bench", "Medium", "Dolly in");
  });
  await soft("storyboard", async () => {
    const sb = await t.op("storyboard.create", { name: "Arrival", sceneId: cur.scenes[0].id });
    const storyboardId = idOf(sb);
    await t.op("storyboard.add_panel", { storyboardId, visual: "image", dataBase64: picture(320, 180, 10), description: "Headlights sweep the stand" });
    await t.op("storyboard.add_panel", { storyboardId, visual: "image", dataBase64: picture(320, 180, 60), description: "Lakshmi on the step" });
    await t.op("storyboard.add_panel", { storyboardId, visual: "placeholder", description: "Insert: route board" });
  });
  await soft("moodboard", async () => {
    const mb = await t.op("moodboard.create", { name: "Coastal light" });
    const moodboardId = idOf(mb);
    await t.op("moodboard.add_image_data", { moodboardId, dataBase64: picture(320, 200, 20), fileName: "harbour.png" });
    await t.op("moodboard.add_note", { moodboardId, text: "Sodium orange at night, cyan before dawn.", isPrivate: false, x: 420, y: 60 });
    await t.op("moodboard.add_link", { moodboardId, url: "https://example.com/monsoon-palette", title: "Monsoon palette" });
  });
  await soft("schedule", async () => {
    await t.op("schedule.create", { name: null });
    let view = await t.op("schedule.get", {});
    const scheduleId = view.schedule.id;
    await t.op("schedule.create_day", { scheduleId, date: "2026-11-09", notes: "Night exteriors", offDay: false, afterDayId: null });
    view = await t.op("schedule.get", {});
    const day = (i) => view.days[i]?.id;
    await t.op("schedule.create_day", { scheduleId, date: "2026-11-10", notes: null, offDay: false, afterDayId: day(0) });
    view = await t.op("schedule.get", {});
    await t.op("schedule.create_day", { scheduleId, date: "2026-11-11", notes: null, offDay: false, afterDayId: day(1) });
    view = await t.op("schedule.get", {});
    const strips = view.unscheduled.map((s) => s.id);
    await t.op("schedule.move_strips", { stripIds: strips.slice(0, 3), dayId: day(0), keepAnyway: true });
    await t.op("schedule.move_strips", { stripIds: strips.slice(3, 6), dayId: day(1), keepAnyway: true });
    await t.op("schedule.move_strips", { stripIds: strips.slice(6, 8), dayId: day(2), keepAnyway: true });
    await t.op("schedule.add_marker", { dayId: day(0), markerType: "Meal", label: "Dinner", atTime: null, durationMinutes: 45, notes: null, index: null });
    await t.op("schedule.set_day_times", { dayId: day(0), crewCall: 15 * 60, shootingStart: 16 * 60 + 30, targetWrap: 23 * 60 + 30 });
  });
  await soft("call sheets", async () => {
    const view = await t.op("schedule.get", {});
    const cs1 = await t.op("callsheets.create", { dayId: view.days[0].id });
    await t.op("callsheets.add_section", { id: idOf(cs1), section: "weather" });
    await t.op("callsheets.create", { dayId: view.days[1].id });
  });
  await soft("budget", async () => {
    const bud = await t.op("budget.create", { currency: "INR" });
    const budgetId = idOf(bud);
    const line = (category, description, planned, actual = null) => t.op("budget.add_line", { budgetId, category, description, planned, actual, notes: null });
    await line("Cast", "Lead and supporting cast fees", 45000000, 42000000);
    await line("Crew", "Camera, art and sound crew", 38000000);
    await line("Locations", "Tower permit and bus stand", 6500000, 7200000);
    await line("Equipment", "Camera package and lights", 22000000);
    await line("Travel/Transport", "Night bus hire, unit vans", 9000000);
    await line("Food", "Unit meals for 18 days", 5400000);
  });
  await soft("files", async () => {
    const dir = join(dataRoot, "capture-files");
    mkdirSync(dir, { recursive: true });
    const files = {
      "Location release - bus stand.txt": "Location release for the Rayapuram bus stand.\nSigned by the panchayat office.\n",
      "Crew contacts.csv": "Name,Role,Phone\nDivya Menon,DOP,555-0101\nKarthik S,Production Designer,555-0102\n",
      "Tower survey notes.md": "# Tower survey\n\nLadder rungs 31 to 36 need replacing before the climb.\n",
    };
    for (const [name, text] of Object.entries(files)) writeFileSync(join(dir, name), text);
    const folder = await t.op("files.create_folder", { name: "Contracts", parentId: null });
    await t.op("files.add", { paths: [join(dir, "Location release - bus stand.txt")], folderId: idOf(folder), mode: "copy" });
    await t.op("files.add", { paths: [join(dir, "Crew contacts.csv"), join(dir, "Tower survey notes.md")], folderId: null, mode: "copy" });
  });

  // Reload so every workspace reads the seeded data, then start on Home.
  await b.refresh();
  await b.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 30_000 });
  await t.reset();
}
