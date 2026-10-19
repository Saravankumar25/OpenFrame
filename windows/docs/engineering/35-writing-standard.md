# Writing standard

> Owner rule, 2026-10-06: "in the entire project application and in the exported files anywhere wherever it is there should be
> no em dashes and there should be no AI patterns of writing."

This standard covers every piece of text a person can read: the app screens, error messages, toasts, tooltips, exported files
(PDF, Fountain, CSV, reports, call sheets, package manifests that users see), installers, store listings, the website, the user
manual, documents in `docs/`, code comments, commit messages and pull request text.

## Hard rules (a check can enforce these)

1. **No em dashes** (the long dash, U+2014) and **no en dashes used as sentence punctuation** (U+2013). Use a full stop, a comma, a
   colon, brackets or two sentences. A range uses the word "to" ("pages 3 to 7"). A hyphen stays for compound words ("read-only").
2. **No banned phrases.** The list lives in `scripts/check-writing-standard.mjs` and grows over time. It includes: "delve",
   "seamless", "seamlessly", "leverage" (as a verb), "robust" (as filler), "elevate", "unlock the power", "game-changer",
   "cutting-edge", "in today's fast-paced world", "it's important to note", "it is worth noting", "navigate the complexities",
   "a testament to", "tapestry", "supercharge", "harness the power", "streamline your workflow".
3. **No emoji in product text.** Status marks in documents use words.
4. **No decorative labels.** Plans, status files and logs use plain headings that say what the work is. Do not name a section
   "Phase 2", "Wave 3" or "M1.04". A reader must know what a heading means without a lookup table.

## Labels OpenFrame builds (owner decision 2026-10-08)

Labels that OpenFrame puts together itself never join their parts with a dash:

- A kind and number before a name take a colon: "Scene 12: INT. HOSPITAL - DAY", "Shot 2B: Medium",
  "Panel 1: Bike skids", "Act 1: The Return", "Draft 2: blackrain (imported)", "Note: Wool, never leather".
- Two names side by side take a middle dot with spaces: "Black Rain · Draft 2", "Pilot · Monsoon",
  "Arjun · Director".
- An empty table cell or value says what is missing ("None", "Not in this draft", "No scene heading") or stays
  empty; it never shows a lone dash.
- The writer's own text inside a label (a scene heading, a draft name) is shown exactly as typed.

## Style rules (a reviewer enforces these)

- Say the thing. Lead with the action or the answer. Cut the opening sentence that announces what the next sentence will say.
- Prefer short, concrete sentences with one idea each. Use the same word for the same thing every time.
- Use active voice. Name who does what: "OpenFrame saves your work" and not "your work will be saved".
- No hedging stacks ("may potentially help to"). State what is true, or state what is not known.
- No triplet padding ("fast, reliable and secure") unless each word is a tested claim.
- No marketing claims the product cannot prove. Every claim in a listing or on the website maps to a feature that is Verified in
  [feature-status.md](../implementation/feature-status.md).
- Error messages say what happened, what the user's data state is, and what to do next. They never blame the user and never show
  internal names, file paths from other machines or stack traces.
- Numbers use digits with units ("8 GB", "7 days"). Dates are `2026-10-06` in files and plain dates on screen.
- Screenplay and filmmaking words follow industry use: "scene heading", "slugline", "call sheet", "Day Out of Days".

## What is exempt

- Quoted text from other people's documents, audits and archived material. Do not edit `docs/audits/` or `docs/archive/`.
- User content. OpenFrame never rewrites what a person writes in their own screenplay, notes or imports. The rule applies to text
  that OpenFrame itself produces. When OpenFrame writes an export, any em dash in the export comes only from user content.
- Standards that require the character, for example a test fixture that checks dash handling. Put such fixtures in
  `tests/fixtures/` and allow-list the path in the check.

## How it is enforced

- `scripts/check-writing-standard.mjs` scans app source strings, `docs/` (outside the exempt folders), `README.md` and the other
  root documents, and the website folder. It fails on a hard-rule hit. Until the existing hits are cleaned up, the check
  runs as a **ratchet**: it compares each folder's count with `scripts/writing-standard-baseline.json` and fails if a count goes
  up. A cleanup pull request lowers the baseline. The target is zero everywhere.
- The `humanizer` skill is a reviewer aid, not a gate. Run it on listing text, website copy and manual chapters.
- Pull requests that add or change user-visible text tick the box in the pull request template.

## Cleanup rule

When you touch a file, fix its hits in the same pull request. Replace the dash with the punctuation a careful writer would use.
Do not insert a hyphen with spaces around it as a substitute, because that is the same pattern in a smaller dash.
