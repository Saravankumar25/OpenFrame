// Body of the Themes page: how to make, import, check and share a custom theme. The six starter files in
// src/static/themes are published as /themes/<id>.json; their swatches are drawn here at build time as SVG with plain
// fill attributes (the site allows no inline styles). Limits and messages follow the app's theme checker.
import fs from 'node:fs';
import { oklch } from './color.mjs';
import { raw } from './html.mjs';

const DIR = new URL('../static/themes/', import.meta.url);
export const STARTERS = fs.readdirSync(DIR).filter((f) => f.endsWith('.json')).sort()
  .map((f) => JSON.parse(fs.readFileSync(new URL(f, DIR), 'utf8')));

export const EXAMPLE = `{
  "formatVersion": 1,
  "id": "my-first-theme",
  "name": "My First Theme",
  "description": "Quiet blue-grey with a warm orange accent.",
  "author": "Your name",
  "seeds": {
    "neutralHue": 250,
    "neutralTint": 0.02,
    "accentHue": 55,
    "accentChroma": 0.16,
    "accentLightnessNudge": 0
  }
}`;

const OVERRIDES_EXAMPLE = `"overrides": {
  "light": { "bg": "#f7f4ee" },
  "dark": { "accent": "oklch(72% 0.15 55)" }
}`;

// Lightness (percent) and tint multiplier of each surface, light and dark, as the app builds a theme from its seeds.
const LADDER = {
  light: { bg: [95.5, 1], panel: [99, 0.35], line: [89.5, 1], ink: [24, 1.6], muted: [53, 1.2], side: [21, 2.2], side2: [27, 2.2], sideMut: [63, 1.2] },
  dark: { bg: [19.5, 1.4], panel: [24, 1.4], line: [31, 1.3], ink: [93, 0.6], muted: [67, 0.8], side: [15, 1.5], side2: [22, 1.5], sideMut: [62, 0.8] },
};

function half(t, mode, x) {
  const s = t.seeds, L = LADDER[mode], at = (k) => oklch(L[k][0] / 100, s.neutralTint * L[k][1], s.neutralHue);
  const accent = oklch((mode === 'light' ? 56 : 70) / 100 + s.accentLightnessNudge / 100, s.accentChroma, s.accentHue);
  const onAccent = mode === 'light' ? '#ffffff' : oklch(0.18, 0.02, s.accentHue);
  return { x, bg: at('bg'), panel: at('panel'), line: at('line'), ink: at('ink'), muted: at('muted'), side: at('side'), side2: at('side2'), sideMut: at('sideMut'), accent, onAccent };
}

export function swatch(t) {
  const part = (h) => `<rect x="${h.x}" y="0" width="120" height="84" fill="${h.bg}"/>
<rect x="${h.x}" y="0" width="26" height="84" fill="${h.side}"/>
<rect x="${h.x + 5}" y="12" width="16" height="5" rx="2.5" fill="${h.side2}"/>
<rect x="${h.x + 5}" y="23" width="16" height="3" rx="1.5" fill="${h.sideMut}"/>
<rect x="${h.x + 5}" y="31" width="12" height="3" rx="1.5" fill="${h.sideMut}"/>
<rect x="${h.x + 34}" y="10" width="78" height="48" rx="5" fill="${h.panel}" stroke="${h.line}"/>
<rect x="${h.x + 42}" y="18" width="50" height="5" rx="2.5" fill="${h.ink}"/>
<rect x="${h.x + 42}" y="28" width="62" height="3.5" rx="1.75" fill="${h.muted}"/>
<rect x="${h.x + 42}" y="36" width="44" height="3.5" rx="1.75" fill="${h.muted}"/>
<rect x="${h.x + 42}" y="44" width="32" height="9" rx="4.5" fill="${h.accent}"/>
<rect x="${h.x + 49}" y="47.5" width="18" height="2.5" rx="1.25" fill="${h.onAccent}"/>
<rect x="${h.x + 34}" y="68" width="42" height="5" rx="2.5" fill="${h.accent}"/>`;
  return `<svg viewBox="0 0 240 84" width="240" height="84" role="img" aria-label="${t.name} in Light on the left and Dark on the right"><title>${t.name}, Light and Dark</title>
${part(half(t, 'light', 0))}
${part(half(t, 'dark', 120))}
</svg>`;
}

const SEED_ROWS = [
  ['neutralHue', 'Needed', '0 to 360', 'The colour tint of the greys: backgrounds, panels, lines. It is a position on the colour wheel. Roughly: 30 red, 70 orange, 100 yellow, 145 green, 200 teal, 260 blue, 310 purple, 350 pink.'],
  ['neutralTint', 'Needed', '0 to 0.04', 'How much of that colour the greys carry. 0 is pure grey, 0.01 is a hint, 0.03 is clearly tinted.'],
  ['accentHue', 'Needed', '0 to 360', 'The colour of buttons, selected items and links, on the same colour wheel.'],
  ['accentChroma', 'Needed', '0 to 0.25', 'How vivid the accent is. 0 is grey, 0.1 is calm, 0.17 is lively.'],
  ['accentLightnessNudge', 'Needed', '-12 to 16', 'Makes the accent lighter (positive) or darker (negative). 0 keeps the standard lightness.'],
];

const TOP_ROWS = [
  ['formatVersion', 'Needed', 'The number 1', 'Says which version of the file layout you are using.'],
  ['id', 'Needed', '3 to 40 characters: lowercase letters a to z, digits and hyphens', 'The internal name. It must be new: not the id of a built-in theme, and not one already in My Themes.'],
  ['name', 'Needed', 'Up to 60 characters', 'The name shown in My Themes. It must have a letter or digit and may not be the name of a built-in theme. The signs < and > are not allowed.'],
  ['description', 'Optional', 'Up to 200 characters', 'One line about the look.'],
  ['author', 'Optional', 'Up to 80 characters', 'Shown as "By ..." under the name.'],
  ['seeds', 'Needed', 'A group with the five numbers below', 'The five numbers that build the whole theme, in Light and in Dark.'],
  ['overrides', 'Optional', 'A group with "light" and "dark"', 'Set single colours yourself. See the next table.'],
];

const TOKEN_ROWS = [
  ['bg', 'The window background'], ['panel', 'Panels, cards and dialogs'], ['surface2', 'Table headers and soft sections'],
  ['surface3', 'Raised areas and button hover'], ['hover', 'A row you point at'], ['well', 'Sunken areas, such as preview panes'],
  ['chrome', 'The status bar at the bottom'], ['ink', 'Main text'], ['ink2', 'Second-level text'], ['muted', 'Quiet text and hints'],
  ['line', 'Thin borders'], ['line2', 'Stronger borders'], ['side', 'The left rail'], ['side2', 'A pointed-at item in the left rail'],
  ['side-ink', 'Text in the left rail'], ['side-mut', 'Quiet text in the left rail'], ['accent', 'Buttons and selected items'],
  ['accent-ink', 'Text on accent buttons'], ['accent-d', 'Accent text, such as a selected item or a link'],
  ['lg1', 'First colour of the logo gradient'], ['lg2', 'Second colour of the logo gradient'], ['lg3', 'Third colour of the logo gradient'],
];

// [what you see, what it means, how to fix it]
export const ERRORS = [
  ['The file', [
    ['Copy the theme file to this computer first (for example to Downloads), then import it.', 'The file sits in a network folder.', 'Copy it to a normal folder on this computer and pick it again.'],
    ['The theme file is larger than 10 KB. Theme files are small; this is not one.', 'A theme file is a few hundred characters. The file you picked is far bigger, so it is probably not a theme.', 'Check that you picked the right file. If it is yours, remove everything the list below does not describe.'],
    ['This is not a theme file. Straft theme files are small JSON documents.', 'The text is not written in the file layout. A missing comma, a missing quote or a missing closing brace is the usual cause.', 'Put a comma after every line except the last one in a group. Use straight double quotes. Start with { and end with }. Compare with the example above.'],
    ['This is not a theme file. It must be one JSON object.', 'The file starts with something other than {, for example a [ or a plain word.', 'Make the first character { and the last character }.'],
    ['The theme is empty.', 'The file or the pasted text has nothing in it.', 'Paste the whole text of the file, from { to }.'],
    ['Please choose a theme file, not a folder or device.', 'You picked something that is not a file.', 'Pick the .json file itself.'],
  ]],
  ['The top of the file', [
    ['The file has a setting Straft does not know: "colour".', 'A name in the file is not one of the allowed names. Misspellings land here.', 'Use only formatVersion, id, name, description, author, seeds and overrides, spelled exactly like that, capital letters included.'],
    ['The file has no "formatVersion". / "formatVersion" must be 1.', 'The version line is missing or has another value.', 'Add "formatVersion": 1.'],
    ['This theme was made for a newer version of Straft. Update Straft to use it.', 'The file uses a number above 1.', 'Update the app, or change the number to 1 if you wrote the file yourself.'],
    ['The id "My Theme" is not allowed. Use 3 to 40 lowercase letters, digits or hyphens.', 'The id has capitals, spaces or other signs, or the wrong length.', 'Write it like my-first-theme.'],
    ['The id "ember-dusk" belongs to a built-in theme. Change the id in the file.', 'You used the id of a built-in theme.', 'Pick another id.'],
    ['A theme with the id "ember-dusk" is already in My Themes. Delete that theme first, or change the id in the file.', 'You imported this id before.', 'Change the id in the file, or delete the old one under My Themes.'],
    ['"Studio Amber" is the name of a built-in theme. Give this theme its own name.', 'The name matches a built-in theme. Capitals and extra spaces do not make it different.', 'Change the name.'],
    ['The theme name is longer than 60 characters. / The description is longer than 200 characters. / The author is longer than 80 characters.', 'A text is too long.', 'Shorten it.'],
    ['The theme name contains characters that are not allowed (invisible or control characters, or < >).', 'The text has hidden characters or the signs < or >.', 'Retype the text by hand.'],
    ['The theme name needs at least one letter or digit.', 'The name is only signs.', 'Add a word.'],
    ['"name" must be text.', 'The value is a number or a group, not words in double quotes.', 'Put it in double quotes.'],
  ]],
  ['The seeds', [
    ['The file has no "seeds" object.', 'The "seeds" group is missing.', 'Add it with all five numbers.'],
    ['Seed "accentHue" is missing.', 'One of the five numbers is missing.', 'Add it.'],
    ['Seed "accentHue" must be between 0 and 360.', 'The number is outside its range. The message names the range.', 'Use a number inside the range from the table above.'],
    ['Seed "accentHue" must be a number.', 'The value is in quotes or is words.', 'Remove the quotes: 250, not "250".'],
    ['"seeds" has a value Straft does not know: "tint".', 'A seed name is misspelled or extra.', 'Use only the five names from the table.'],
  ]],
  ['The overrides', [
    ['"overrides" must be an object with "light" and "dark". / "overrides" may only have "light" and "dark", not "night".', 'The group has the wrong shape or an extra name.', 'Use only "light" and "dark", each holding colours.'],
    ['"overrides.light" must be an object. / "overrides.light" has too many entries.', 'A mode holds the wrong thing, or more entries than there are colours to set.', 'Put one colour per name, inside curly braces.'],
    ['"sidebar" cannot be changed by a theme file. Allowed: bg, panel, ...', 'The colour name is not one of the allowed names. The message lists them.', 'Use a name from the colour table.'],
    ['The colour for "ink" (light) is not allowed. Use #rrggbb or oklch(L% C H).', 'The colour is not written in one of the two accepted ways, or a number is out of range.', 'Write #1b1b1f, #fff, or oklch(24% 0.02 260). The first number is 0 to 100 with a percent sign, the second 0 to 0.4, the third 0 to 360.'],
  ]],
  ['Readability and saving', [
    ['Light mode: Muted text on panels is too hard to read (3.20 to 1; at least 4.5 to 1 is needed).', 'After the app\'s own small corrections, this text is still too faint against its background. The theme is not saved.', 'Make the text colour darker or the background lighter (or the other way round in Dark). Change the colour that the message names, or the seeds, and check again.'],
    ['My Themes is full (100 themes). Delete a theme first. Nothing was changed.', 'My Themes keeps up to 100 themes.', 'Delete one you no longer use.'],
    ['The theme file changed after it was checked. Check it again. Nothing was changed.', 'You edited the file while the result was open.', 'Close the result and import the file again.'],
  ]],
];

const NOTES = [
  'Light mode: button text was set to white so it stays readable.',
  'Light mode: the accent colour was made darker by 2 points so button text stays readable.',
  'Dark mode: selected and accent text were darkened toward the text colour so they stay readable.',
];

export function themesBody(ctx) {
  const { html, osBlocks, mock, cta, THEMES } = ctx;
  const win = html`<ol class="isteps">
<li><b>Put the file on your computer.</b> Save it in Downloads or another normal folder. A file in a network folder is refused.</li>
<li><b>Open Settings.</b> On Home, press the gear. Choose <b>Appearance</b>.</li>
<li><b>Scroll to My Themes.</b> Press <b>Import Theme File…</b> and pick the .json file. The app checks it as soon as you pick it. If you have the text instead of a file, press <b>Paste Theme…</b>, paste the whole text, and press <b>Check Theme</b>.</li>
<li><b>Read the result.</b> You see the theme in Light and in Dark. A green box says the theme is readable. A blue box lists the small readability changes the app made. A red box says the theme cannot be used and lists every problem. Nothing is saved yet.</li>
<li><b>Choose what to do.</b> <b>Save and Apply</b> saves it and uses it now. <b>Save to My Themes</b> keeps it for later. In the red case both buttons stay off: fix the file and import it again. After pasting, <b>Edit the text</b> takes you back to the text.</li>
<li><b>Use it later.</b> Under My Themes each theme has <b>Apply</b>, <b>Rename</b>, <b>Export</b> and <b>Delete</b>. Deleting the theme in use switches you back to Straft Default.</li></ol>
<div class="tshot">${mock('win', 'settings-theme-check-ok', { caption: 'The result of the check on Windows' })}</div>`;
  const mac = html`<ol class="isteps">
<li><b>Put the file on your Mac.</b> Save it in Downloads or another normal folder.</li>
<li><b>Open Settings.</b> Press Command and comma, then choose <b>Appearance</b>.</li>
<li><b>Scroll to My Themes.</b> Press <b>Import Theme File…</b> and pick the .json file. The sheet checks it at once. If you have the text instead of a file, press <b>Paste Theme…</b>, paste the whole text, and press <b>Check Theme</b>.</li>
<li><b>Read the result.</b> The sheet shows the theme in Light and in Dark, then says it is readable, lists the small readability changes the app made, or says the theme cannot be used and lists every problem. Nothing is saved yet.</li>
<li><b>Choose what to do.</b> <b>Save and Apply</b> saves it and uses it now. <b>Save to My Themes</b> keeps it for later. If the theme cannot be used, both stay off: fix the file and import it again.</li>
<li><b>Use it later.</b> Under My Themes each theme has <b>Apply</b>, <b>Rename</b>, <b>Export</b> and <b>Delete</b>. Deleting the theme in use switches you back to Straft Default.</li></ol>
<div class="tshot">${mock('mac', 'settings-theme-paste-sheet', { caption: 'The Paste Theme sheet on the Mac' })}</div>`;
  const shareWin = html`<ol class="isteps">
<li>Open <b>Settings</b>, <b>Appearance</b>, and find the theme under <b>My Themes</b>.</li>
<li>Press <b>Export</b> on its row. The suggested file name is the theme name.</li>
<li>Choose a folder and save. You get a small .json file.</li></ol>`;
  const shareMac = html`<ol class="isteps">
<li>Open <b>Settings</b>, <b>Appearance</b>, and find the theme under <b>My Themes</b>.</li>
<li>Press <b>Export</b> on its row, or choose the theme and press <b>Export Selected Theme…</b> at the bottom of the page.</li>
<li>Choose a folder and save. You get a small .json file.</li></ol>`;

  return html`
<section class="phero"><div class="wrap"><span class="eyebrow">Themes</span><h1>Make your own theme.</h1>
<p class="lead">A custom theme is one small .json file. Download a starter, change a few numbers, import it, and the app checks it for you. Your screenplay pages, PDFs and call sheets stay black on white.</p>
<nav class="tnav" aria-label="On this page"><a href="#starters">Starter themes</a><a href="#import">Import a theme</a><a href="#make">Make your own</a><a href="#fields">Every field</a><a href="#errors">If the check says no</a><a href="#share">Share a theme</a></nav></div></section>

<section class="sec-s pt30"><div class="wrap"><div class="frow">
<div class="ftxt"><h2>What a custom theme is</h2>
<p>A theme changes colours only. Yours is a text file with a name and five numbers: two for the greys, three for the accent. From those the app builds the whole look, in Light and in Dark. You can also set single colours yourself.</p>
<p>The app checks every theme before it keeps it. Text must be easy to read: at least 4.5 to 1 against its background. If a theme is only a little under, the app fixes it for you and tells you what it changed. A theme file cannot do anything except set colours.</p>
<h3 class="mt26">Built-in themes</h3><div class="pill-list">${THEMES.map((t) => html`<span>${t}</span>`)}</div>
<p>These work in Light or Dark, or follow your system with System. High Contrast is for people who need stronger separation.</p></div>
<div>${osBlocks({ win: mock('win', 'settings-appearance', { caption: 'Settings, Appearance, on Windows' }), mac: mock('mac', 'settings-appearance', { caption: 'Settings, Appearance, on the Mac' }) })}</div>
</div></div></section>

<section class="tint sec-s" id="starters"><div class="wrap"><div class="sec-head"><h2>Starter themes</h2><p class="lead">Download one and import it as it is, or open it in any text editor and change the numbers to make it yours. Each preview shows Light on the left and Dark on the right.</p></div>
<div class="tcards">${STARTERS.map((t) => html`<article class="tcard"><div class="tsw">${raw(swatch(t))}</div><h3>${t.name}</h3><p>${t.description}</p>
<a class="btn btn-ghost" href="/themes/${t.id}.json" download="${t.id}.json">Download ${t.id}.json</a></article>`)}</div></div></section>

<section class="sec-s" id="import"><div class="wrap narrow"><div class="sec-head"><h2>Import a theme</h2><p class="lead">The same steps work for a starter, a file a friend sent, or one you made.</p></div>
${osBlocks({ win, mac })}</div></section>

<section class="tint sec-s" id="make"><div class="wrap narrow"><div class="sec-head"><h2>Make your own</h2><p class="lead">Start from a starter, or copy the example below. Open it in a plain text editor such as Notepad or TextEdit (in plain text mode), change it, and save it with the ending .json.</p></div>
<div class="codebox"><pre><code>${EXAMPLE}</code></pre></div>
<ol class="isteps">
<li><b>Give it its own id and name.</b> <code>id</code> is for the app, <code>name</code> is what you see in My Themes. If you started from a starter, change both, or the app refuses the theme as a copy.</li>
<li><b>Change the numbers.</b> Move <code>accentHue</code> to change the colour of buttons, then <code>accentChroma</code> to make it calmer or livelier. Move <code>neutralHue</code> and <code>neutralTint</code> to colour the greys.</li>
<li><b>Keep the shape.</b> Every name and every piece of text sits in double quotes. Numbers have no quotes. Each line ends with a comma except the last line in a group. The file cannot hold notes, so do not add any.</li>
<li><b>Import it and read the check.</b> If the check finds a problem it says exactly what. The list under "If the check says no" explains each message.</li></ol>
<h3 class="mt26">Setting single colours</h3>
<p>If the five numbers do not give the exact colour you want, add an <code>overrides</code> group after <code>seeds</code>, with a comma between them. Colours are written as <code>#rrggbb</code>, <code>#rgb</code>, or <code>oklch(L% C H)</code>, where L is 0 to 100 with a percent sign, C is 0 to 0.4 and H is 0 to 360.</p>
<div class="codebox"><pre><code>${OVERRIDES_EXAMPLE}</code></pre></div>
<p>The readability check still applies to what you set, so the app may adjust button text or accent text, and refuses a colour that leaves text too faint.</p></div></section>

<section class="sec-s" id="fields"><div class="wrap narrow"><div class="sec-head"><h2>Every field</h2><p class="lead">These are all the settings a theme file can have. Anything else is refused.</p></div>
<h3>The file</h3>
<div class="tbl-wrap"><table class="tbl"><thead><tr><th scope="col">Field</th><th scope="col">Needed</th><th scope="col">Allowed</th><th scope="col">What it does</th></tr></thead><tbody>${TOP_ROWS.map((r) => html`<tr><td><code>${r[0]}</code></td><td>${r[1]}</td><td>${r[2]}</td><td>${r[3]}</td></tr>`)}</tbody></table></div>
<h3 class="mt26">The five seeds</h3>
<div class="tbl-wrap"><table class="tbl"><thead><tr><th scope="col">Seed</th><th scope="col">Needed</th><th scope="col">Allowed</th><th scope="col">What it does</th></tr></thead><tbody>${SEED_ROWS.map((r) => html`<tr><td><code>${r[0]}</code></td><td>${r[1]}</td><td>${r[2]}</td><td>${r[3]}</td></tr>`)}</tbody></table></div>
<h3 class="mt26">Colours you can set yourself</h3>
<div class="tbl-wrap"><table class="tbl"><thead><tr><th scope="col">Name</th><th scope="col">What it colours</th></tr></thead><tbody>${TOKEN_ROWS.map((r) => html`<tr><td><code>${r[0]}</code></td><td>${r[1]}</td></tr>`)}</tbody></table></div>
<p class="small-print">The whole file must stay under 10 KB. My Themes keeps up to 100 themes. The file must be saved as ordinary text.</p></div></section>

<section class="tint sec-s" id="errors"><div class="wrap narrow"><div class="sec-head"><h2>If the check says no</h2><p class="lead">When a theme cannot be used, nothing is saved and nothing changes. The red box lists every problem at once, using these words. Fix them all, then import the file again.</p></div>
${ERRORS.map(([group, rows]) => html`<h3 class="mt26">${group}</h3><div class="tbl-wrap"><table class="tbl"><thead><tr><th scope="col">What you see</th><th scope="col">What it means</th><th scope="col">How to fix it</th></tr></thead><tbody>${rows.map((r) => html`<tr><td>${r[0]}</td><td>${r[1]}</td><td>${r[2]}</td></tr>`)}</tbody></table></div>`)}
<h3 class="mt26">Changes the app makes for you</h3>
<p>These are not errors. A blue box lists them when a theme is a little under 4.5 to 1. You can save the theme as it is. Examples:</p>
<ul class="ticks">${NOTES.map((n) => html`<li><span>${n}</span></li>`)}</ul></div></section>

<section class="sec-s" id="share"><div class="wrap narrow"><div class="sec-head"><h2>Share a theme</h2><p class="lead">A theme is a file, so sharing it is sending a file. There is no account and no upload.</p></div>
${osBlocks({ win: shareWin, mac: shareMac })}
<p>Send the .json file by email or chat, or put it in a shared folder. The other person imports it with the steps above. Their app checks it again, so a damaged or edited file cannot slip through. Put your name in <code>author</code> if you want credit, and keep a copy: deleting a theme from My Themes cannot be undone.</p></div></section>
${cta()}`;
}
