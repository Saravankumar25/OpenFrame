// @e2e-data-root: own
//
// Every export format a writer uses (Windows phase W4, "every export format"). The screenplay goes
// out as PDF, Final Draft, Fountain, DOCX and plain text, and each text format comes back in as a
// new draft with its three scenes and its dialogue; the Beat Board goes out as PDF; the
// diagnostics bundle is written where the person chose. Production exports (schedule, Day Out of
// Days, call sheet, sides, reports, shot list, storyboard sheet, moodboard) are proved in specs 61
// and 62, and packages in spec 70. The exports call the same operations the Export dialogs call,
// with a path in the spec's own data folder, so no system file dialog is needed.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { click, launchApp, openBeatBoard, stepper } from "../lib/app.mjs";
import { firstLaunchProject, importFountain, menuItem } from "../lib/flows.mjs";
import { NIGHT_BUS } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "every-export");
const dir = join(process.env.E2E_DATA_ROOT ?? ".", "exports");

/** Call an application operation exactly as the UI does (ipc/client.ts `call`). */
async function op(name, args) {
  return browser.execute((n, a) =>
    window.__TAURI_INTERNALS__
      .invoke("of_invoke", { op: n, args: a })
      .then((v) => ({ ok: v }))
      .catch((e) => ({ err: e })), name, args).then((r) => {
    if (r.err) throw new Error(`${name} failed: ${JSON.stringify(r.err)}`);
    return r.ok;
  });
}

const written = (path, min = 100) => existsSync(path) && statSync(path).size >= min;

describe("every export format", () => {
  before(async () => {
    browser = await launchApp();
    mkdirSync(dir, { recursive: true });
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("the screenplay exports as PDF, Final Draft, Fountain, DOCX and plain text, and each text format imports back", async () => {
    await step("screenplay", async () => {
      await firstLaunchProject(browser, "Out And Back");
      await importFountain(browser, NIGHT_BUS);
      const ov = await op("screenplay.overview", {});
      const draftId = ov.drafts.find((d) => d.isCurrent).id;
      for (const format of ["pdf", "fdx", "fountain", "docx", "txt"]) {
        const path = join(dir, `night-bus.${format}`);
        await op("screenplay.export", { draftId, format, path, options: { titlePage: true, sceneNumbers: true } });
        assert.ok(written(path), `the ${format} file was not written`);
      }
      assert.ok(readFileSync(join(dir, "night-bus.pdf"), "latin1").startsWith("%PDF-"), "the PDF is not a PDF");
      assert.ok(readFileSync(join(dir, "night-bus.fountain"), "utf8").includes("You came back."));
      for (const format of ["fdx", "fountain", "docx", "txt"]) {
        const prev = await op("screenplay.import_preview", { path: join(dir, `night-bus.${format}`) });
        const rep = await op("screenplay.import_apply", { previewId: prev.previewId, mode: "new_draft", screenplayId: ov.screenplay.id, draftName: `Back from ${format}` });
        const doc = await op("screenplay.document", { draftId: rep.draftId });
        const texts = doc.scenes.flatMap((s) => s.elements.map((e) => e.text));
        assert.equal(doc.scenes.length, 3, `${format}: ${doc.scenes.length} scenes came back`);
        assert.ok(texts.includes("You came back."), `${format}: the dialogue did not come back`);
        assert.ok(doc.scenes.some((s) => s.heading.includes("BUS STOP")), `${format}: the scene heading did not come back: ${doc.scenes.map((s) => s.heading).join(" | ")}`);
      }
    });
  });

  test("the Beat Board exports as PDF", async () => {
    await step("story", async () => {
      await openBeatBoard(browser);
      await click(browser, "Add Act");
      await (await browser.$('//input[@aria-label="Act title"]')).setValue("Act One");
      await browser.keys("Enter");
      await click(browser, "Add");
      await menuItem(browser, "Scene Card");
      await (await browser.$('//textarea[@aria-label="Short description"]')).setValue("Meera waits at the bus stop");
      await browser.keys("Enter");
      const path = join(dir, "story-board.pdf");
      await op("story.export_board", { format: "pdf", path });
      assert.ok(written(path, 500), "the Beat Board PDF was not written");
      assert.ok(readFileSync(path, "latin1").startsWith("%PDF-"));
    });
  });

  test("the diagnostics bundle is written where the person chose, and holds no project text", async () => {
    await step("diagnostics", async () => {
      const path = join(dir, "diagnostics.zip");
      await op("diagnostics.export_bundle", { path });
      assert.ok(written(path, 22), "the diagnostics bundle was not written");
      const bytes = readFileSync(path);
      assert.equal(bytes.toString("latin1", 0, 2), "PK", "the bundle is not a zip file");
      assert.ok(!bytes.toString("latin1").includes("Meera waits at the bus stop"), "project text reached the diagnostics bundle");
    });
  });
});
