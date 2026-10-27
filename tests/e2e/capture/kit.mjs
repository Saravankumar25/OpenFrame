// The toolkit every WindowsHTML capture section receives (tests/e2e/capture/sections/*.mjs).
//
// A section is `export default async function (t) { ... }`. It starts with the Night Bus project
// open on Home with no dialog, menu or drawer open, captures states with `t.state(name, fn)`, and
// must end the same way (`t.reset()` gets there). Sections run one after another in one data root
// in the full tour, so a section must not delete or rename seeded data that another section uses;
// work on items it creates itself, and undo state changes (unlock, reopen) before it returns.
//
// Names are kebab-case and unique across the whole tour; prefix them with the area
// ("vault-", "story-", "sp-", "bd-", "prod-", "cs-", ...).

import { sleep } from "../lib/app.mjs";

const xq = (s) => (s.includes('"') ? `concat("${s.replace(/"/g, '", \'"\', "')}")` : `"${s}"`);

export function makeKit(getBrowser, cap) {
  const b = () => getBrowser();
  const t = {
    xq,
    sleep,
    get browser() {
      return b();
    },
    $: (sel) => b().$(sel),
    $$: (sel) => b().$$(sel),

    /** XPath for a button, menu item, tab, option or radio by visible text, aria-label or title. */
    btn(name, within = "") {
      const n = xq(name);
      return (
        `${within}//button[not(@disabled)][normalize-space(.)=${n} or @aria-label=${n} or @title=${n}]` +
        ` | ${within}//*[@role="menuitem" or @role="tab" or @role="option" or @role="radio" or @role="menuitemradio" or @role="menuitemcheckbox"][normalize-space(.)=${n} or @aria-label=${n}]`
      );
    },
    /** XPath for a dialog by its heading. */
    dialog(title) {
      return `//*[@role="dialog"][.//h2[normalize-space(.)=${xq(title)}]]`;
    },

    /** Wait for an element (selector or element) and return it. */
    async el(sel, timeout = 10_000) {
      const el = typeof sel === "string" ? await b().$(sel) : sel;
      await el.waitForDisplayed({ timeout, timeoutMsg: `not shown: ${typeof sel === "string" ? sel : "element"}` });
      return el;
    },
    /** Click and note the click as the way to the next captured screen (for links between pages). */
    async go(sel) {
      const el = typeof sel === "string" ? await b().$(sel) : sel;
      await el.waitForClickable({ timeout: 10_000, timeoutMsg: `not clickable: ${typeof sel === "string" ? sel : "element"}` });
      await el.scrollIntoView({ block: "center" }).catch(() => undefined);
      await cap.noteClick(el);
      await el.click();
      await sleep(250);
    },
    /** Click a button or menu item by name (see btn). */
    async press(name, within = "") {
      await t.go(t.btn(name, within));
    },
    /** Click a menu item whose text starts with `text` (menu items carry their shortcut after the label). */
    async menuItem(text) {
      await t.go(`//*[@role="menuitem" or @role="menuitemradio" or @role="menuitemcheckbox"][starts-with(normalize-space(.),${xq(text)})]`);
    },
    /** Right-click (context menu). */
    async rightClick(sel) {
      const el = await t.el(sel);
      await el.scrollIntoView({ block: "center" }).catch(() => undefined);
      await cap.noteClick(el);
      await el.click({ button: "right" });
      await sleep(250);
    },
    async keys(combo) {
      await b().keys(combo);
      await sleep(200);
    },
    /** Type into a field (selector or element), replacing its value. */
    async type(sel, text) {
      const el = await t.el(sel);
      await el.click();
      await b().keys(["Control", "a"]);
      await b().keys("Backspace");
      if (text) await el.setValue(text);
    },
    /** Set a native date or time input (WebDriver cannot drive the picker). */
    async setNative(sel, value) {
      const el = await t.el(sel);
      await b().execute(
        (node, v) => {
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(node, v);
          node.dispatchEvent(new Event("input", { bubbles: true }));
          node.dispatchEvent(new Event("change", { bubbles: true }));
        },
        el,
        value,
      );
    },
    async text(sel = "main") {
      return b().execute((s) => document.querySelector(s)?.innerText ?? "", sel);
    },
    async waitText(text, { sel = "body", timeout = 15_000, present = true } = {}) {
      await b().waitUntil(async () => (await t.text(sel)).includes(text) === present, {
        timeout,
        timeoutMsg: `${present ? "not shown" : "still shown"}: "${text}"`,
      });
    },
    /** Call an app operation directly (seeding only; capture states through the UI). */
    async op(name, args = {}) {
      const r = await b().execute(
        (o, a) =>
          window.__TAURI_INTERNALS__.invoke("of_invoke", { op: o, args: a }).then(
            (v) => ({ ok: v ?? null }),
            (e) => ({ err: typeof e === "string" ? e : JSON.stringify(e) }),
          ),
        name,
        args,
      );
      if (r.err) throw new Error(`${name}: ${r.err}`);
      return r.ok;
    },

    /** Go to a workspace through the left navigation. */
    async workspace(label) {
      await t.go(`//nav[@aria-label="Workspaces"]//button[normalize-space(.)=${xq(label)} or @aria-label=${xq(label)}]`);
      await b().$(`//main[@aria-label=${xq(label)}]`).waitForExist({ timeout: 15_000 });
      await sleep(400);
    },
    /** Production › tab. */
    async productionTab(label) {
      if (!(await (await b().$('//nav[@aria-label="Production"]')).isExisting())) await t.workspace("Production");
      await t.go(`//nav[@aria-label="Production"]//button[normalize-space(.)=${xq(label)}]`);
      await sleep(400);
    },
    /** Story › tab (Story, Board, Outline, Characters, Timeline). */
    async storyTab(label) {
      if (!(await (await b().$('//*[@role="tablist"][@aria-label="Story views"]')).isExisting())) await t.workspace("Story");
      await t.go(`//*[@role="tablist"][@aria-label="Story views"]//*[@role="tab"][normalize-space(.)=${xq(label)}]`);
      await sleep(400);
    },
    /** Open the project menu and choose an item (Project settings, Notes & Tasks, ...). */
    async projectMenu(item) {
      await t.go('//button[starts-with(@aria-label,"Project: ")]');
      await t.menuItem(item);
      await sleep(400);
    },
    /** Settings page tab (role=tab in the Settings tablist). */
    async settingsTab(label) {
      await t.go(`//*[@role="tablist"][@aria-label="Settings"]//*[@role="tab"][normalize-space(.)=${xq(label)}]`);
      await sleep(300);
    },

    /** Close menus, popovers, drawers and dialogs with Escape (up to 5 presses). */
    async closeOverlays() {
      for (let i = 0; i < 5; i++) {
        const open = await b().execute(
          () => !!document.querySelector('[role="dialog"], [role="menu"], [role="listbox"], aside.drawer, [data-radix-popper-content-wrapper]'),
        );
        if (!open) return;
        await b().keys("Escape");
        await sleep(250);
      }
    },
    /** Back to the starting state: overlays closed, AI panel closed, Home workspace. */
    async reset() {
      await t.closeOverlays();
      const ai = await b().$('//aside[@aria-label="AI Assistant"]');
      if (await ai.isExisting()) await (await b().$('//button[@aria-label="Close AI Assistant"]')).click().catch(() => undefined);
      const nav = await b().$('//nav[@aria-label="Workspaces"]');
      if (await nav.isExisting()) {
        await (await b().$('//nav[@aria-label="Workspaces"]//button[normalize-space(.)="Home" or @aria-label="Home"]')).click();
        await sleep(400);
      }
    },

    /**
     * Reach a state with `fn` and capture it. A failure is recorded (with a screenshot in
     * tests/e2e/artifacts/) instead of stopping the tour, then overlays are closed.
     * Options are those of capture(): blur, keepToasts, wait, home.
     */
    async state(name, fn, opts) {
      try {
        if (fn) await fn();
        return await cap.capture(name, opts);
      } catch (e) {
        await cap.fail(name, e);
        await t.closeOverlays().catch(() => undefined);
        return null;
      }
    },
    /** Record a state that cannot be reached, and why (listed in WindowsHTML/README.md). */
    unreachable(name, why) {
      cap.unreachable(name, why);
    },

    /** Switch to another project through Application Home (closes the current one). */
    async openProject(title) {
      await t.projectMenu("Close project");
      await b().$('//main[@aria-label="Your projects"]').waitForExist({ timeout: 20_000 });
      await t.go(`//main//button[starts-with(@aria-label,${xq(`Open ${title}`)})]`);
      await b().$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
      await sleep(800);
    },
    /** Run `fn` inside the empty project "Empty Short" (for empty states), then return to Night Bus. */
    async inEmptyProject(fn) {
      await t.closeOverlays();
      await t.openProject("Empty Short");
      try {
        await fn();
      } finally {
        await t.closeOverlays().catch(() => undefined);
        await t.openProject("Night Bus");
      }
    },
  };
  return t;
}
