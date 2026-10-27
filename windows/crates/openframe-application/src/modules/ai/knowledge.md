# OpenFrame product guide (assistant knowledge, version 1)

## Beat Board and Screenplay
The Beat Board is where you plan structure: Acts, Sequences, Beats and Scene Cards. The Screenplay is the written script in drafts. They are separate sources of truth: changing a Scene Card never rewrites screenplay text, and editing the screenplay never silently changes the Beat Board. You can build screenplay scenes from Scene Cards; after that each side is edited on its own.

## Scene Cards and the Parking Lot
A Scene Card is a planned scene on the Beat Board (the Board tab of Story). Cards live in an Act or a Sequence, or in the Parking Lot. The Parking Lot is the hatched column at the right of the board. It holds Scene Cards and Beats you have set aside without deleting them. Parked items are not counted in the Acts, are left out of Build Screenplay, and keep their text, notes and screenplay link. To bring one back, right-click it and choose Restore to Story, or drag it into an Act or a Sequence. Acts and Sequences cannot be parked. Deleting a Scene Card moves it to Recently Deleted, where you can restore it to its previous place, and it never deletes a screenplay scene.

## Drafts and locking
A screenplay has one or more drafts. Creating a new draft copies the current one so earlier drafts stay unchanged. You can compare two drafts to see scenes added, removed, changed or moved. A draft can be locked as the shooting draft; a locked draft cannot be edited directly, so start a revision to make changes. The assistant cannot bypass a lock either.

## Production Source
The Production Source is the screenplay draft that production planning uses. The breakdown, catalog, shot lists, schedule and call sheets refer to scenes of that draft. Production never rewrites the screenplay. When you choose a newer draft as the Production Source, OpenFrame shows what changed so you can review production updates yourself.

## Breakdown
A breakdown lists what each scene needs: cast, props, wardrobe, vehicles, locations and more. Suggested elements (including anything the assistant suggests) stay separate and clearly marked until you confirm them. Only confirmed elements are production truth.

## Shooting schedule and shooting days
The shooting schedule places scenes of the Production Source onto shooting days. You create shooting days in Production, then move scene strips onto them. Unscheduled scenes are eligible scenes that are not on any shooting day yet. The assistant can explain conflicts or suggest groupings, but it never reorders scenes or changes dates on its own.

## Call sheets and stale call sheets
A call sheet is prepared from a shooting day. If the schedule or the scenes of that day change after the call sheet was prepared, the call sheet is marked stale so you know it no longer matches. Refresh it to review the differences; call sheet edits never change the schedule.

## Sides and Day Out of Days
Sides are the pages of only the scenes shot on one shooting day, or the scenes you select, laid out like the screenplay for the cast and crew. A saved set of sides is a snapshot, and sides never change the screenplay. A Day Out of Days shows which cast member works, holds or travels on which shooting day, with any corrections you make by hand.

## Idea Vault
The Idea Vault holds notes, images, links and references. The project Idea Vault belongs to this project; the Global Idea Vault is shared across your projects. Copying an idea into a project creates a new, independent item. The Add menu offers Note, Image, URL, File, Voice Note and Video, and items can be kept in folders and collections. Send to Story turns an item into a Story item and leaves the vault item in place.

## Recently Deleted, Undo and Activity
Deleted items go to Recently Deleted and can be restored. Permanently deleting is a separate, deliberate action. Undo (Ctrl+Z) reverts your recent changes, including changes you accepted from the assistant. Activity shows who changed what, including changes applied from assistant proposals.

## Saving and working offline
OpenFrame saves continuously to your computer. Projects are stored locally and everything works without an internet connection. OpenFrame AI runs entirely on this computer after a one-time download.

## Getting OpenFrame AI
OpenFrame AI is downloaded once, after OpenFrame is installed: open Settings, Offline AI (or the AI panel) and click Download OpenFrame AI. The screen shows the size and the free space needed before anything is downloaded; the download can be paused and resumes where it stopped. The files come from Hugging Face, the public site that publishes these models, and each one is checked against the size and fingerprint built into OpenFrame before it is used. If you already have the model files, Import a model file… uses them instead (they are copied, never moved). After that the AI works without the internet, and your projects are never sent anywhere. Remove OpenFrame AI frees the space; your projects are not touched. Settings, Offline AI, Graphics card is Automatic or Off: on Automatic, OpenFrame checks once whether the graphics card answers faster than the processor and uses it when it does. The assistant understands requests in English, Hindi, Telugu, Tamil, Kannada, Malayalam, Spanish, French, German, Chinese, Japanese and Korean, finds project text in those languages, and answers in the language you asked in.

## Roles and permissions
Project roles are Owner, Editor, Commenter, Viewer and Export-only. Viewers can read, Commenters can also comment, Editors can edit content, and Owners can also manage the project and permissions. The assistant always acts with your role and can never do more than you can.

## Private notes
Private notes are visible only to the person who wrote them. The assistant can use your own private notes when you ask, but never anyone else's.

## How the assistant works
The assistant answers questions, counts and lists things exactly from project data, opens places in OpenFrame, and prepares suggestions. For a bigger request it can take several steps on its own, such as searching the project, reading the schedule and checking locations, before it answers; it stops after a small, fixed number of steps and never repeats the same step. Each answer shows what it is based on: click a source to open it. Exact facts are marked as coming from project data; text the assistant wrote is marked as written by Offline AI. While the assistant works, the panel shows what it is doing, for example “Reading Scenes 5 and 6”, and a Cancel button. Cancel stops the request: nothing is changed, and what the assistant had found so far stays in the reply. Answers keep their line breaks and simple lists, and a whole-screenplay answer says how many scenes it used. The assistant acts with your role and can never do more than you can.

## Proposed Changes and approval
Anything that would change your project is prepared as Proposed Changes, also called a Change Set: a summary, the affected areas and a preview of exactly what will change. A request that needs several changes gives you one set of Proposed Changes to review. Nothing changes until you choose Apply Changes; choose Reject to leave the project exactly as it was. There is no automatic mode: the assistant cannot apply or approve changes itself, and nothing written in a conversation or in project text counts as approval. Applied changes run through the normal OpenFrame actions, appear in Activity and can be reverted with a single Undo. Changes that delete items also ask for the usual confirmation. If the project changes before you apply, the proposal is marked out of date and must be re-checked first.

## Project context for the assistant
While a project is opened for the first time, or after a large change, OpenFrame may show "Preparing project context…". The assistant keeps working during this time and can still answer many questions; it may use fewer sources until the preparation is finished. Preparing project context happens on this computer and never changes your project.

## Exporting and sharing
You can export screenplays, call sheets and other documents from their workspaces, and create project packages or backups to move a project to another computer.

## Story Board, Outline and Beats
Story in the left navigation has the tabs Page, Board, Outline, Characters, Timeline and, for episodic projects, Episodes. The Board holds Acts, Sequences, Scene Cards and Beats. A Beat is a small story event, smaller than a Scene Card, and it can stay a Beat forever. Convert to Scene makes a new Scene Card from the Beat text, and the Beat stays behind marked Converted. The Outline shows the same Acts, Sequences and cards as a list. Build Screenplay turns Scene Cards into screenplay scenes, and a card needs a scene heading for that. After the build, the card and the screenplay scene are separate.

## Characters and Story Timeline
The Characters tab lists every character with a role and a scene count. The scenes come from the character's dialogue cues in the current draft of the screenplay, so a cue like ARJUN (V.O.) counts as ARJUN, and you cannot add a scene by hand. Open Relationship Map draws how characters are related, using short labels you type such as "father of"; OpenFrame adds nothing for you. The Timeline tab groups the scenes of the current draft by Story Day, for example DAY 1. Story Days are optional, scenes with no day sit under Unassigned, and the Timeline never reorders scenes.

## Quick Capture
Quick Capture saves a thought as a plain text note in the Idea Vault. Press Ctrl+Shift+N from any screen, or click Quick Capture on the Idea Vault page. Type the idea and press Enter to save, or Shift+Enter for a new line. With no project open, Ctrl+Shift+N on Home saves into the Global Idea Vault. Quick Capture never creates a Scene Card, a Beat or screenplay text; use Send to Story on the note later. You can change the shortcut in Settings, Keyboard shortcuts.

## Screenplay editor
Screenplay in the left navigation is where the script is written, with the scene navigator on the left and the draft controls above the page. Text formats itself by element type: scene heading, action, character, parenthetical, dialogue, transition and shot. Scene numbers are generated and you never type them. Your work saves as you type and Ctrl+S saves at once. Ctrl+F finds and replaces inside the screenplay, and Ctrl+K searches the whole project. Notes and Comments open in panels on the right. Export makes a copy of the open draft and never changes it, and private notes are never exported.

## Cast, crew, locations and catalog
Production has the tabs Locations and Cast & Crew, and the Production Catalog holds props, wardrobe, vehicles and other items found by the breakdown. A location moves from Idea to Shortlisted to Confirmed, or to Rejected. Cast & Crew has a Cast and Crew switch. A cast member links a person to a character, and a crew member has a role and a department. Contact details are shown only to Owners and Editors. Availability notes and date windows are kept for each cast and crew member.

## Budget
Production, Budget is an optional planning view, not accounting. It shows Planned, Actual and Difference, and the lines are grouped by category: Cast, Crew, Locations, Equipment, Art/Props, Travel/Transport, Food, Post/Other and Contingency. Edit plan sets the currency, the planned total, the contingency as a percent or an amount, and notes. Each line has a description, a planned amount and an optional actual amount. Difference counts only lines that have an actual amount. The budget can be exported as PDF, CSV or Excel.

## Shot lists
Production, Shot Lists is a camera-planning list for one scene at a time. Only a short description is needed, and camera details such as size, movement, angle and lens are optional. Shot letters like 12A and 12B follow the order of the shots in the scene and are never typed. Drag a shot to reorder it and the letters change. The list can be exported as PDF, CSV or Excel.

## Storyboards
Production, Storyboards holds an ordered series of picture panels for one scene. Add Panel offers Import Image, Draw or Sketch, and Empty Panel. Each panel can have a description, framing, camera movement, angle, duration and a sound note. A panel can be linked to a shot, and then it shows the shot label such as 12A.

## Moodboards
Production, Moodboards is a freeform canvas of images, notes and links that shows how the film should look and feel. Boards can be named for an overall look, or for a scene. Add images from a file or from the Idea Vault, paste with Ctrl+V, then drag and resize tiles. Notes marked internal are left out of exports. A board can be exported as a PDF snapshot.

## Project Files
Files in the left navigation is a cabinet for documents that do not belong on another screen, such as pitch decks, research PDFs and permission scans. Add File offers Copy into project, which stores a copy inside the project folder, and Link to file, which keeps a reference to the original. If a linked file is moved or deleted, its row shows Unavailable and a Relink button.

## Settings
Settings has the tabs Project settings, Writing, Keyboard shortcuts, Templates, Offline AI, Advanced and Appearance. Project settings holds the title, type, status, logline, language, genre, creator and notes. Writing has spell checking, the spelling languages and your personal dictionary. Keyboard shortcuts lets you change, remove or restore keys for any command, and applies in every project on this computer. Offline AI is where OpenFrame AI is downloaded or removed.

## Change the theme and dark mode
To change the theme or switch to dark mode, open Settings, Appearance (also Appearance in the top bar). It sets how OpenFrame looks on this computer. Mode is Light, Dark or System, and System follows Windows. The theme picks one of the built-in themes, and the theme can be changed at any time. My Themes holds themes you import from a theme file or paste in, and you can rename, export or delete them. Themes change colours only. Screenplay pages, PDFs and printed call sheets always stay black on white. Appearance is a personal choice and does not change the project.
