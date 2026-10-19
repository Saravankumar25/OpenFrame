# 34: UX laws checklist for OpenFrame UI work

> Created 2026-10-05 from the 20 "UX laws" list the owner supplied. Use it as a **review checklist** for any new or changed
> screen, together with the UX/UI specification and `HTML_Mockups/` (which win on labels and layout) and the accessibility
> rules in CONTRIBUTING §3. It adds no requirements of its own: if a law conflicts with the specification, the specification wins.
> The supplied image numbered "Postel's law" twice (16 and 17); it is listed once here, so 19 distinct entries are below plus the
> non-law item 8.

Each row: the rule, then what it means in OpenFrame. Cite the law in the pull request when a design choice relies on it.

| # | Law | Rule | In OpenFrame |
|---|---|---|---|
| 1 | Hick's law | More choices, slower decisions | Keep menus short; progressive disclosure (Advanced for custom elements, page setup, shortcuts); six default call-sheet sections, the rest behind + Add Section |
| 2 | Fitts's law | Big, near targets are quick to hit | Primary actions large and near the work; no tiny icon-only targets; keyboard route for every command |
| 3 | Jakob's law | Users expect your product to work like others they know | Screenwriting conventions (Final Draft habits: Enter flow, Tab, element shortcuts); standard Windows patterns for dialogs, Ctrl+K / Ctrl+F |
| 4 | Law of proximity | Related things sit together | Group fields by section (call sheet, scene drawer); labels next to their controls |
| 5 | Miller's law | Working memory holds about seven items | Chunk long lists (breakdown categories, schedule days), paginate or group; avoid long forms in one view |
| 6 | Doherty threshold | Respond within about 400 ms | Typing, drag, undo and navigation inside budget (doc 21); show progress for anything longer |
| 7 | Von Restorff effect | The odd one out is remembered | One clear primary button per view; warnings and stale states visible but not everywhere |
| 8 | Minimise target distance | Keep related actions and their targets close (a Fitts's-law corollary, not a named law) | Context menus at the pointer; actions inside the card or drawer they affect |
| 9 | Serial position effect | First and last items are remembered best | Put the most used items first and last in menus and navigation; keep destructive items away from the ends |
| 10 | Peak-end rule | People judge by the peak and the end | Make first launch, first project and the end of an export or save feel clear and safe; Undo toasts after destructive actions |
| 11 | Zeigarnik effect | Unfinished tasks stay in mind | Needs Review queues, import reports and stale-state banners show what is unfinished; save-state indicator |
| 12 | Law of Prägnanz | People read the simplest form | Simple layouts and consistent shapes; avoid decorative clutter; screenplay page looks like a screenplay |
| 13 | Law of similarity | Similar things look like they belong together | One style per control type; category colour always paired with text and icon |
| 14 | Law of uniform connectedness | Connected things are seen as related | Use borders, lines or shared backgrounds to bind related items (scene to its breakdown, panel to its shot) |
| 15 | Tesler's law | Complexity cannot vanish, only move | Keep the complexity in the engine (pagination, FDX mapping), not in the writer's way; never push it to the user without need |
| 16 | Postel's law | Be liberal in what you accept, conservative in what you send | Import accepts messy FDX, Fountain, PDF and pasted text with a report; export strictly valid, clean files |
| 17 | Parkinson's law | Work expands to fill the time available | No indefinite spinners; bound long tasks; defaults that finish quickly |
| 18 | Occam's razor | Prefer the simplest explanation or design | Fewer controls, one way to do common things; no feature added only because a competitor has it (Revision 1 §59) |
| 19 | Pareto principle | Most results come from a few causes | Optimise the 20% of flows most used: write, reorder, breakdown, schedule, call sheet |

## How to apply

1. In the feature contract (CONTRIBUTING §3 item 10) list the three laws most relevant to the screen.
2. In review, check the table above after the specification and mock-up. Do not invent a layout that departs from the mock-up on the strength of a law.
3. Pair with the `impeccable` and `frontend-design` skills for design critique, and with doc 30 for theme tokens.
