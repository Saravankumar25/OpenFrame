# Manual and tutorial content: brief and schema

The two user manuals (Windows and macOS, PDF) and the website tutorials are generated from one source per platform:
`src/content/manual/<win|mac>/<area-slug>.mjs`, one file per app area. The manual prints every area in order; every
workflow in an area becomes one website tutorial. Screens are named by their file name without the number
(`cs-create-dialog`, not `653-cs-create-dialog`), so the content survives a recapture that renumbers screens.

## Sources, in order of authority

1. The screens themselves. Windows: `research/WindowsHTML-src/light/NNN-name.html` (the app's own DOM, captured at
   1600x1000). Mac: `research/MacHTML-src/light/NNN-name.html` (window picture plus the accessibility layer). Open a
   page to see what is on it. The extracted control lists are in `research/screens-win.json` and
   `research/screens-mac.json` (title, area, dialog or menu title, buttons, fields with values, toggles, tabs, menu
   items, alerts; Mac: `texts`, the accessibility labels and values).
2. `research/capability-matrix.md`: what is implemented on each platform, audited against the code. A feature that is
   "planned" or "unavailable" there is not described as working.
3. The older tutorials in `.build-removed/tutorials/<win|mac>/*.mjs` (kept outside the build) and `research/tutorials-*.json`: earlier
   step-by-step text checked against screens. Their screen numbers are out of date; match by name.
4. `research/facts.json`: product facts with source lines.

Never describe a control, step, setting or result that no screen or source shows. If a step between two captured
screens is not captured (for example a system file dialog), say in plain words what the person does there without
inventing the dialog's contents. If something is not known, leave it out.

## Writing rules

- Plain, direct English for people who have never used the app. Second person, active voice, imperative steps.
- No em dashes or en dashes anywhere (use a full stop, comma, colon or the word "to"). No emoji.
- No AI-sounding phrases: delve, seamless, leverage, robust, elevate, unlock, game-changer, cutting-edge, tapestry,
  supercharge, harness, streamline, "it's important to note", "it is worth noting".
- Name controls exactly as the screen labels them, in the screen's capitalisation (Create Call Sheet, Add Section).
- Windows uses Ctrl, Alt and right-click menus; the Mac uses Command, Option, the menu bar and Control-click. Never
  copy a Windows instruction into the Mac manual or the reverse: read the Mac screen.
- No internal information: no test-machine paths, no capture tooling, no build or release status, no team testing,
  no passwords. The demo project in the screens is an example; refer to "your project" in steps and mention example
  names only to point at what the picture shows.
- A feature description says what exists; a workflow teaches how to do it. Keep them apart.

## File schema (`src/content/manual/<plat>/<slug>.mjs`)

```js
export default {
  area: 'Call sheets',          // exactly the area name in screens-<plat>.json
  slug: 'call-sheets',          // URL part, lowercase and hyphens
  order: 26,                    // reading order in the manual (see the area order table below)
  part: 'Shoot',                // manual part: Start, Ideate, Write, Plan, Shoot, Assistant, Project and data
  intro: [                      // 2 to 5 short paragraphs: what this area is for and how it is laid out
    'Call sheets tell cast and crew where to be and when on one shooting day. ...',
  ],
  layout: [                     // the parts of the main screen, top to bottom, each control group explained
    { name: 'Call sheet list', text: 'Search, the All, Draft, Final and Needs refresh filter, and Create Call Sheet.' },
  ],
  workflows: [                  // every task a person does here, in a sensible order; each becomes a tutorial
    {
      id: 'create-call-sheet',  // unique within the platform, lowercase and hyphens
      title: 'Create a call sheet from a shooting day',
      purpose: 'One sentence: what you will have done at the end.',
      level: 'Beginner',        // Beginner, Intermediate or Advanced
      prerequisites: ['A shooting day exists in the schedule.'],
      steps: [                  // 2 to 12 steps; each step shows one screen (the state after or during the step)
        { screen: 'cs-list', do: 'Open Call Sheets in the left rail and click Create Call Sheet.',
          explain: 'What the controls on this screen do, what to look at, what each choice means.' },
      ],
      result: 'What you see when it worked.',
      cautions: ['Mistakes, error states and how to recover (name the error screen when one exists).'],
      related: ['finalize-call-sheet'],   // ids of other workflows on the same platform
    },
  ],
  screens: {                    // EVERY screen of this area: one or two sentences, what this state shows and when
    'cs-create-dialog': 'The Create Call Sheet dialog. It asks for the source shooting day ...',
  },
  shortcuts: [['Ctrl+Enter', 'Create the call sheet']],   // only shortcuts a screen or the shortcuts list shows
  limits: ['Plain statements of what this area does not do.'],
  troubleshooting: [{ problem: 'The call sheet says Needs refresh.', fix: 'The schedule changed after ...' }],
};
```

Coverage rule: every screen of the area appears in `screens`, and every screen appears in at least one workflow step
unless it is purely a variant of a screen that does (a scrolled view, a second highlighted item); then its `screens`
text says which workflow it belongs to. `node scripts/check-content.mjs` checks the names, the coverage and the
writing rules.

## Area order

Windows: Library setup, First launch, Application Home, Import a screenplay, Project window and navigation, Idea
Vault, Story and Beat Board, Screenplay, Breakdown, Production Plan: overview and catalog, Locations, Cast and crew,
Moodboards, Storyboards, Shot lists, Shooting schedule, Day Out of Days, Daily View, Sides, Call sheets, Reports,
Budget, AI Assistant, Files, Notes and tasks, Activity, Recently Deleted, Templates, Backups, packages and exchange,
Settings, Trial and purchase.

Mac: Library setup, Home, Menus, Search, Project Home, Idea Vault, Story, Screenplay, Breakdown, Production Plan,
Schedule, Day Out of Days, Daily View, Sides, Call Sheets, Reports, Budget, AI Assistant, Files, Notes and Tasks,
Activity, Recently Deleted, Project Settings, Templates, Packages, Recovery, Settings.
