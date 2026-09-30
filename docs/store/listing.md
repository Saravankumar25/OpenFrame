# Microsoft Store listing draft (en-US)

> Draft for Partner Center > Store listings > English (United States). Field limits are Partner
> Center's. Before submitting, check every claim against the build you upload. The Store rejects
> listings that describe features the package doesn't have (policy 10.1). Lines marked **[AI]**
> apply only when the build ships the local AI assistant. Remove them otherwise.

## Product name

OpenFrame Studio

(Must match the name reserved in Partner Center and the manifest `DisplayName`, which comes from
`packaging/msix/store.config.json` → `displayName`.)

## Description (required, up to 10,000 characters)

OpenFrame Studio is a filmmaking studio for Windows that keeps your whole project in one place,
from the first idea to the call sheet. It is local-first: no account, no subscription to a cloud
service, and your projects stay on your computer.

**[AI]** OpenFrame includes an optional AI writing and planning assistant that uses generative AI.
The assistant runs entirely on your PC. You download it separately from inside the app, and your
project content is never sent to a cloud AI service. AI suggestions are only suggestions. Nothing
changes in your project until you review and apply it. To report inappropriate AI output, use
[support link or e-mail].

**Capture ideas.** Keep loglines, notes, images, audio, video and links in an Idea Vault for each
project, plus a Global Idea Vault you can use across projects.

**Shape the story.** Build your story on a board of acts, sequences, beats and scene cards, or work
in outline view. Track characters, the story timeline and episodes for series.

**Write the screenplay.** Industry-standard screenplay formatting with drafts, review and locked
drafts, and revisions. Import and export Final Draft (FDX), Fountain, PDF and DOCX.

**Plan production.** Break down scenes into cast, props, wardrobe, locations and more. Build shot
lists and storyboards, schedule shooting days and produce call sheets and PDF reports.

**Work together without the cloud.** Share review and exchange packages as files. Collaborators
send back their notes and changes, and you decide what to accept.

**Built to protect your work.** Continuous autosave, undo and redo, recovery after a crash, and
Recently Deleted for everything you remove. Projects are ordinary folders in your Documents, so you
can back them up any way you like, and uninstalling the app never deletes them.

Requirements: Windows 10 version 1809 or later, or Windows 11, on a 64-bit (x64) PC. OpenFrame uses
the Microsoft Edge WebView2 Runtime, which is included with Windows 11 and installed on most
Windows 10 PCs. **[AI]** The optional AI assistant needs about [N] GB of free disk space for the
download and works best with [N] GB of RAM or more.

## What's new in this version

First Microsoft Store release.

## Product features (up to 20, 200 characters each)

1. Local-first: no account, no telemetry, and your projects stay on your PC
2. Idea Vault per project, plus a Global Idea Vault for ideas you reuse
3. Story board with acts, sequences, beats and scene cards, plus an outline view
4. Screenplay editor with standard formatting, drafts, locking and revisions
5. Import and export Final Draft (FDX), Fountain, PDF and DOCX screenplays
6. Script breakdown, catalog, locations, cast and crew
7. Shot lists and storyboards
8. Shooting schedule and call sheets with PDF export
9. File-based review and exchange packages for collaborators
10. Autosave, undo and redo, crash recovery and Recently Deleted
11. **[AI]** Optional on-device AI assistant. Nothing is sent to the cloud.

## Keywords (up to 7)

screenwriting, screenplay, filmmaking, storyboard, script breakdown, shooting schedule, call sheet

## Copyright and trademark info

© [year] [copyright holder]. [License statement once selected, e.g. "Licensed under the … license".]

## Additional license terms

[Leave empty to use the Standard Application License Terms, or link the project's open-source
license once selected (LICENSE-PENDING.md).]

## Developed by

[Developer or organization name]

## Screenshots (required: at least 1, recommended 4 to 8; desktop 1366 x 768 or larger, PNG, up to 50 MB, captions up to 200 characters)

Capture them at 1920 x 1080 (or 2560 x 1440) from the MSIX build, using a demo project with
invented content. Don't use real people's contact details. Keep key content in the top two-thirds,
and don't add extra logos or marketing text.

| # | Screen | Caption |
|---|---|---|
| 1 | Story board with acts, beats and scene cards | Shape your story on a board of acts, sequences, beats and scene cards. |
| 2 | Screenplay editor | Write in industry-standard screenplay format, with drafts and revisions. |
| 3 | Idea Vault | Keep every idea, image and reference together, per project or across projects. |
| 4 | Breakdown | Break scenes down into cast, props, wardrobe, locations and more. |
| 5 | Schedule / call sheet | Schedule shooting days and produce call sheets. |
| 6 | Home / project list | Your projects are folders on your PC. No account, no cloud required. |
| 7 **[AI]** | AI assistant suggestion review | The optional AI assistant runs on your PC. You review every change. |

## Store logos

- **1:1 app tile icon, 300 x 300 PNG (strongly recommended):** export
  `apps/desktop/src-tauri/icons/source.png` at 300 x 300. Without it, the Store uses the package logo.
- 2:3 poster art and 1:1 box art apply to games only.
- 16:9 super hero art (1920 x 1080, no text, no UI) is optional. It is needed only for trailers
  and featuring.

## Properties page values

- **Category:** Photo & video (alternative: Productivity)
- **Privacy policy URL:** the hosted copy of `docs/store/privacy-policy.md`
- **Website / support contact:** [URLs]
- **Product declarations:** check "This app depends on non-Microsoft drivers or NT services" =
  **No**. The rest per Partner Center defaults.
- **System requirements:** x64. Minimum memory [N] GB. **[AI]** Recommended memory [N] GB for the
  AI assistant.
