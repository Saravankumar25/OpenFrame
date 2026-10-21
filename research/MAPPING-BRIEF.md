# Tutorial screen mapping brief (shared by the four mapping agents)

Goal: for each tutorial ID you own, find the ordered captured screens of ONE platform that show the real workflow,
and the exact control the user acts on in each screen. These maps drive animated tutorial players on the public site.
A tutorial with a wrong screen is a failed tutorial: accuracy beats coverage.

Sources (read-only):
- Windows: C:\OpenFrameWebsite\research\WindowsHTML-src\ (README.md, index.html lists every screen with its title,
  light\NNN-name.html are the pages; captured at 1600x1000 from the real app; invisible <a> overlays link a control to
  the screen it leads to, which is strong evidence of the real flow). README lists states that could not be captured.
- Mac: C:\OpenFrameWebsite\research\MacHTML-src\ (README.md, light\pages.json has n/file/title/area/expect texts for
  all 692 screens; pages hold one .el element per accessibility element with data-id and aria-label; an <a class="el"
  href="NNN-x.html"> means that control led to that captured screen).
- App source for checking a flow: Windows C:\OpenFrame\apps\desktop\src ; Mac
  C:\Users\SARAVA~1\AppData\Local\Temp\claude\C--Users-Saravan-Kumar--claude-spares-spare-OBdvmq\9cc92431-6d5c-49b4-9ce4-11e16ae79ad1\scratchpad\mac\mac\apps\macos

Probe tool (run from C:\OpenFrameWebsite):
- `node research/probe.mjs win 003-app-home-empty.html` lists visible controls with a suggested selector and rect.
- `node research/probe.mjs mac 003-home-empty.html '[data-id="home-new-project"]'` checks a selector: it must
  return count 1. Selectors are Playwright selectors: CSS (`[data-id="x"]`, `button[aria-label="Settings"]`),
  `role=button[name="Create Project"]`, or `text=...`. Prefer data-id on Mac and role/aria-label on Windows.
- If a selector matches more than one element, make it unique (`>> nth=0` is allowed only as a last resort).

Output: a JSON array in your own file (named in your prompt). One object per tutorial ID:
{
 "id": "create-project",
 "available": true,                 // false if this platform lacks the workflow or the screens were not captured
 "reason": "",                      // required when available is false or the flow is partial
 "steps": [
  {"screen": "003-app-home-empty.html", "action": "click", "target": "role=button[name=\"Create Project\"]",
   "label": "Click Create Project", "text": null, "key": null},
  {"screen": "004-app-new-project-dialog.html", "action": "type", "target": "role=textbox[name=\"Title\"]",
   "label": "Type a title", "text": "Night Bus", "key": null},
  {"screen": "...", "action": "view", "target": null, "label": "Your new project opens", "text": null, "key": null}
 ],
 "notes": "Exact names of the buttons, menus, fields and sections shown, in the app's own words, plus anything a
           writer must know to describe this flow truthfully on this platform (e.g. Mac uses a sheet, Windows a dialog)."
}
Rules:
- action is one of click, doubleclick, rightclick, type, key, scroll, drag, view. Screen i shows the state BEFORE the
  action; doing the action on target leads to screen i+1. The last step is "view" (the result), target null.
- type: "text" must be what appears typed in the next screen (read it from that page). key: "key" uses the platform's
  words (Windows "Ctrl+S", Mac "Cmd+S").
- 3 to 9 steps per tutorial. Every screen file must exist in light/ (and the same name in dark/). Every non-null
  target must be verified with the probe (count 1). Do not invent screens or states.
- Keep the same seeded project across steps where possible (Windows demo project "Night Bus").
- If a better or missing workflow deserves a tutorial in your area, add it with a new kebab-case id (max 3 extra).
- No em dashes anywhere. Plain English.
When done, reply with a short table: id, available, step count, and any doubts.
