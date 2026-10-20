// Writes WindowsHTML/README.md after a full capture run (tests/e2e/specs/zz-capture.test.mjs):
// how the pages were made, how to make them again, the commit, the check table and what was not
// captured.

import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { OUT } from "./build-windows-html.mjs";

/** Screens that need the operating system or a device, outside the web view the tour drives. */
const OUTSIDE = [
  ["Windows Open, Save and folder dialogs", "They belong to Windows, not the web view (Import, Export, Add File, Browse, Locate, backups and packages)."],
  ["Drag and drop states", "Dragging files from Explorer and the drag previews on the Beat Board, schedule and Idea Vault need a real mouse drag across windows."],
  ["Microphone recording", "Recording a voice note needs a microphone; the Record Note dialog is captured only where the tour can reach it."],
  ["Printing", "The Windows print dialog opened by Print in Print Preview."],
  ["Explorer and other apps", "Reveal in File Manager and Open externally open other programs."],
  ["Failure screens that need a broken disk or a crash", "Save errors, the leave guard, the recovery offer after a forced quit, error boundaries and list load errors."],
];

export function writeReadme({ manifest, results, commit, threshold, maxRatio, width, height }) {
  const bySection = new Map();
  for (const s of manifest.screens) bySection.set(s.section, (bySection.get(s.section) ?? 0) + 1);
  const rows = manifest.screens
    .map((s) => {
      const l = results.find((r) => r.id === s.id && r.mode === "light");
      const d = results.find((r) => r.id === s.id && r.mode === "dark");
      const pct = (r) => (r ? `${(r.ratio * 100).toFixed(3)} %` : "-");
      return `| ${s.id.slice(0, 3)} | [${s.name}](light/${s.id}.html) | ${pct(l)} | ${pct(d)} | ${l?.ok && d?.ok ? "pass" : "FAIL"} |`;
    })
    .join("\n");
  const passed = results.filter((r) => r.ok).length;
  const max = results.length ? Math.max(...results.map((r) => r.ratio)) : 0;
  const unreachable = manifest.unreachable.map((u) => `- **${u.name}** (${u.section}): ${u.why}`).join("\n") || "- None recorded.";
  const sections = [...bySection].map(([k, v]) => `| ${k} | ${v} |`).join("\n");
  const text = `# WindowsHTML

Every screen and state of OpenFrame Studio for Windows, captured from the running app as standalone HTML pages,
in Light and Dark mode of the default theme. It is the source for the website's screen recreations (Chapter 2).

- **${manifest.screens.length} screens and states**, each in \`light/\` and \`dark/\` under the same \`NNN-name.html\`.
- Captured from commit \`${commit}\` at a ${width}x${height} window (the pages are laid out for that size).
- [index.html](index.html) lists every screen with a Light and Dark switch. Controls that lead to another captured
  screen carry an invisible link to it.

## How the pages were made

Nothing here is drawn by hand. The capture tour \`tests/e2e/specs/zz-capture.test.mjs\` drives the real app through
the end-to-end WebDriver harness. It seeds the invented demo project Night Bus (\`tests/e2e/capture/seed.mjs\`: the
bundled example screenplay, Beat Board, characters, Idea Vault, a second draft with comments, breakdown, catalog,
locations, cast and crew, shots, a storyboard, a moodboard, a schedule with days, call sheets, a budget, notes,
tasks and files) and an empty project for empty states, then walks the sections in \`tests/e2e/capture/sections/\`.

For each state, \`tests/e2e/lib/capture.mjs\` takes a full-window screenshot and serializes the web view's own DOM
(the web view is the app): form values become attributes, scroll positions are kept with an invisible
\`scroll-initial-target\` marker, canvas and app-asset images become the same pixels cut from the screenshot, and
scripts are removed. \`scripts/windows-html/build-windows-html.mjs\` stores the app's style sheets once in
\`assets/\` and writes the pages. They use no script and load nothing from the network. Text uses the Windows
fonts Segoe UI and Courier New, as the app does; they are not embedded (their licences do not allow it), so a
computer without them substitutes similar fonts.

## How to make them again

Build the debug app once (\`node tests/e2e/run.mjs --only=zz-capture\` builds it), then:

\`\`\`bash
E2E_CAPTURE=1 node tests/e2e/run.mjs --only=zz-capture --no-build
\`\`\`

The run replaces \`light/\`, \`dark/\`, \`assets/\`, \`index.html\` and this file. Raw screenshots stay in
\`.build-cache/windows-html-capture/\` (not committed; 2400x1500 pixels each at 150 percent display scaling).

## Check

Every page was opened in the same WebView2 at the same size and compared with its screenshot (the visual diff
helper of \`90-visual-baseline\`): a pixel differs when a colour channel differs by more than ${threshold} of 255, and a
page fails above ${(maxRatio * 100).toFixed(1)} % differing pixels. ${passed} of ${results.length} pages passed; the
largest difference was ${(max * 100).toFixed(3)} %.

| Section | Screens |
|---|---|
${sections}

| No. | Screen | Light | Dark | Result |
|---|---|---|---|---|
${rows}

## Not captured

States the app cannot reach from inside the web view, or that need hardware or a failure on purpose:

${OUTSIDE.map(([a, b]) => `- **${a}**: ${b}`).join("\n")}

States the tour recorded as unreachable:

${unreachable}
`;
  writeFileSync(join(OUT, "README.md"), text);
}
