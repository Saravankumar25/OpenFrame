> Moved here on 2026-10-27 as history. A snapshot of 2026-10-09; the code has moved on (ADR-0027 to ADR-0038). Current: [architecture/README.md](../architecture/README.md), [feature-status.md](../implementation/feature-status.md), [adr/README.md](../adr/README.md).

# OpenFrame Studio: retrospection and audit

> Written on 2026-10-09 for the founder, Saravan Kumar M K. This report explains, from scratch and in simple words, what
> OpenFrame is, how it was built, how it works, what is finished, what is not, what is risky, and what to do next. It is a
> snapshot of one day. Findings name the file and line so that anyone can check them. Re-check a finding in the code before
> acting on it, because the code moves.

## 0. How to read this report

### 0.1 Who this is for

You are new to Rust, to Tauri, to building Windows and Mac desktop apps, and to Claude Code. So every technical word is
explained the first time it appears, and Section 3 is a plain-language course in the ideas behind the project. If you
only want answers, read Section 1. If you want to understand how everything works, read Sections 2 to 7 in order. If you
want to decide what happens next, read Sections 9 to 16.

### 0.2 What each section gives you

| Section | What you get |
|---|---|
| 1. The short answer | Answers to the questions you asked, in one page each |
| 2. What OpenFrame is | The product in plain words |
| 3. Concepts from scratch | Desktop apps, Rust, Tauri, React, SQLite, local AI, installers, Git and Claude Code, each with an OpenFrame example |
| 4. How it was built | A day-by-day story from 2026-09-30 to 2026-10-09 |
| 5. How the application works | The big picture, with diagrams |
| 6. Inside the code | The modules, the patterns, the habits of the codebase |
| 7. The AI system | How the model, the tools and the approval step work |
| 8. Feature inventory | Every feature group, its state and the proof |
| 9. Quality report | Built, not built, production-ready, to test, broken, poorly made, quick fixes, improvements, design |
| 10. Completion estimate | The percentage, how it was calculated, how uncertain it is |
| 11. Risks | Ranked, with what blocks internal testing |
| 12. Builds and storage | Why the disk filled up and the rules from now on |
| 13. Documentation | What every document is, what changed, what stayed |
| 14. Claude Code and context rot | How the tools were used, what to simplify, how to keep Claude accurate |
| 15. The plan | Chapter 1 in order, testing, Chapters 2 and 3 |
| 16. Decisions | What only you can decide |
| 17 to 20. A guided tour | Six actions followed through the code, each crate, the database, all 483 operations, the screens, the tests, the decisions and the engineering documents |
| Appendices | A coverage of this audit, B glossary, C commands, D where to look, E sources, F every commit, G the five-minute version |

### 0.3 How much to trust each statement

Three labels are used when it matters.

- **Proved by code.** I read the file named next to the statement, or I counted it with a command.
- **Claimed by documents.** A plan, a log or a feature row says it. I did not re-run the test. Test results in this report
  are claims of this kind unless a line says "counted".
- **Unverified.** I could not check it. It needs a person, a Mac, a clean computer or a command that costs time or money.

No test, build or installer was run for this report. That was your rule for the investigation, and it is also the honest
limit of the findings: nothing here says "this test passes today". It says "the logs say it passed on 2026-10-09 at this
commit".

### 0.4 What this audit covered, and what it did not

The code base has about 195,000 lines of Rust and 41,000 lines of TypeScript screens (without tests). Nobody reads all of
that in one sitting, and claiming so would be false. I read the parts that decide whether the product is safe and works:
how the app starts, how every action is checked and saved, how upgrades and backups work, how the editor saves typing, how
the AI is started and kept from changing things, how the project is built and tested. I counted and searched the rest. The
exact list, with what remains, is in Appendix A. The handoff file carries the same list for the next session.

### 0.5 A correction

Earlier in this session I said "no test asserts the safety backup before a migration". That was wrong. I copied it from
a feature row that was out of date. The test `migration_fixtures.rs` (lines 234 and 354) checks that the backup file exists
for every old version. The row has been corrected. I mention it here because it teaches the most important lesson of the
audit: **a status text can lag the code, so read the code**. Three rows were stale (CORE-004, CORE-005, CORE-015).

## 1. The short answer

### 1.1 What OpenFrame contains today

- A desktop application with 12 screens (workspaces): Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call
  Sheets, Files, Recently Deleted, Activity, Notes and Tasks, Settings.
- About 195,000 lines of Rust and 41,000 lines of screens in TypeScript and React. 22 database migrations. 483
  registered operations (150 reads, 333 changes). A bundled offline AI of about 1.1 GB.
- 173 rows in the feature list: 65 Verified, 45 Reachable, 31 Built, 10 Designed, 17 Specified, 2 process rows, 3 not
  applicable. Nothing is Release-ready. (Counted with `node scripts/check-feature-status.mjs`.)
- No installer has ever been built. The Mac version has never run.

### 1.2 What is genuinely complete, incomplete or unverified

- **Complete and well proved on the development machine:** the Story Board, the screenplay editor and its formats, the
  schedule and Day Out of Days, call sheets and exports, themes, packages and backup, the AI safety path (it can only
  propose, a person applies, an interrupted apply is undone on the next open).
- **Incomplete:** visual planning has no end-to-end step; accessibility is almost untested; the AI is not measured for
  quality or speed on small laptops; the end-user licence agreement does not exist.
- **Unverified:** everything about installers, clean computers, the Mac, OneDrive-synced folders, the release build, the
  team password gate in a real build, and real writers' use.

### 1.3 Strongest and weakest engineering

Strongest: a single guarded door between screens and data, one write path that saves, makes undo steps and updates search
in one transaction, careful upgrade and backup code, bounded parsers, a clear AI approval boundary, and a large body of
tests. Weakest: no proof outside the development machine, a very large single Rust crate, test programs that waste disk,
documents that lag the code, and an AI whose usefulness is unmeasured.

### 1.4 The most serious Chapter 1 blockers

1. No installer exists, and none may be built without your word.
2. There is no macOS AI engine, no disk image setting, and no Mac run at all.
3. Typed text may be lost if the window closes within 0.4 seconds of typing (found by reading; not yet reproduced).
4. Projects inside OneDrive-synced folders are not protected, and Windows 11 often syncs Documents.
5. The AI is unmeasured, English-centred, and slow on a processor-only machine (20 to 30 seconds per answer in the logs).
6. The release build is never driven by the end-to-end suite, so the team password gate and the release-only paths have
   only unit tests.

### 1.5 The completion estimate

Chapter 1: about 80 percent of the weighted feature list is built, about 50 percent is proved at feature level on the
development machine, and about 0 of 6 internal-release gates are met. A single blended figure is about 60 percent, give or
take 10 points. Section 10 shows the arithmetic.

### 1.6 Major contradictions found

Between documents and code (the stale rows, the README table, the signed-dmg sentence), between documents and your vision
("filmmakers" versus writers, screenwriters and production teams; "AI optional" versus "AI bundled"), and inside the tooling
(claude-mem disabled in the instructions, enabled in your settings). Section 13.5 lists them all.

### 1.7 What happened to the documents

They were organised by your three chapters. New: a vision page, three chapter plans, a testing strategy and pilot plan, a
one-page architecture, an archive index, a handoff page and this report. Rewritten: the documentation map and
`CLAUDE.md`. Amended: the build plan, owner decisions, the storage policy, the root README. Nothing was moved or deleted.
Section 13 explains why.

### 1.8 The next actions, in order

1. Decide the open questions in Section 16 (team password, default folder, wording, engine).
2. Write the failing end-to-end step for "type then close", then fix it.
3. Write the missing delete and locate tests; add the cloud-folder warning.
4. Lower the debug setting in the build profile and review the dead worktrees (saves about 22 GB).
5. Say the word, and build the first internal Windows installer; install it on a clean machine.
6. Bring the Mac build up on your Mac.
7. Run the 2 to 3 day writer pilot.

## 2. What OpenFrame is, in plain words

### 2.1 The product

OpenFrame Studio is a program that a writer, a screenwriter or a production team installs on a Windows 11 or Apple
silicon Mac computer. It keeps everything on that computer. There is no account and no cloud.

A person can:

- Collect ideas (notes, pictures, web links, voice notes) in the Idea Vault.
- Arrange a story as acts, sequences, beats and scene cards on the Story Board.
- Write the screenplay in an editor that follows the rules of screenplay format: scene headings, action, character
  names, dialogue, parentheticals, transitions. It paginates like real script pages.
- Keep drafts, compare them, review them in rounds, lock a "shooting draft" and track revisions.
- Break a script down: mark props, wardrobe, vehicles and so on in every scene, and keep a catalog of locations, cast and
  crew.
- Plan visually with moodboards, storyboards and shot lists.
- Plan the shoot with a stripboard schedule and a Day Out of Days chart.
- Produce call sheets, "sides" (the script pages for a day), reports and a light budget.
- Import and export Final Draft, Fountain, Word, PDF and text files; make backups and review packages.
- Ask an offline AI assistant questions about the project and let it propose changes, which the person approves.

### 2.2 Who it is for

Your words: writers, screenwriters and production teams, all using the same application. The code was written with a film
production tool in mind (the code and the older documents say "filmmakers"). The features for writers (Idea Vault, Story
Board, screenplay editor, drafts, formats) and for production teams (breakdown, schedule, call sheets, budget) live in one
program. That fits your statement that there are no separate editions.

### 2.3 What it is not

It is not a cloud service, a team chat or a live co-writing tool. Live network collaboration was removed on 2026-09-30
(ADR-0007). People share work by exporting a package file and sending it any way they like.

### 2.4 The three chapters

1. Finish, test and validate the application and give writers an internal `.exe` and `.dmg` to try for 2 to 3 days.
2. Build the public website on Cloudflare Pages.
3. Package for the Microsoft Store and the Mac App Store with a 7-day free trial and paid activation.

Only Chapter 1 is in progress.

### 2.5 One picture of the whole thing

```text
   A writer on a Windows or Mac laptop
                  |
        OpenFrame Studio (one program)
                  |
   +--------------+---------------+--------------------+
   |              |               |                    |
 Screens      Rust brain      Project files        Offline AI
 (what you    (all rules,     (a folder per        (a small language
  see and      saving,         project with a       model, run by a
  click)       exports)        database inside)     helper program)
```

Everything is inside one installed program, stored in one folder per project, with no internet needed.

## 3. Concepts from scratch

Read this section once. After it, the rest of the report reads easily. Each idea has a short explanation and an example
from OpenFrame.

### 3.1 What a desktop app is

A website lives on someone else's computer (a server) and you look at it through a browser. A **desktop app** is a program
that is installed on your own computer and runs there. OpenFrame is a desktop app.

Why this matters for OpenFrame:

- **Your files stay with you.** A writer's script is a folder on their disk, not a record on a server. That is the whole
  promise of the product: it works with no internet, and nobody else can read it.
- **You are responsible for everything the server used to do.** Saving, backups, upgrades, crash recovery and licensing
  all happen inside the program, on every customer's computer, in conditions you cannot see. A writer's laptop may have
  OneDrive syncing the Documents folder, a strict antivirus, 8 GB of memory and a nearly full disk.
- **An installer is part of the product.** A person double-clicks a file, the program is copied to their computer, and
  later an upgrade must replace it without losing their work.

Some words you will meet:

| Word | Meaning | OpenFrame example |
|---|---|---|
| Process | A running program | `openframe-desktop.exe`; also the two AI helper programs |
| Window | What the process shows | The OpenFrame window, 1440 by 900 at start |
| Install folder | Where the program files go | Windows: your user's program folder (per-user install) |
| User data folder | Where the program keeps settings | `%LOCALAPPDATA%\OpenFrame` (settings, logs) |
| Documents | Where people expect their work | `Documents\OpenFrame\Projects\Title.openframe` |
| Crash | The program dies without warning | A power cut while saving; the app must reopen safely |

### 3.2 Why OpenFrame uses two programming languages

OpenFrame is written in **TypeScript** (with the library **React**) for what you see and click, and in **Rust** for
everything that must be right: saving, rules, files, the AI helper.

- **TypeScript** is JavaScript with checks. Browsers run JavaScript, so screens built with it can be made quickly. React
  is a library that turns "this is the data" into "this is the screen" and updates the screen when the data changes.
- **Rust** is a language for programs that must not crash or corrupt data. It refuses to compile code that could misuse
  memory. It is fast. It is harder to learn than TypeScript.
- **Why not one language?** A screen written in TypeScript is easy to change but is allowed to do anything inside the
  browser engine. A brain written in Rust is strict. OpenFrame puts the screens in a cage (they cannot touch files or the
  network) and lets only the Rust brain do real work. That is the main safety idea of the whole design.

### 3.3 Rust from zero

#### 3.3.1 Cargo and crates

- **Cargo** is Rust's build tool and package manager. You type `cargo build` or `cargo test` and it does the work.
- A **crate** is one Rust package: a folder with a `Cargo.toml` file (its name and its needs) and source files in `src/`.
- A **workspace** is a group of crates built together. OpenFrame's root `Cargo.toml` lists 12 members: the desktop shell and
  11 crates under `crates/`.
- `Cargo.lock` records the exact version of every package used. OpenFrame uses 632 packages (counted in `Cargo.lock`),
  most of them small libraries written by other people (for SQLite, ZIP files, PDFs, fonts, encryption and so on).

OpenFrame's crates, in plain words:

| Crate | Job |
|---|---|
| `openframe-domain` | The shared words: error types, roles, identities |
| `openframe-persistence` | Talks to the SQLite database: connection rules, migrations, undo capture |
| `openframe-project-format` | The project folder layout and the "only one program may write" lock |
| `openframe-security` | Checks for paths, ZIP files, links, and hiding private data in logs |
| `openframe-platform` | Everything that differs between Windows and Mac |
| `openframe-entitlement` | Trial, licence and team password rules |
| `openframe-import-export` | Final Draft, Fountain, Word, PDF, text; page layout; fonts |
| `openframe-search` | Search index (words and meaning) |
| `openframe-ai` | Starts and checks the AI helper programs |
| `openframe-application` | The brain: 21 modules for every feature |
| `openframe-test-support` | Helpers used only by tests |

The brain crate `openframe-application` is by far the largest: about 150,000 of the 195,000 lines of Rust (counted).

#### 3.3.2 Compiling

Rust turns your source text into a machine program by **compiling**. Compiling is slow for large programs because the
compiler checks and optimises a lot. Key words:

- **Debug build.** Compiles faster, runs slower, contains information that helps a developer find mistakes. Used while
  developing and by the test suite.
- **Release build.** Compiles slower, runs faster, is smaller. This is what customers get. OpenFrame's release profile
  uses "thin LTO", which means extra optimisation across crates.
- **Profile.** A named set of compiler settings in `Cargo.toml`. OpenFrame has `dev`, `release` and a special `perf`.
- **`target/` folder.** Where all compiled output goes. It can become huge (see Section 12).
- **Linker.** The final tool that joins compiled pieces into one `.exe`. OpenFrame uses the fast `rust-lld` linker for
  development and the Microsoft linker for releases (the Microsoft one is required for some security flags).
- **sccache.** A helper that remembers compiled pieces so that a second compile of the same code is faster.
- **Incremental compile.** Rust remembers what it compiled last time so that small edits compile faster.

Why a small change can still be slow: a change inside `openframe-application` means every test program that includes it
must be linked again. OpenFrame has 46 separate test programs for that crate alone.

#### 3.3.3 The tools that check Rust code

| Command | What it does | OpenFrame use |
|---|---|---|
| `cargo fmt` | Makes formatting uniform | Checked before every commit |
| `cargo clippy` | A strict code reviewer for common mistakes | Run with "warnings are errors" |
| `cargo test` | Runs the tests | 1,200 or more test functions (counted); the log says 2,013 passed |
| `cargo deny` | Checks licences and known security problems of packages | Run in CI and locally |
| `cargo audit` | Checks packages against a security database | Same |

#### 3.3.4 How Rust handles errors

Rust has no hidden exceptions. A function that can fail returns a `Result`, which is either "ok with a value" or "error".
OpenFrame uses its own error type `AppError` with a stable code (for example `storage.disk_full`) and a message written
for a human, such as "The drive is full, so OpenFrame could not upgrade this project. Free up some space and try again.
Nothing was changed." The code `unwrap()` means "crash if this is an error". In OpenFrame's production code `unwrap` and
`expect` are rare (about 60 lines outside tests, counted by a script) because a crash is the worst outcome.

#### 3.3.5 `unsafe` Rust

`unsafe` marks code where the developer promises the compiler that something is safe. It is needed to talk to Windows or
macOS directly. OpenFrame has 73 uses, nearly all in `openframe-platform` (calls to the operating system) and in the Mac
StoreKit bridge. They are listed and reviewed by a rule in `scripts/architecture-lint.mjs`.

### 3.4 Tauri from zero

#### 3.4.1 What Tauri is

Tauri is a toolkit for building desktop apps from a web interface plus a Rust program. It gives you three things:

1. A **window** that contains a **webview**: a small browser inside your app. On Windows the webview is Microsoft Edge
   WebView2 (already part of Windows 11). On the Mac it is WKWebView (part of macOS).
2. A **bridge** that lets the web page call Rust functions.
3. A **bundler** that builds installers (`.exe` through NSIS, `.msi`, and on a Mac the `.app` and `.dmg`).

The alternative is Electron, which carries a whole copy of Chrome and Node.js in every app (large, memory hungry). Tauri
uses the browser already on the computer, so the program is small. The choice is recorded in ADR-0001.

#### 3.4.2 The window, the page and the brain

```text
 +--------------------------- OpenFrame process ---------------------------+
 |   Window (Tauri)                                                        |
 |    +---------------------------+     bridge      +------------------+  |
 |    |  Webview: the React screens| <------------> |  Rust brain      |  |
 |    |  (no files, no network)    |   of_invoke    |  (all real work) |  |
 |    +---------------------------+                 +------------------+  |
 +-------------------------------------------------------------------------+
```

#### 3.4.3 A Tauri command

A **command** is a Rust function the web page may call. Most apps have dozens. OpenFrame deliberately has four:

- `of_invoke(op, args)`: the one door for all project work. `op` is a name such as `"story.create_act"`.
- `of_open_asset`: opens a project file by its id, never by a path from the page.
- `of_reveal_path`: shows a known folder in the file manager.
- `of_open_url`: opens a web link in the user's browser.

They are registered in `apps/desktop/src-tauri/src/lib.rs` line 508. The design is recorded in ADR-0004. The advantage:
the Rust side has one place where every request enters, so checking it is easy, and the page cannot ask for anything that
is not on the list of 483 operations.

#### 3.4.4 Capabilities and the content security policy

Tauri 2 requires a list of **capabilities**: what the web page may do. OpenFrame's list
(`apps/desktop/src-tauri/capabilities/default.json`) allows listening to events, setting the window title, asking whether
the window is full screen, reading the app version and opening file dialogs. It does not allow reading files, running
programs or making web requests.

The **content security policy (CSP)** is a rule for the page itself (in `tauri.conf.json`): scripts only from the app, no
frames, no forms, connections only to the bridge. It stops a hostile script from phoning home even if one slipped in.

#### 3.4.5 `tauri.conf.json`

The settings file for the shell. In OpenFrame it names the app (`OpenFrame Studio`), the window size, the content security
policy, the installer targets (`nsis` and `msi` only) and the installer description. A second file,
`tauri.bundled-ai.conf.json`, adds the 1.1 GB AI folder as a resource, so it is copied next to the program.

#### 3.4.6 Application bundles

- **Windows `.exe` installer (NSIS):** one file that installs the program for the current user without administrator rights.
- **Windows `.msi`:** the Windows Installer format; also configured.
- **Windows `.msix`:** the Microsoft Store package format. Not used for the internal build.
- **Mac `.app`:** a folder that looks like one file, holding the program and its resources (`Contents/Resources`).
- **Mac `.dmg`:** a disk image that contains the `.app` so a person can drag it to Applications.
- **Mac `.pkg`:** an installer package for the Mac App Store.

### 3.5 React and TypeScript from zero

- A **component** is a small piece of screen as a function: "show this list of cards". Components contain other components.
- **State** is data that the screen remembers. OpenFrame keeps only view state in the browser (which workspace is open,
  whether the AI panel is open) using a small library named zustand. The project data never lives there.
- **React Query** is a cache for answers from Rust. A screen asks `useOp("story.board", args, ["story_card"])` and shows the
  result. When Rust says "the table `story_card` changed", every screen that depends on it asks again. There is no polling.
- **ProseMirror** is a library for rich text editors. The screenplay editor is built on it, with rules that decide what
  Enter, Tab and Shift+Tab do for each element type.
- **Vite** builds the screens into plain files the app can load. **Vitest** runs screen tests.
- **ts-rs** generates 809 TypeScript type files from the Rust data types, so the two sides agree on the shape of every
  message. If Rust changes a type and the screens are not updated, the build fails.

### 3.6 SQLite from zero

SQLite is a database stored in one file. There is no server. Every OpenFrame project has `project.sqlite`.

Key ideas:

- **Table and row.** A table is a grid; a row is one record. The project database has 81 tables (counted in the
  migrations), for acts, cards, scenes, screenplay elements, breakdown elements, schedule strips, call sheets and so on.
- **Transaction.** A group of changes that happen all together or not at all. If the power fails halfway, the database
  returns to the state before the transaction. OpenFrame wraps every change in one.
- **WAL mode (write-ahead log).** SQLite writes changes to a side file (`-wal`) first, which makes it fast and crash-safe,
  but it also means a project has extra files while it is open. OpenFrame sets `synchronous=FULL`, so a committed change
  survives a power cut.
- **Migration.** A numbered script that changes the database layout when the app gets a new feature. OpenFrame has 16 for
  projects, 4 for the Global Idea Vault and 2 for the app's own settings. Rule: a shipped migration is never edited.
- **Foreign key.** A link between tables that the database enforces.
- **FTS5.** SQLite's built-in full-text search. OpenFrame uses it for the search box and for AI keyword retrieval.
- **Trigger.** A small program inside the database that runs on every change. OpenFrame uses triggers to record every
  change so that undo and redo are generic.
- **Integrity check.** A built-in test that the file is not damaged. OpenFrame runs it every time a project opens and
  refuses to open a damaged file without touching it.
- **Backup.** SQLite can copy a database safely while it is open. OpenFrame uses this for safety backups.

OpenFrame uses the Rust library `rusqlite` with SQLite built into the program (ADR-0002), so the customer installs nothing.

### 3.7 Local AI from zero

#### 3.7.1 The words

| Word | Plain meaning | In OpenFrame |
|---|---|---|
| Language model | A program that predicts the next piece of text | Google Gemma 3 1B Instruct |
| Parameters, "1B" | The number of learned numbers inside it: one billion | A small model; chat services use models 100 times larger |
| Weights | The file holding those numbers | `model.gguf`, 1,069,306,368 bytes |
| GGUF | A file format for model weights that llama.cpp reads | Both models are GGUF |
| Quantization, "Q8_0" | Storing each number with 8 bits to make the file smaller | The model is Q8_0 |
| llama.cpp | A fast open program that runs GGUF models on a normal computer | OpenFrame bundles version b11259 for Windows |
| Token | A piece of a word (about three quarters of a word) | The model reads and writes tokens |
| Context window | How many tokens the model can look at at once | 8,192 tokens for chat |
| Embedding | A list of numbers that stands for the meaning of a text | BGE small English v1.5, 384 numbers per text |
| Vector search | Finding texts whose embeddings are close in meaning | `sqlite-vec`, exact comparison |
| RAG | Retrieval-augmented generation: find relevant project text first, then let the model answer from it | The assistant's retrieval step |
| Tool | A named action the model may ask the program to run | 253 tools |
| Agent | A loop: model picks a tool, program runs it, model sees the result, repeat | At most 12 steps, 8 by default |
| Hallucination | The model states something that is not true | Why facts must come from tools |
| Prompt injection | Text inside the data that tries to give the model orders | Why project text is marked as data |

#### 3.7.2 How the pieces fit

1. OpenFrame starts two copies of `llama-server.exe` as helper processes: one for chat (Gemma) and one for embeddings
   (BGE). They listen only on `127.0.0.1` (this computer), on a random port, protected by a key made at launch.
2. When the writer asks something, Rust gathers relevant project text (search plus meaning search), picks a few tools that
   fit the request, and asks the model for one JSON object: "which tool, with which arguments".
3. Rust runs the tool, adds the result to the conversation, and asks again, up to 12 times.
4. If the model wants to change something, it calls a `propose_*` tool. That only prepares a **Change Set**: a reviewable
   list of changes. Nothing is written. The writer clicks **Apply Changes**, and only then does Rust apply them as one
   undoable step.

#### 3.7.3 Why a 1B model needs care

A one-billion-parameter model is small. It follows instructions loosely, forgets, and invents. OpenFrame handles that with
walls instead of trust: facts come from tools, the output must be valid JSON (the model's output is constrained by a
grammar), the number of steps is capped, repeated identical calls end the loop, project text is marked as data, and the
model can never write. The cost is that it may choose the wrong tool or misunderstand a request, and then the writer sees
a poor answer or a clarification. The logs record exactly this: the model proposed the same task twice, answered "Open
Scene 2." with a question, and sent a screenplay summary to the product guide.

### 3.8 Installers, signing and the warnings people see

| Word | Meaning | Effect on the internal build |
|---|---|---|
| Code signing | A digital signature from a certificate bought from an authority; proves who made the file | Not used yet, so Windows shows a warning |
| SmartScreen | Windows' reputation filter for downloaded programs | "Windows protected your PC": More info, Run anyway |
| Notarization | Apple scans a signed app and records it | Not used yet |
| Gatekeeper | macOS' check of downloaded apps | "Cannot be opened because Apple cannot check it": Open Anyway in Settings |
| WebView2 runtime | The browser engine Windows apps use | Preinstalled on Windows 11; the installer downloads it if missing |
| Visual C++ runtime | Microsoft libraries the AI engine needs | Shipped next to the engine (decided 2026-10-08) |
| App-local deployment | Putting needed libraries in the app's own folder | Keeps the AI working on a clean PC |
| Sandbox | A fence the Mac App Store puts around apps | Needed for the store version, not the `.dmg` |

The warnings are **installation restrictions, not defects**. The test plan tells testers what to click.

### 3.9 Git from zero

| Word | Meaning | OpenFrame example |
|---|---|---|
| Repository | The folder with the full history of changes | `C:\OpenFrame\.git` |
| Commit | A saved snapshot with a message | 215 commits on `main` |
| Branch | A separate line of work | `docs/chapter-reset` (this restructure) |
| Merge | Joining one branch into another | `session-2` into `main` on 2026-10-09 |
| Remote | A copy on a server such as GitHub | `origin`; nothing has been pushed since 2026-10-06 |
| Tag | A named point in history | `baseline/pre-masterplan-2026-10` |
| Worktree | A second working folder for another branch, sharing one history | Used by agents; the cause of much waste |
| Squash | Replace many commits by one | `main` starts with two squashed commits on 2026-09-30 |
| Junction | A Windows shortcut folder that looks like the real one | A worktree's `node_modules` points to the main one |

Why a **worktree** exists: you can have two branches checked out at once without copying the history. Agents used them so
that several of them could edit code at the same time without overwriting each other. The price: each worktree also needs
its own build folder, and mistakes with them deleted files once (Section 12).

### 3.10 Claude Code from zero

Claude Code is a program in which an AI model (Claude) reads your files, runs commands and edits code while you talk to it.
The words you asked about:

| Word | Plain meaning | In OpenFrame |
|---|---|---|
| Session | One conversation from start to end | "Session 1", "session 2" are groups of work |
| Context window | How much Claude can hold in mind at once; it is finite | Everything read in a session uses it up |
| Compaction | When the window fills, old parts are summarised | Details can be lost in a summary |
| Context rot | Quality drops as the context gets long, stale or contradictory | The reason for the handoff file and short `CLAUDE.md` |
| `CLAUDE.md` | A file Claude reads at the start of every session in a project | 14.8 KB before today, 153 lines now |
| Memory | Small notes Claude keeps for you across sessions | 10 files in `~/.claude/projects/C--OpenFrame/memory/` |
| Skill | A saved how-to Claude can load when a task matches | `superpowers`, `ecc`, `caveman`, dozens more |
| Plugin | A bundle of skills, agents, commands and hooks | 11 enabled in your settings |
| Subagent | A helper Claude that works on a separate task in its own context | 33 GSD agents and the ECC reviewers |
| Hook | A script that runs automatically before or after a tool use | graphify guard, GateGuard fact check, GSD guards |
| MCP server | A connection to an outside service | GitHub, Figma, Notion, Vercel and others are listed |
| Permission mode | What Claude may do without asking | Local commits yes, push no |
| Worktree isolation | A subagent gets its own worktree | Created the `.claude/worktrees` folders |
| graphify | A knowledge graph of the code | `graphify-out/`, 22,062 nodes |

Two ideas matter most for you.

**Instructions are loaded every time.** `CLAUDE.md` and memory are read at the start of every session and cost context
whether or not they matter to the task. Long, old or contradictory instructions make Claude less accurate. That is why
this restructure made `CLAUDE.md` short and moved history out.

**Subagents multiply cost.** Each subagent loads its own context, may run builds, and may create a worktree. Eight agents
building at once on a 16 GB laptop produced the storage and memory trouble of Section 12.

### 3.11 Testing words

| Word | Meaning | OpenFrame example |
|---|---|---|
| Unit test | Tests one small function | `theme.rs` contrast checks |
| Integration test | Tests many parts together through the real entry point | `core_pipeline.rs::permissions_are_enforced_before_any_mutation` |
| End-to-end (E2E) test | Drives the real program like a user | `01-journey.test.mjs` |
| Fixture | Prepared sample data for a test | The "Night Bus" demo project; 13 Final Draft files |
| Golden file | A saved correct output that new output is compared with | PDF layout goldens |
| Snapshot or visual baseline | Saved screenshots to compare | 15 baseline screens |
| Flaky test | Sometimes passes, sometimes fails without a code change | `forced_kill` timing under load |
| Soak test | Runs for a long time to find slow leaks | `soak.rs` (ignored by default) |
| Property test | Checks a rule against many generated cases | Undo then redo gives the same data |
| Smoke test | A very short "does it start" test | Missing for the release build |
| Evidence | A saved result you can show | `docs/release/evidence/` |

A green test run proves the tests passed. It does not prove the product is ready for customers.

### 3.12 Reading real code: four annotated examples

You do not need to write Rust to follow these. Read the comments after each block.

#### Example 1: a Rust function that guards the door (`registry.rs`, line 524)

```rust
fn check_arg_shape(v: &Value, depth: usize) -> AppResult<()> {
    let too_big = || {
        AppError::invalid_input("That request is too large.").with_detail("argument shape limit")
    };
    if depth > MAX_ARG_DEPTH {
        return Err(too_big());
    }
    match v {
        Value::Array(items) => {
            if items.len() > MAX_ARG_ARRAY_LEN {
                return Err(too_big());
            }
            items.iter().try_for_each(|i| check_arg_shape(i, depth + 1))
        }
        Value::Object(map) => {
            if map.len() > 1_000 {
                return Err(too_big());
            }
            map.values().try_for_each(|i| check_arg_shape(i, depth + 1))
        }
        _ => Ok(()),
    }
}
```

What it says, line by line:

- `fn check_arg_shape(v: &Value, depth: usize) -> AppResult<()>`: a function named `check_arg_shape`. It takes a borrowed
  JSON value `v` (the `&` means "look but do not take") and a number `depth`. It returns a `Result`: either `Ok(())` (all
  fine, nothing to return) or an `Err` with an `AppError`.
- `let too_big = || { ... }`: a small named recipe for building the error, so it is written once.
- `if depth > MAX_ARG_DEPTH { return Err(too_big()); }`: JSON can nest like boxes in boxes. More than 32 levels deep is
  refused. This stops an attack that tries to crash the program with a deeply nested message.
- `match v { ... }`: look at what kind of JSON value this is. An array (list), an object (record with keys), or anything else.
- For a list, refuse more than 50,000 items; otherwise check every item one level deeper (`try_for_each` stops at the first
  error).
- For an object, refuse more than 1,000 keys; otherwise check every value one level deeper.
- `_ => Ok(())`: numbers, text, true and false are fine.

Why it matters: this runs for every request, before any feature code. It is a tiny function that closes a whole category of
problem. This is the style of the project: small, strict, named after what it protects.

#### Example 2: a Tauri command that opens a web link (`lib.rs`, line 205)

```rust
#[tauri::command]
async fn of_open_url(url: String) -> Result<(), AppError> {
    let url = openframe_security::web_link(&url)
        .ok_or_else(|| AppError::invalid_input("Only web links (http or https) can be opened."))?;
    openframe_platform::shell::open_url(url).map_err(|e| {
        AppError::new("internal.open_failed", "OpenFrame couldn't open that link.")
            .with_detail(e.to_string())
    })
}
```

- `#[tauri::command]` is a label that tells Tauri "the web page may call this function".
- `async fn ... (url: String)`: the page passes a text. The function may wait without freezing the window.
- `openframe_security::web_link(&url)`: a checker that accepts only `http` or `https` links up to 4,096 characters. A link such
  as `file:///C:/...` or `javascript:...` is refused. `.ok_or_else(...)` turns "nothing" into an error with a friendly
  message; the `?` at the end means "if this failed, stop and return the error".
- `openframe_platform::shell::open_url(url)`: the platform crate, the only place allowed to ask the operating system to open a
  browser. If it fails, the user sees "OpenFrame couldn't open that link." and the technical reason is kept for support.

Notice what is not there: the page never gets to open anything it likes. Even this small door checks its input and uses the
platform crate.

#### Example 3: the screen side of the same door (`ipc/client.ts`)

```ts
export async function call<T = unknown>(op: string, args: object = {}): Promise<T> {
  try {
    return await invoke<T>("of_invoke", { op, args });
  } catch (e) {
    throw toOpError(op, e);
  }
}
```

- `export async function call<T>(op: string, args: object = {})`: a function other files can use. `T` is "the type of the
  answer", chosen by the caller. `args` defaults to an empty object.
- `invoke("of_invoke", { op, args })`: Tauri's bridge to the Rust function of the same name (Example 2's cousin).
- `try ... catch`: if Rust returned an error, turn it into an `OpError` (a class with a `code`, a message, a detail and a
  `retryable` flag) so that every screen handles errors the same way: by the code, never by the message text.

Every one of the 483 operations is reached through these seven lines.

#### Example 4: the permission table (`openframe-domain/src/auth.rs`)

```rust
Role::Viewer => matches!(cap, C::View | C::UseAi),
Role::ExportOnly => matches!(cap, C::View | C::Export | C::CreatePackage),
```

- A `Role` is one of five words. `matches!(cap, A | B)` asks "is this capability one of A or B?".
- A Viewer may only View and use the AI to ask questions. An Export-only person may View, Export and create packages.
- The whole security model of the product is this table of five rows plus the rule that every change goes through
  `Store::mutate`, which asks the table first.

The lesson: the code is built from small, readable rules. When you read a new file, look for the rule it enforces.

## 4. How the project was built, step by step

### 4.1 What can and cannot be known

This story is rebuilt from four kinds of evidence: the Git history, the progress log, the decision records (ADRs) and
the plan documents. What is solid:

- **Dates and commit messages** on `main` (215 commits) and on the old branch `master` (80 earlier commits).
- **The progress log**, which every session wrote at its end, and the evidence folders.
- **Memory files**, which record rules you gave Claude and lessons from sessions.

What is not known, and is not guessed here:

- **The first two days are compressed.** The first commits on `main` are huge snapshots: `25d27a6` (2026-09-30) with the
  whole first application, and `f9d4660` (security review fixes). The fine-grained history of the first day lives on
  `master` and on 31 old `worktree-agent-*` branches that all end on 2026-09-30. They show eight or more agents working in
  parallel that day, each on a "chapter" of the user manual (for example "WIP checkpoint: ch5 audit fixes", "eval harness
  (BLACK RAIN fixture, retrieval)"), but not who asked for what.
- **The conversations themselves** are not in the repository. Why a choice was made is known only where an ADR or a log
  entry wrote it down.
- **The Git author** on 212 of 215 commits is "Manjunath", the name set in the Git configuration of this computer. Claude's
  commits were made under that name. Three commits carry your name.
- **The project lived at another path first.** The leftover worktree folders have `.git` files that point to
  `C:/dev/OpenFrame`, which no longer exists. The project was moved to `C:\OpenFrame` at some point. When is not recorded.

### 4.2 The timeline at a glance

| Date | Commits on main | What happened |
|---|---|---|
| 2026-09-30 | 2 (plus 80 on `master`) | First full application written; specifications imported; first security review and its fixes |
| 2026-10-01 | 1 | Offline AI merged: Gemma agent, hybrid retrieval, toolbox, Change Sets |
| 2026-10-02 | 0 | First "production and store readiness" audit written (`docs/audits/`) |
| 2026-10-03 | 0 | Manual-versus-repository audit; a 4,900-line completion plan written |
| 2026-10-05 | 0 (not committed) | Documentation restructured; build plan with waves and milestones; governance scripts; tools installed; memory ran out of the machine; first parallel agent round |
| 2026-10-06 | 13 | Foundation session: 13 failing tests fixed in the product, platform and entitlement crates, Mac trial design, plan renamed to plain names, repository made private, local-only rule set |
| 2026-10-07 | 99 | Session 2 begins: parallel agents in worktrees for areas 9 to 16, themes, trial screens, three new ADRs, the 273 GB incident and its cleanup |
| 2026-10-08 | 89 | Owner decisions (goal is the internal builds, AI bundled), build queue, schedule, call sheets, bundled AI, the first full test run on the joined code fails 23 of 30 spec files and is fixed |
| 2026-10-09 | 11 | Final fixes, evidence, 79 plan boxes ticked, session 2 merged into `main`, 37 branches deleted, this retrospection |

The shape: a fast first build, four days of planning and governance, then three days with 200 commits.

### 4.3 2026-09-30 to 2026-10-01: the first build

**What was done.** A complete first application was produced: the Tauri shell, the Rust crates, the React screens, the
database layout, import and export, and a long list of specification documents placed in the repository root (a product
requirements document, a functional specification of 5,800 lines, a UX specification of 4,500 lines, domain and data,
security, AI, import and export, offline collaboration). Eight or more agents then audited the work chapter by chapter
against the user manual and fixed gaps. A security review found 6 high and 10 medium problems and they were fixed the same
day (`f9d4660`; the report is `docs/engineering/SECURITY_REVIEW_2026-09-30.md`). On 2026-10-01 the offline AI was added:
a Gemma agent, search by meaning, a toolbox of actions, and the rule that only a human applies changes.

**Why the architecture looks the way it does.** The first decisions were written down as ADRs that day:

- **ADR-0001, Tauri v2 + React + Rust.** The product needs a small installer, low memory when idle, a strict boundary
  between untrusted screens and trusted code, and the ability to host a local AI helper later. Electron would have been
  larger. A fully native UI would have cost far more.
- **ADR-0002, rusqlite over a heavier database library.** SQLite is built into the program, supports full-text search and
  safe online backups, and needs no server.
- **ADR-0003, a project is a folder** named `Title.openframe` with a database inside and one writer at a time. A writer
  can copy, back up or delete it like any folder.
- **ADR-0004, one door.** The screens may call exactly four commands; all project work goes through `of_invoke`, which
  looks the name up in a registry. This is what makes the security story short.
- **ADR-0005, generic undo.** Database triggers record every change, so every feature gets undo and redo without writing
  its own.
- **ADR-0006 and ADR-0013, the AI.** Local only; a managed llama.cpp helper on `127.0.0.1`; first three Qwen model sizes
  chosen by hardware, then (the same day) one model, Gemma 3 1B, plus a small embedding model, because the owner asked for
  "no model choice complexity at all".
- **ADR-0007, live network collaboration removed.** The owner decided that sharing is by files (packages), not a live
  connection. This removed a large risk and a large amount of work.

**What the first build left behind.** A working base with strong foundations and the usual first-version gaps: tests that
were agent-written and partly broken, documents in the repository root, and a long gap list. The 2026-10-02 audit listed
the gaps for a store release.

### 4.4 2026-10-02 to 2026-10-05: audits, plans and a documentation overhaul

Three planning documents followed, each longer than the last:

1. **The production and store readiness audit** (2026-10-02, 1,857 lines): what stops a store release.
2. **The manual-versus-repository audit** (2026-10-03, 341 lines): the user manual describes features the code does not
   have, and the reverse.
3. **The end-to-end completion plan** (2026-10-03, 4,904 lines, now in `docs/archive/`). It introduced the six states
   (Specified, Designed, Built, Reachable, Verified, Release-ready), a "hard definition of done" for every feature, seven
   parallel lanes A to G, and the rule "freeze the architecture before adding more feature code".

On 2026-10-05 these became working structure: the 13 specification files moved into `docs/specs/`, 57 live-collaboration
sections were marked superseded, the decision register (`docs/adr/README.md`) was created, and a build plan was written
with "waves" and milestones named M0 to M16 (that is where codes such as "M1.04" come from). Governance scripts were added:
an architecture lint (`scripts/architecture-lint.mjs`), a migration checker, a feature status checker and a link checker.
The tooling was installed the same day: the `superpowers`, `caveman`, `agent-skills` and `security-guidance` plugins.

The first parallel agent round (8 agents) ran on 2026-10-05 and had to be stopped: the computer was out of memory. A Windows
background service, the AppX Deployment Service, held 38.6 GB and 159,000 handles, and every build failed with "memory
allocation failed". The log records the owner being asked to restart Windows.

### 4.5 2026-10-06: the foundation session

A long session (13 commits, a very large uncommitted tree) that changed three things.

**It fixed what the first build got wrong.** Thirteen tests that agents had written were failing, and each was fixed in the
product, not by skipping the test. Real defects were found this way:

- Making a backup did not create the recovery folder and left a half-written file when the final rename failed.
- Writing a package deleted the old file before the new one was in place and overwrote read-only files silently.
- SQLite quietly opened a read-only copy under a file lock, so a project opened and then failed on the first edit.
- Paths longer than 260 characters were refused.
- The AI index rebuild never ended after its project closed, and closing a project could freeze for 20 seconds.
- Reading hostile PDF, Fountain and text files took 95 seconds or more (quadratic code).
- A ZIP bomb could expand without the declared size being checked.

**It added two crates.** `openframe-platform` (everything that differs between Windows and Mac, with a lint rule that
forbids operating-system code anywhere else) and `openframe-entitlement` (the trial, licence and later the team password).
The Mac App Store trial design and a licence server (`services/licence-server`) were also built that day. All Mac code is
unverified: no Mac has run it.

**It reset the rules.** The plan was rewritten with plain names (no more "M1.04"). The repository was made private. Branch
protection was attempted and refused by GitHub (it needs a paid plan). After a session pushed several fix-up commits to
an open pull request while the automatic checks were red, you set the rule that remains: **work and test locally; never
push without your word**.

GSD, a planning framework, was installed from an npm package in the evening (17:33). Nothing in the project documents
mentions it afterwards.

### 4.6 2026-10-07: session 2 and the parallel agents

The plan had 26 areas. Eight areas (themes; Story Board and Idea Vault; screenplay engine, formats and fonts; writing
comfort; breakdown and production people; schedule; call sheets, budget and templates; packages) were handed to separate
agents, each in its own Git worktree on its own branch. A written brief (`.claude/session-2-brief.md`, git-ignored) told
every agent the rules: commit only on your branch, never push, never touch `main`, use the migration number given, keep
memory in check, take a lock before running the end-to-end suite.

This produced 99 commits in one day and most of the product's feature work. It also produced the costs:

- **Migration numbers.** Several agents needed a new database migration at once. Placeholder migrations 0012 to 0015 were
  committed on branches so that numbers did not collide, and later replaced.
- **Line endings.** Windows tools wrote CRLF endings and Git showed whole files as changed. Many commits are titled
  "restore LF line endings".
- **Merge work.** Each area was merged into a `session-2` branch several times as other areas landed.
- **Disk.** Each worktree kept its own build folder. By evening the project folder was 273 GB (211 GB in worktrees, 60 GB
  in the build folder). 39 orphan worktrees were cleaned and about 205 GB came back. The first storage policy was written.

Decisions made that day (ADR-0014 to ADR-0016 accepted, ADR-0022 to ADR-0024 written): 15 built-in themes with Light, Dark
and System; production scene numbers; Indian-script PDF fonts; the Mac App Store trial end lock; the Microsoft Store trial;
and the four release files with internal builds behind a team password.

### 4.7 2026-10-08: decisions, joining and the first full run

**The decisions.** You decided that local merges to `main` happen only after tests and end-to-end specs pass; that a project
may hold several screenplays; and, late in the day, that the goal is the internal Windows `.exe` and Mac `.dmg` with the
AI weights inside every package (ADR-0025, which replaced the "download the model after install" design).

**The tooling.** `sccache` and the `rust-lld` linker were set up, and a build queue (`scripts/cargo-queued.mjs`) was written
because five parallel builds had left 0.4 GB of memory free. The queue allows one heavy build at a time with 4 jobs.

**The joining.** All branches were merged into `session-2`. The first full end-to-end run on the joined code failed 23 of
30 spec files. Every failure was traced and fixed in the product or in the spec, none skipped or weakened. Examples of
real product fixes: the store lock refused the team password screen's own question; a splash screen looped under the
lock; files from disk were shown through the wrong route; a screenplay layout change could drop a waiting save; a
project closing could report "no project open" before the crash marker was removed, so the next start offered recovery
after a clean close.

**The incident.** While removing a leftover worktree, `rm -rf` in Git Bash followed the `node_modules` junction into the main
checkout, emptied it and deleted 1,088 tracked files. They were restored from Git and `npm ci`. The policy now says: unlink the
junction first and never `rm -rf` a worktree.

### 4.8 2026-10-09: closing session 2

Dashes were removed from every text the app shows (your writing rule), the AI panel was docked beside the page, the
character-name Enter flow was changed, one more full end-to-end run passed (30 spec files, 158 tests, 0 failed, about 20
minutes), 79 plan boxes were ticked with evidence, `session-2` was merged into `main` locally as `c8ea7da`, and 37 merged
branches were deleted. No installer was built and nothing was pushed.

### 4.9 How a feature was developed (the repeating pattern)

For almost every area the same steps were followed. Knowing them tells you what "built" means here.

1. **Requirement** from a specification section or an owner decision.
2. **Decision record** (ADR) when the specification was silent or two specifications disagreed.
3. **Database migration** (a new numbered SQL file, never edit an old one) and a migration test from the previous version.
4. **Operations** registered in the Rust registry with metadata: who may call it, whether it is destructive, whether the
   AI may use it and why not if hidden.
5. **Module code** that uses `Store::mutate` for every change.
6. **Tests first** (the process asked for them), including crash, failure and edge cases (empty, huge, Telugu and Hindi
   text, interrupted save).
7. **Screens** in React using the shared design system, matching the HTML mock-up.
8. **AI decision**: a read tool, a proposal tool, or "hidden" with a written reason; the coverage document is regenerated
   from the registry by a test.
9. **End-to-end spec** per workspace, run once at the end of the session.
10. **Feature status row** updated with evidence.

The pattern is strong. Its weakness is that step 10 is manual and lags: three rows were out of date.

### 4.10 Decisions that changed over time

| Topic | First | Later | Why it changed |
|---|---|---|---|
| Live collaboration | Live over the local network | Removed; file packages only | Risk and work (ADR-0007, 2026-09-30) |
| AI models | Three Qwen sizes chosen by hardware | One Gemma 3 1B model | Owner wanted no model choice (ADR-0013) |
| AI delivery | Download after install, signed manifest | Bundled in every installer | Owner wanted AI to work at first start; download code removed (ADR-0025, 2026-10-08) |
| Release files | One signed installer | Three, then four files (two internal) | Store plans, then the internal team builds (ADR-0017, 0023, 0024) |
| Trial | Local trial record and a signed licence file | The stores run the trial; those pieces were deleted | Stores decide purchase and trial (ADR-0023) |
| Platforms | Windows first | Windows 11 and Apple silicon, both required | ADR-0017 |
| Themes | Light only | 15 themes, Light, Dark, System | ADR-0014 |
| Screenplays per project | One | Several, deletable | Owner decision 2026-10-08 |
| Parallel work | Eight agents, a target folder each | One queue, then (now) no parallel agents by default | Memory and disk (Section 12) |
| When to run the end-to-end suite | Per branch | Once at the end of a set of areas | Hours wasted (memory file) |
| Pushing to GitHub | Branch, pull request, merge on green | Never, unless the owner says | A session pushed while checks were red |
| Internal `.dmg` signing | Signed and notarized | Unsigned for now | ADR-0024 amendment |
| Chapters (2026-10-09) | 26 areas, store first | Three chapters, internal testing first | Your retrospection |

### 4.11 Shortcuts taken to move forward

Not criticism: each one was reasonable at the time. This is the list of those that must now be paid back or accepted.

| Shortcut | Where | Verdict |
|---|---|---|
| Placeholder migrations 0012 to 0015 to reserve numbers | Branches on 2026-10-07 | Gone; the real ones landed |
| Agent-written tests committed failing, fixed later | 2026-10-06 | Fixed in the product; keep the habit "fix, do not skip" |
| Debug-only fault injection code in production source | `store::fault`, `exports::fault` | Acceptable if compiled out of release; verify once |
| "Verified" meaning "passed on the development machine" | Feature status words | Acceptable for now; a pilot gate is added |
| Test-only environment switches (data root, AI folder, deterministic sampling) | `lib.rs` | Acceptable; they are ignored by release builds and tested |
| Mac code written without a Mac | Entitlement, StoreKit, platform | Unverified; the Mac bring-up is Chapter 1 |
| Documents allowed to lag the code | Feature rows, README table | Fixed today; the lag will recur without a checker |
| One 150,000-line crate | `openframe-application` | Works; costs build time and link time |
| Writing the user manual early | `docs/manual/` | Manual still has the removed LAN figures; to rebuild |
| Dead worktree folders left in place | `.claude/worktrees` | To review and remove |
| GSD and extra plugins installed without a decision record | `.claude/`, user settings | To decide |

### 4.12 What to do differently, from this history

1. **One work item at a time**, finished and merged, before the next. The best days (2026-10-08 evening) were fixing
   failures on one branch; the worst were eight branches colliding.
2. **A file-ownership map before any parallel work**, and parallel work only when you ask for it.
3. **A status check in the build**: make the README table and feature rows come from one source so they cannot drift.
4. **Run the real thing earlier.** Eight days passed with no installer. The first release build on a clean machine will find
   problems that 2,000 tests cannot.
5. **Keep instructions short and current** so that a new session does not start with 15 KB of rules, some stale.
6. **Write the handoff**, not only the log.

## 5. How the application works (the big picture)

This section is the high-level design (HLD): the main parts and how they talk. Section 6 goes into the low-level design
(LLD): modules, functions and habits.

### 5.1 The four layers

```text
 Layer 1  Screens          React 19, TypeScript          apps/desktop/src            41,000 lines
 Layer 2  Shell            Tauri v2 (Rust)               apps/desktop/src-tauri      2,900 lines
 Layer 3  Brain            Rust crates                   crates/                     190,000 lines
 Layer 4  Data and helpers SQLite files, AI helper exe   project folders, ai/        files on disk
```

| Layer | Allowed to do | Not allowed to do |
|---|---|---|
| Screens | Show data, take clicks and typing, keep view state (which tab is open), call `of_invoke` | Read or write files, open the database, use the network, run programs |
| Shell | Create the window, apply the security policy, expose four commands, start the brain, serve project pictures through a safe protocol, write logs | Contain business rules (it is "an adapter, not the architecture") |
| Brain | Everything: rules, saving, search, import, export, AI control, licence checks | Depend on Tauri (it can be tested without a window) |
| Data | Hold the truth | Be opened by anything except the brain |

The rule "screens cannot touch anything" is enforced three ways: the Tauri capability list (no file, shell or network
permission), the content security policy, and ESLint rules that forbid importing the Tauri file and shell plugins. A
separate script, `scripts/architecture-lint.mjs`, checks the Rust side (for example, no operating-system code outside
`openframe-platform`, no database write outside `Store::mutate`).

### 5.2 What happens when the program starts

The function `run()` in `apps/desktop/src-tauri/src/lib.rs` (line 407) does this, in order:

1. **Detect how it was installed** (`distribution.rs`): store package or not; packaged builds keep data in the package's
   data folder.
2. **Choose the data folders.** Normally `%LOCALAPPDATA%\OpenFrame` for settings and `Documents\OpenFrame` for work. The
   end-to-end tests set `OPENFRAME_DATA_ROOT`, honoured by debug builds only, so tests never touch real data.
3. **Decide the channel**: Microsoft Store (if running inside an MSIX package), Mac App Store (store build on a Mac), or
   an internal team build (everything else).
4. **Find the bundled AI folder** next to the program (`<app resources>/ai`).
5. **Read the team password hash** if this build has one (embedded at build time by `build.rs`).
6. **Start logging** to `%LOCALAPPDATA%\OpenFrame\logs`: daily files, 14 kept, JSON lines; a panic hook records a redacted
   summary; release builds cap third-party log detail so no content can leak.
7. **Check the WebView2 runtime.** If missing, a message box offers Microsoft's download page and the program stops.
8. **Allow only one copy** of the program (second launch focuses the first), except in tests.
9. **Register the `ofasset` protocol**, the safe way for screens to show project pictures: a request is answered only if
   the file is inside the open project's `assets/` folder or the Global Idea Vault's `assets/`.
10. **Create the brain** (`AppCore::new`): open `app.sqlite` (settings, recent projects, shortcuts, themes), open the Global
    Idea Vault database, build the registry of 483 operations, load or create the local profile (a random id and your
    Windows user name as display name), start the entitlement service.
11. **Subscribe to sleep and wake**, so the open project is folded into its file before sleep.
12. **Start background maintenance** on another thread (remove items deleted more than 30 days ago from the Global Idea
    Vault) except while the team password is pending.
13. **Show the window.** The page loads, asks "is a project open?" and shows Application Home.

On a quit, the window event "destroyed" calls `core.shutdown()`: close the current project cleanly (see 5.9), flag running
tasks to stop. The AI helper programs are tied to the process (Windows Job Object) so they die with it.

### 5.3 How a screen talks to the brain

On the screen side (`apps/desktop/src/ipc/client.ts`):

```text
call("story.create_act", { title: "Act One" })
   -> invoke("of_invoke", { op: "story.create_act", args: {...} })
```

On the shell side (`lib.rs` line 65): `of_invoke` takes the name and arguments, runs the work on a background thread so the
window never freezes, measures the time (logs a line if slower than 250 ms), and returns either the result or an
`AppError` with a code, a human message and an optional technical detail.

On the brain side (`registry.rs` line 482), `dispatch`:

1. Finds the operation by name. Unknown name: refused with `security.unknown_operation`.
2. Rejects absurd arguments (more than 32 levels deep, a list over 50,000 items, an object over 1,000 keys).
3. Asks the entitlement service whether the licence state allows the operation (the store lock and the team password gate
   live here, so they cannot be bypassed by any screen).
4. Calls the handler.

Each handler parses its arguments into a Rust struct with `deny_unknown_fields`, so an extra or misspelled field is an
error, then does its work.

### 5.4 Reads and writes

**A read** (a query) uses a separate read-only database connection so it never waits for a write. Example: opening the Story
Board calls a query that returns acts, sequences and cards.

**A write** (a command) always goes through one function, `Store::mutate` (`store.rs` line 433). It:

1. Checks the actor has the needed permission (`Capability::Edit` and so on).
2. Marks the save indicator "Saving".
3. Takes the single write connection and begins an IMMEDIATE transaction.
4. Resets the change-capture list. Database triggers record every row changed inside the transaction.
5. Runs the feature's code.
6. Reads the captured changes, re-indexes the changed rows for search, writes an Activity entry and (unless the command is
   marked not undoable) one undo step containing all the changes.
7. Commits. If anything failed, the transaction rolls back and files created during the attempt are removed.
8. After commit, deletes files that were scheduled for removal, then sends a `DataChanged` event listing the tables and ids.
9. Marks the save indicator "Saved" (or shows an error for a storage failure).

Why this matters: any feature built on `mutate` gets permissions, atomic saving, undo, redo, search and history without
extra code, and no feature can forget a step. The cost is that the pipeline is the most important code in the product; its
tests (`core_pipeline.rs`, `property_invariants.rs`) are among the most valuable.

### 5.5 Events and the screen cache

After every commit the brain sends one event on the channel `of://event`, for example "tables `story_scene_card` and
`story_act` changed, ids ...". The screens use React Query with `staleTime: Infinity`: a query result never expires by
itself. Each query declares which tables it reads. When an event names those tables, the cache entry is invalidated and the
screen asks again. No polling, no manual refresh. The effect: two panels showing the same data always agree, and a change
made by the AI Apply button appears in every open screen.

### 5.6 Undo and redo

Undo is generic. At open, the database installs triggers on every table that appears in the catalog. When a command runs,
the triggers write before and after images of the changed rows to a hidden capture table. `mutate` bundles them into one
step labelled with the command ("Move scene"). Undo reverses the rows; redo replays them. Two protections: a step refuses
to undo if later work by someone else touched the same rows, and the AI crash recovery uses undo to compensate a half-applied
Change Set. The editor is special: while you type, each save sends the difference to Rust, and Rust records one undo step
per burst of typing (a coalesce key).

### 5.7 The project on disk

```text
Documents\OpenFrame\Projects\Night Bus.openframe\
  openframe.json         identity and versions (human readable)
  project.sqlite         all structured data
  project.sqlite-wal     recent changes, while the project is open
  project.sqlite-shm     shared memory index for the WAL, while open
  assets\ab\<id>.png     pictures, audio, video, files, in 256 sub-folders by id
  cache\                 rebuildable: thumbnails, intelligence.sqlite for the AI
  recovery\              session.json (the crash marker) and checkpoint.sqlite (last confirmed copy)
  backups\               pre-migration and pre-replace safety copies
  .lock                  a file lock so only one OpenFrame writes this project
```

Other places:

- `Documents\OpenFrame\Global Idea Vault\` with its own `vault.sqlite` and `assets`.
- `%LOCALAPPDATA%\OpenFrame\app.sqlite`: settings, recent projects, shortcuts, themes, the local profile.
- `%LOCALAPPDATA%\OpenFrame\logs\`: 14 days of redacted logs.
- `%LOCALAPPDATA%\OpenFrame\entitlement\`: licence notices and, in internal builds, the team gate counter.

### 5.8 Opening a project (`open_at`, `project.rs` line 750)

1. Refuse network folders and reserved names. Refuse if the folder is missing.
2. If this project is already open, return it; if another is open, close that one cleanly first.
3. Take the single-writer lock.
4. Remove staging files left by a crash.
5. Check whether the last session ended uncleanly (the crash marker `recovery/session.json` still exists).
6. Open the database: check integrity, screen the schema for code OpenFrame did not create (triggers and views from a
   hostile file), then plan the migration. If an upgrade is needed, write and verify a safety backup first, then run all
   pending migrations in one transaction with foreign key and integrity checks before commit.
7. Check that the identity in the database matches `openframe.json`.
8. If the last session was unclean, prepare a recovery offer: time of the last activity, whether a checkpoint exists, its time.
9. Write a new crash marker.
10. Make sure the local user is a member (Owner) of the project.
11. Finish any AI Change Set that was interrupted mid-apply, by undoing it (ADR-0019).
12. Purge items deleted more than 30 days ago.
13. Update the manifest versions. Rebuild search if the index is empty or the project was migrated.
14. Register the project in the recent list and tell the screens.

### 5.9 Closing a project and recovery

A clean close folds the WAL into the main file (a checkpoint), copies the whole database to `recovery/checkpoint.sqlite`
("the last confirmed saved state"), and deletes the crash marker. If the program is killed, the marker remains. The next
open offers two choices: keep the latest autosaved state (everything committed is already in `project.sqlite`) or return to
the last confirmed checkpoint (the older copy). The dialog was once wrong about which was newer; `describeRecovery`
(`RecoveryDialogs.tsx` line 39) now compares the actual changes and times and has tests.

Autosave is simply "every change is a committed transaction". There is no separate save step; the Save button only
refreshes the checkpoint. The typing path adds the screen-side debounce: the editor waits 400 ms after the last key, then
sends the difference. A failed send keeps the text on screen and retries every 3 seconds.

### 5.10 The database in detail

- **Connection policy** (`openframe-persistence/src/lib.rs`): WAL; `synchronous=FULL` (a committed change survives power
  loss); foreign keys on; busy timeout 5 seconds; attaching other databases disabled (limit 0), so SQL smuggled in a
  received project cannot create files elsewhere.
- **Windows long paths:** paths of 240 characters or more use a special file layer so SQLite can open them.
- **Two kinds of connection:** one writer (guarded by a lock) and one read-only reader (`query_only=ON`).
- **Untrusted schema screening:** a project received from someone else may carry triggers or views that run code inside
  OpenFrame's connection. At open, every schema object must have a plain name, and every trigger and view must match
  exactly what OpenFrame's own migrations create. Anything else refuses the file untouched.
- **Migrations:** 16 for projects (core, vault, story, screenplay, production, visual planning, schedule, AI, collaboration,
  performance, AI apply journal, story card and vault research, screenplay engine, production depth, schedule timeline, call
  sheets and budget), 4 for the vault, 2 for the app. A checksum of each applied migration is stored; opening a project whose
  migration text changed is refused. `migrations/frozen.json` and `scripts/check-migrations.mjs` enforce "never edit a
  shipped migration".
- **Identities:** every row has a UUIDv7 text id (time-ordered), never a visible number. Visible numbers (scene 12) are
  derived at read time or frozen as labels at lock time.
- **Deletes are soft:** a deleted item moves to Recently Deleted for 30 days and can be restored; the purge is automatic and
  runs at project open.

### 5.11 Search

Two searches share the data. **Keyword search** is SQLite FTS5 over a `search_doc` table that the mutation pipeline updates
in the same transaction as the change. **Meaning search** (for the AI) is a separate derived file, `cache/intelligence.sqlite`,
rebuilt from the project by a background worker after each commit: documents per scene, card, character and so on,
cut into chunks, each chunk turned into 384 numbers by the embedding helper, compared by exact vector distance with the
`sqlite-vec` extension compiled into the program (not loaded at run time). A small graph of relations (character appears in
scene, scene belongs to sequence) is stored beside it. If the file is damaged or from another version it is deleted and
rebuilt. Private notes are filtered by owner at every step.

### 5.12 Import, export and page layout

The `openframe-import-export` crate (17,700 lines) reads and writes Final Draft (FDX, as XML), Fountain, Word (a ZIP of XML),
PDF (reading text back with embedded text markers) and plain text. The screenplay page layout (margins, keep-together rules,
(MORE) and (CONT'D), page breaks, header and footer) is one function used by the on-screen page view, the print preview and
the PDF, so they agree. PDF fonts are embedded and shaped (Courier Prime and Noto Sans for Devanagari, Telugu, Tamil,
Malayalam and Kannada) so Indian-script text prints correctly. All parsers have size limits, depth limits and time limits;
a ZIP bomb or a PDF with a quadratic structure is refused. A failed export writes nothing and leaves the previous file.

### 5.13 Packages and backups

A **backup** or **full project package** is a ZIP with a manifest, a copy of the database and the assets, written to a place
you choose through a temporary file and an atomic rename. **Exchange, review and response packages** carry selected
scenes and comments to another person. Restoring a package creates a new project with a new identity, or replaces an existing
one after a verified safety backup of it (`backup.rs` around line 1120). Private notes in a restored backup belong to the
person restoring it (decision 2026-10-08). Interrupted restores leave nothing behind (tested).

### 5.14 Licence, trial and the team password

`openframe-entitlement` holds the state machine. In an internal build the state is always "InternalBuild": nothing is locked.
If a team password hash is embedded, the brain refuses every operation except the password screen's own and the appearance
read until the password is right. Wrong tries are counted before the check and saved, so killing the program cannot reset the
counter: 5 wrong tries wait 1 minute, doubling to 30 minutes. For the stores the same service reads the store's licence and,
when a trial has ended without a purchase, refuses every operation except Buy and Restore. This code has only been tested
with fake stores.

### 5.15 Windows and macOS

One code base. `openframe-platform` hides the differences behind plain functions.

| Topic | Windows (written and run) | macOS (written, compiled in CI, never run) |
|---|---|---|
| Program folder | Per-user install under the user profile | `.app` bundle; resources in `Contents/Resources` |
| Settings folder | `%LOCALAPPDATA%\OpenFrame` | `~/Library/Application Support/OpenFrame` |
| Documents | Known Documents folder (may be inside OneDrive) | `~/Documents` |
| Kill helper programs with the app | Job Object with kill-on-close | Process group |
| Atomic file replace | Windows replace call | POSIX rename |
| Spell check | Windows spelling API, with the user's dictionaries | Falls back to the web view's own checking; adapter not run |
| Key labels | Ctrl, Alt | Command, Option |
| Web view | WebView2 (Chromium) | WKWebView (Safari engine); layout may differ |
| AI engine | llama.cpp Windows x64, processor build, pinned | Not pinned; no engine |
| Installer | NSIS `.exe` and `.msi` configured | `.dmg` not configured |
| Trial plumbing | Microsoft Store licence calls (fake) | StoreKit 2 bridge in Swift and a licence server (fakes) |
| Automated end-to-end tests | `tauri-driver` with Edge WebDriver | No driver exists for macOS |
| Warnings on first run | SmartScreen | Gatekeeper |

Because the web views differ, CSS and editor behaviour that works in Chromium can differ in Safari's engine. This is the
largest unknown of the Mac build and the reason for a manual Mac checklist.

### 5.16 Security boundaries, in one list

1. Screens cannot touch files, the shell or the network (capabilities, CSP, lint).
2. One door with a closed list of operations, argument shape limits, unknown fields refused.
3. Paths from outside are resolved and confined in Rust (`openframe-security::confine`); network and device paths are
   refused; Windows reserved names and short names are rejected; files that Windows would execute are shown, not opened.
4. Archives and parsers have hard limits (entries, declared size, depth, time).
5. Project and index databases from elsewhere are screened for hostile schema objects.
6. The AI helper listens only on `127.0.0.1` with a random port and a per-launch key, runs with `--offline`, never runs from
   a project folder, its environment is an allow-list, its files are hash-checked before every start.
7. The AI cannot apply changes; only a local user action can.
8. Logs never contain project content; panic text is redacted; the asset protocol's refusal message is switched off in
   release builds.
9. The webview cannot read the database, the AI index, recovery data or backups (the asset scope covers `assets/` only).
10. Nothing is sent anywhere automatically. There is no telemetry.

Not protected: projects are not encrypted at rest (anyone with access to your Windows account can read them); the team
password hash in an internal build can be attacked offline by someone who holds the program file; the AI folder under a
per-user install is writable by the same user, so hash checks catch damage, not a malicious program running as you
(recorded as T16 in the threat model).

### 5.17 Errors, logs and diagnostics

Every error carries a stable code, a human message that says what happened to the data and what to do, and a technical
detail kept for support. The screens branch on the code, never the message. Logs record operation names, codes and timings,
not content. A user-started **diagnostics export** (Settings, General, Support) builds an inspectable bundle with no
content, paths, contacts, prompts or keys (test `core_015_...`).

### 5.18 Updates

There is no updater yet. A new version is a new installer. The store builds will update through the stores. An approval-based
updater for the direct installer is a Chapter 3 decision (the internal build has none).

### 5.19 Performance design

Heavy work runs off the window thread. Lists use virtual scrolling. Filters in the breakdown run in Rust in one query
(under 20 ms median in the performance document). The editor keeps typing inside a frame budget even with 6,000 elements
(test `typingBudget.test.ts`). The AI engine starts on demand and stops after 15 idle minutes. The first-answer time of the AI
on a processor-only machine is the one number that is poor (20 to 30 seconds in the logs).

## 6. Inside the code (low-level design)

### 6.1 The repository map

```text
C:\OpenFrame
  apps\desktop\            the program
    src\                   React screens (TypeScript)
    src-tauri\             Tauri shell (Rust) + capabilities + icons + macos\StoreKitBridge (Swift)
  crates\                  the 11 Rust crates
  services\licence-server\ small server for the Mac store channel (Rust)
  migrations\              SQL files: project (16), global (4), app (2), frozen.json
  packaging\               ai\ (pinned AI file list + notices), msix\ (Store package template)
  scripts\                 governance and build scripts (Node)
  tests\e2e\               end-to-end specs, drivers, run folders (git-ignored)
  tests\fixtures\          sample files for tests (13 Final Draft classes and more)
  templates\samples\       four sample scripts compiled into the app
  schemas\                 empty folder (no content)
  docs\                    documents
  HTML_Mockups\            181 design mock-ups
  .github\workflows\       CI: ci.yml, mac.yml, release.yml
  .ai-assets\              AI runtime and model files (1.1 GB, not in Git)
  .build-cache\            compiled output (34 GB, not in Git)
  graphify-out\            knowledge graph (git-ignored)
  .claude\                 Claude Code files (git-ignored)
```

Items that do not belong at the root according to the project's own layout rule: `HTML_Mockups`, `gsd-install.log`,
`.ruff_cache`, an untracked `AGENTS.md`, `desktop.ini` and an empty `schemas` folder. `HTML_Mockups` and `services` are
accepted by habit; the rest is clutter.

### 6.2 The size of the code (counted on 2026-10-09)

| Crate or folder | Rust lines | Of which in `tests/` |
|---|---|---|
| `openframe-application` | 150,467 | 43,903 |
| `openframe-import-export` | 17,697 | 7,070 |
| `openframe-entitlement` | 5,782 | 0 (tests inside files) |
| `openframe-ai` | 5,144 | 1,887 |
| `openframe-search` | 2,443 | 0 |
| `openframe-platform` | 2,432 | 0 |
| `openframe-security` | 1,949 | 466 |
| `openframe-persistence` | 1,604 | 0 |
| `openframe-test-support` | 989 | 0 |
| `openframe-domain` | 794 | 0 |
| `openframe-project-format` | 525 | 0 |
| Tauri shell | 2,867 | inside files |
| Licence server | 2,208 | inside files |
| Total Rust (all) | 194,901 | |
| TypeScript and TSX screens | 41,000 (without tests) | 61 test files |

The big crate: `openframe-application` is 77 percent of all Rust. A quarter of it (44,000 lines) is test code.

### 6.3 The 21 modules of the brain

| Module | Lines | What it does |
|---|---|---|
| `ai` | 28,273 | Assistant: loop, toolbox, 253 tool definitions, Change Sets, retrieval, index, knowledge guide |
| `schedule` | 10,946 | Schedule, stripboard, timeline, Day Out of Days, call sheets, budget |
| `packages` | 10,261 | Backup, project packages, exchange and review packages, import session, restore |
| `production` | 9,247 | Breakdown, catalog, locations, cast and crew, categories |
| `screenplay` | 8,541 | Screenplays, drafts, elements, editing, locking, numbering, page setup, compare, review |
| `story` | 6,893 | Acts, sequences, beats, cards, characters, timeline, episodes, Build Screenplay |
| `visual` | 5,141 | Moodboards, storyboards, shots |
| `exports` | 4,935 | PDF, XLSX, CSV exports of production documents; branding |
| `idea_vault` | 3,516 | Vault items, collections, research flags, send to story |
| `interchange` | 2,324 | Import and export of screenplays |
| `appearance` | 2,142 | Themes, Light and Dark, theme import and checks |
| `notes` | 2,108 | Notes, tasks, templates |
| `writing` | 1,830 | Suggestions, shortcuts, proofing, navigator columns |
| `comments` | 1,678 | Comments, private notes, review state |
| `project.rs` | 2,015 | Create, open, close, duplicate, archive, delete, recents, recovery |
| `diagnostics.rs` | 1,189 | Support bundle |
| `entitlement.rs` | 951 | Operations for licence and team gate |
| `files.rs` | 753 | Project Files cabinet |
| `retention.rs` | 223 | 30-day purge |
| `history.rs`, `search.rs` | 196, 176 | Activity and search operations |

Note: the AI module is bigger than the screenplay, story and breakdown modules combined. That is not an accident. For every
feature there is also an AI description of it (see 6.5).

### 6.4 The habits of the Rust code

These patterns repeat. If you know them, any file is easier to read.

**1. Register an operation.** Every module has a `register(r: &mut Registry)` function that adds operations by name:

```text
r.query("story.board", s_board)        reads, never changes
r.command("story.create_act", create)  changes; goes through Store::mutate
```

Each registration also carries metadata: who may call it (required capability), what it does in one sentence, whether it
is destructive or irreversible, whether the AI may call it and, if hidden, the reason code. A test refuses a build in which
an operation lacks metadata.

**2. Arguments are structs.** `#[serde(deny_unknown_fields)]` means a misspelled or extra field is an error. Results are
structs too, with a derive that generates the TypeScript type.

**3. One error type.** `AppError` carries a code ("permission.denied"), a message, an optional detail and a retry flag.
Helper builders exist for common cases (`not_found`, `invalid_input`, `storage`).

**4. Everything that changes goes through `Store::mutate`.** The closure receives a `Tx` helper with the connection, the
actor, file-creation tracking (files made in the transaction are removed if it rolls back) and deferred file deletion
(files removed only after commit). `soft_delete` and `restore_deleted` are shared helpers for Recently Deleted.

**5. Search projections.** A module registers how each table becomes a search document; the pipeline calls it for changed
rows, so search is never forgotten.

**6. Roles and capabilities.** Five roles (Owner, Editor, Commenter, Viewer, Export-only) map to capabilities (View, Edit,
SoftDelete, Import, CreatePackage, UseAi, ApplyChangeSet and others). The local user is always Owner.

**7. Test helpers.** `openframe-test-support::TestEnv` creates an isolated application with a project, runs operations as an
actor with any role, restarts the application, and offers a fake embedder and stress helpers.

**8. Fault points.** Debug builds have switches that make a commit fail or a disk fill, to test failure paths. They are
compiled out of release builds (a unit test in the shell checks the related switches stay off).

### 6.5 How the AI mirrors the product

For each feature domain there is a file under `modules/ai/catalog/` that describes the "proposal tools": which operations a
tool uses, its argument schema, how to resolve names to rows ("Day 3", "the barn"), and how to preview the change. They are
declared with a macro (`spec!`). There are 193 proposal tools and 60 read tools. A test generates `ai-tool-coverage.md` from
the registry and fails if an operation has no explicit AI decision.

Good: the AI cannot reach an operation by accident; adding a feature forces a conscious decision.
Costly: every feature change has a matching AI change (9,000 lines of catalog), and the documentation lists 178 operations
that are deliberately hidden with a reason.

### 6.6 The screens

| Folder | Lines | What |
|---|---|---|
| `workspaces/screenplay` | 7,192 | Editor (ProseMirror), drafts, compare, review, lock, page setup, panels |
| `workspaces/production` | 7,066 | Tabs: overview, catalog, locations, cast and crew, moodboards, storyboards, shots, schedule, reports, sides, budget, Day Out of Days |
| `workspaces/story` | 4,708 | Board, outline, characters, timeline, episodes, drawers |
| `workspaces/call-sheets` | 3,202 | Call sheet editor and schedule views |
| `workspaces/idea-vault` | 3,154 | Vault grid, drawer, dialogs |
| `ipc` | 6,508 | Client, query hooks, generated types (809 files) |
| `app` | 2,640 | Home, shell, navigation, search overlay, toasts, shortcuts |
| `features/*` | about 5,500 | Packages, interchange, licensing, export, appearance, team gate |
| `workspaces/settings` | 1,905 | Settings pages |
| `workspaces/breakdown` | 1,622 | Breakdown views and dialogs |
| `design-system` | 403 | Buttons, inputs, dialogs, empty states |
| `ai` | 800 | The assistant panel |

State: `zustand` for view state only (current route, open draft, panel open). Server data: React Query keyed by operation
name and arguments. Styling: Tailwind v4 with design tokens; every colour is a token so that 15 themes work (lint rule T1).
Accessibility: 458 `aria-label` or `role` attributes counted; no automated accessibility test.

Routes are found by file convention: each folder under `workspaces/` with an `index.tsx` becomes a workspace
(`app/routes.tsx` uses `import.meta.glob`). A missing workspace shows "This workspace isn't available in this build."

### 6.7 Where the real complexity is

Counted largest files (lines): `packages/import.rs` 3,512; `schedule/ops.rs` 2,678; `schedule/callsheet.rs` 2,374;
`story/board.rs` 2,277; `production/breakdown.rs` 2,194; the AI catalog files 1,400 to 2,100 each; `project.rs` 2,015. On the
screen side: `CallSheetEditor.tsx` 1,002; `ScreenplayEditor.tsx` 987; `moodboards.tsx` 962; `storyboards.tsx` 960.

Files of 2,000 lines are hard to read and to review, but they are internally regular (long lists of similar operations).
They are a maintenance cost, not a bug. A split is justified only when a file is changed often.

### 6.8 Scripts and gates

| Script | Job | Cost |
|---|---|---|
| `architecture-lint.mjs` | Rules W (writes), C (config), R (platform), T (theme colours) | Seconds |
| `check-migrations.mjs` | Numbering, frozen checksums | Seconds |
| `check-feature-status.mjs` | Table format, tests present, reachability | Seconds |
| `check-reachability.mjs` | Every Reachable row names a real route | Seconds |
| `check-writing-standard.mjs` | Dashes and banned words, with a ratchet baseline | Seconds |
| `check-links.mjs` | Relative links in living documents | Seconds |
| `plan-status.mjs` | Counts plan boxes | Seconds |
| `cargo-queued.mjs` | One heavy build at a time | n/a |
| `prepare-ai-assets.mjs` | Verify and stage the AI files by hash | Seconds |
| `release-validate.mjs` | Lists missing release prerequisites | Seconds |
| `ai-benchmark.mjs` | Time the AI | Minutes |
| `build-msix.mjs` | Store package | Not used now |

CI (`.github/workflows/ci.yml`, 236 lines): jobs for Rust, web, governance, end-to-end (release gate), supply chain.
`mac.yml` only compiles the Mac shell and runs the platform and entitlement tests on an Apple silicon runner. `release.yml`
(405 lines) is the release pipeline, never run to a release. CI cannot yet run the end-to-end suite or the AI tests because
the AI files (1.1 GB) are not on the runners; this was left for "the next push".

### 6.9 What a new developer should copy

- The registration and metadata pattern for every new operation.
- `TestEnv` tests that go through the registry, not around it.
- Plain human error messages with a stable code.
- One migration per schema change, with a fixture test from the previous version.
- A feature status row with evidence in the same change.

## 7. The AI system

This section answers the questions you asked about the bundled AI. Everything here comes from the code unless it says
"logs say".

### 7.1 What is packaged

| Item | File | Size | Source |
|---|---|---|---|
| Chat model | `ai/models/gemma-3-1b-it-q8-0-v1/model.gguf` | 1,069,306,368 bytes | Google Gemma 3 1B Instruct, Q8_0 conversion from `ggml-org` |
| Embedding model | `ai/models/bge-small-en-v1.5-q8-0-v1/model.gguf` | 36,685,152 bytes | BAAI BGE small English v1.5 (MIT), 384 numbers per text |
| Engine | `ai/runtimes/llama-cpp-b11259-win-x64-cpu/` (22 files) | about 43.7 MB | llama.cpp build b11259, processor build, Windows x64 only |
| Notices | `ai/notices/` | small | Gemma notice, BGE notice, llama.cpp licence |
| Total pinned | 25 files | 1,149,732,477 bytes | `packaging/ai/ai-assets.lock.json` |

How it gets into the installer: the build copies the folder `.ai-assets` (kept outside Git because it is 1.1 GB) into
`.build-cache/ai-bundle`, checks every file's size and SHA-256 against the lock file, and the Tauri setting
`tauri.bundled-ai.conf.json` adds it as a resource named `ai/`. The installed program finds it next to itself.

How the program trusts it: the lock file is compiled into the program. Before the first engine start (once per installed
version) every file is hashed; later starts compare size and time with the recorded check. A missing file switches AI off
with a plain message ("repair by reinstalling"), a changed file the same. There is no download, no model host and no key to
manage. States: not checked, checking, verified, missing, damaged, unsupported (a build with no engine for this kind of
computer). On the Mac today the state is **unsupported**: the lock file lists only a Windows engine. Core features keep
working; the assistant says it is not available.

### 7.2 Does it work with no internet

By construction yes. Engines run with `--offline` and `--no-webui`, listen on `127.0.0.1` only, and nothing downloads. The
end-to-end spec `72-ai-assistant` checks that while the AI works the program and its engines own only loopback sockets
(logs say it passed on 2026-10-09). Open items: the web view processes were not traced (threat T33), and nobody has run the
AI on a computer with the network adapter disabled and no Visual C++ runtime installed (the "clean machine" box).

### 7.3 Hardware needs

The metadata in the model file says minimum 4 GB of memory and recommended 8 GB; context 8,192 tokens. The engine is the
processor build, so a graphics card does not help. Threads: physical core count (clamped 1 to 16) for chat and half of that
(1 to 4) for the embedding engine. The engine starts when first needed, loads the model (the start timeout is 10 minutes), and
unloads after 15 idle minutes. After a crash it may restart twice within 10 minutes.

Measured by the logs on the development machine (processor only): full bundle check 1.5 seconds; one chat reply 4.0
seconds for a short test; a real project question took 20 seconds (exact count), 24 seconds (retrieval with sources) and 30
seconds (a proposal). Not measured: an 8 GB laptop, a low-end processor, the Mac, memory use of both engines plus the
program. The benchmark script exists (`scripts/ai-benchmark.mjs`); no results are in the evidence folder.

### 7.4 What the model receives

For each request `run_agent` (`orchestrator.rs` line 817) builds:

1. A **policy** text: the rules the model must follow. (It currently says "a filmmaking application".)
2. A **tool catalogue**: the core tools plus the tools most relevant to the request, trimmed to 7,000 characters. Relevance
   is a word-overlap score between the request's words and each tool's name and description (`toolbox.rs` line 356). Tools
   that change data are offered only when the request contains a "change" word.
3. A **scope**: what the person is looking at (current screenplay, selected scene, whole project).
4. **Retrieved context**: the best matches from keyword search plus meaning search, fused by reciprocal rank, each marked
   as untrusted data inside `<project_data>` tags.
5. **History**: the last 4 turns, shortened.
6. The **request**, which is the only text treated as an instruction.

The model must answer with one JSON object, constrained by a grammar, naming a tool and its arguments and saying whether
that tool's result will complete the answer.

Limits in code: request text 2,000 characters; model output 4,000; step output 400 tokens; tool arguments 4,096 bytes;
observation text 1,500 characters each and 6,000 in total; at most 15 observation items; final text 1,200 characters;
default 8 steps and never more than 12.

### 7.5 Retrieval, embeddings and the graph

- **Keyword retrieval:** SQLite FTS5 with BM25 over the same `search_doc` table the search box uses, using up to 12
  significant words after removing stop words (a hard-coded English list).
- **Meaning retrieval:** exact nearest-neighbour search over chunk embeddings with `sqlite-vec` (48 candidates).
- **Graph retrieval:** the top five entities are expanded through relations stored in the derived index.
- **Fusion:** reciprocal rank fusion with explainable boosts (explicit scope, exact name, relation, current draft).
- **Safety:** privacy is filtered during gathering and checked again from the real database rows when the context is
  assembled. A tampered index can change which objects are considered but never their text or visibility.
- **Status:** the design is complete and tested for correctness. **Quality is not measured**: there is no gold set of
  questions and no recall figures (feature row AI-003 is Specified). The plan says to drop the graph stage if it does not
  beat plain hybrid retrieval; nobody can tell yet.
- **Language limit:** the embedding model and the stop word list are English. Telugu or Hindi text is searchable by
  keyword (FTS5) but its meaning search is weak, and tool selection by word overlap fails for non-English requests.

### 7.6 Tools: how they are defined, chosen and run

1. **Definition.** A tool is a name, a description, a strict argument schema (closed objects, bounded strings, numbers and
   lists, 8 KB total) and the operations it uses. 60 tools read or compute; 193 propose changes.
2. **Exposure.** An operation is offered to the AI only if its metadata says so. Of 483 operations, 305 are reachable
   through tools; 178 are hidden, each with a coded reason. Examples of hidden groups: accepting or rejecting a Change Set
   (human only), project creation and deletion, permanent deletion, file-path operations, layout dragging, page setup,
   production numbers, licence and trial, Offline AI controls.
3. **Choice.** The model picks one tool per step from the offered list (see 7.4). A short list of plain commands ("Create
   a task to ...", "Mark the task ... as done") skips the model and goes straight to the proposal tool, because the 1B
   model answered such commands with reads on an empty project.
4. **Run.** `run_tool` re-checks everything (the offered list is not the boundary): unknown tool, arguments against the
   schema, permission. Read tools are bounded (pages of 80 items) and use the person's own permissions; another user's
   private notes are never visible. Proposal tools resolve names to rows ("Day 3", scene numbers) and ask one question if
   several rows match, never guess, record the base revision of every row they touch, and return a draft.
5. **Loop guards.** The same tool with the same arguments twice ends the loop with what was found. A product-guide answer
   after project facts were found is dropped. Hitting the step limit ends with a note.

### 7.7 Approval: how the AI is kept from changing things

- A proposal tool creates a **Change Set**: a title, a list of operations from the allow-list, previews, targets and base
  revisions. It is stored. Nothing in the project changes.
- The person sees **Proposed Changes** in the assistant panel and presses **Apply Changes** or **Reject**.
- Only a local user action can accept, reject or re-check a Change Set (`change_set.rs` line 66, `require_human`: the
  actor's origin must be `Local`). The tools for accepting do not exist: they are in the hidden list with the reason
  "human-only approval boundary".
- On apply, the program re-checks each target against its base revision. If anything changed meanwhile, the proposal is
  **stale** and is not applied.
- Apply runs each operation through the normal registry (the same permission, validation and undo as a manual action) and
  becomes **one undo step**.
- Before applying, a **journal row** is written in the same transaction that marks the Change Set accepted. If the program
  dies mid-apply, the next project open undoes the applied operations and marks the Change Set failed with a plain message
  (ADR-0019, tested at every boundary in `ai_apply_recovery.rs`).
- Destructive operations keep the extra confirmation the manual screen would ask.
- Prompt injection: project text is wrapped as data and the policy says never to follow instructions inside it. A 1B model
  cannot be trusted to obey that. The real protection is structural: even a fooled model can only produce a proposal, which a
  person reads before applying.

Conclusion: unauthorised data change by the AI is prevented by design, not by the model's good behaviour. This is the
strongest part of the AI work.

### 7.7a What a human could still be misled by

The proposal text is generated by a weak model. A person might apply a wrong proposal that looks plausible (for example
renaming the wrong character when two names are close). Mitigations: ambiguity asks a question, the preview names exact
targets, apply is one undo step, and the stale check prevents applying to changed rows. Residual risk is human, and the
pilot should watch for it.

### 7.8 Failures, timeouts, wrong tool calls, interruptions

| Event | Handling | Evidence |
|---|---|---|
| Engine fails to start | Up to 3 launches on new ports if the port was taken; otherwise AI says it is not ready; core features unaffected | `supervisor.rs`, `m1_10` tests |
| Engine crashes | Restart up to 2 times in 10 minutes, then give up | supervisor config |
| Engine hangs | Chat request timeout 180 seconds, embedding 120 seconds; start timeout 600 seconds | `client.rs`, `manager.rs` |
| Model returns invalid JSON | Constrained output; the step is parsed strictly; unknown tool names are refused | `parse_step` |
| Model picks a wrong tool | Schema and permission re-checked; ambiguity asks; the person can reject | `toolbox.rs` |
| Model repeats | Loop ends with a note | `REPEATED_NOTE` |
| Request cancelled or project closed | Background index tasks are cancelled (a bug found and fixed on 2026-10-06) | `ai_retrieval` tests |
| Program killed during apply | Undone on next open | ADR-0019 |
| Bundle damaged or missing | AI switched off, plain message, project untouched | `bundled.rs` tests |
| Index damaged | Deleted and rebuilt; retrieval falls back to keyword search | `IntelligenceDb::open` |
| Windows engine log | Sanitised (`redact_process_output`); a past leak of prompt text was fixed | `supervisor.rs` |

### 7.9 Are all intended operations AI-reachable

Not all, by design. Reachable: reading most data and proposing most edits in Story, Screenplay (scene level), Breakdown,
Production, Schedule, Call Sheets, Budget, Idea Vault, Notes and Tasks, Files, Comments. Hidden on purpose: exports, backups
and packages (they write to places the person chooses), project lifecycle, appearance, licence, Offline AI controls, page
layout and production numbers. Two gaps worth a decision: the AI cannot start an export or a backup, and it cannot navigate
screens that have no read tool. Neither blocks internal testing.

### 7.10 Is it good enough for paying users

Unknown, and that is the honest answer. The safety design is strong. The usefulness of a 1B model for tool-using work is
the open question:

- The logs show the model proposing a duplicate task, answering a command with a clarification, and misrouting a summary
  request. Fixes were added around the model (direct commands, grounded names) rather than inside it.
- Speed on a processor is 20 to 30 seconds for a multi-step answer on the development machine. On an 8 GB laptop it will be
  slower.
- Non-English requests are largely unsupported.

A fair statement for a first pilot: "an experimental assistant that is safe but not yet reliably smart". The recommended
next steps are measurement (a gold set of 120 questions with figures), a hardware benchmark on an 8 GB machine, and a
decision after reading the numbers. The model and architecture are not to be changed without evidence and approval.

### 7.11 Redistribution obligations

| Component | Licence | What shipping it requires | State |
|---|---|---|---|
| Gemma 3 1B weights | Gemma Terms of Use + Prohibited Use Policy (ai.google.dev/gemma/terms) | A copy of the terms to recipients; the use restrictions as an enforceable term in the user agreement; a "Notice" text file with a fixed sentence; mark modified files | Notice file present and exact; the end-user agreement does not exist yet; counsel to confirm; the owner's check is open |
| BGE small English | MIT | Keep the licence notice | Notice present |
| llama.cpp | MIT | Keep the licence text | Present |
| Visual C++ runtime DLLs | Microsoft redistributable terms | Ship app-local | Decided; not yet on a clean machine |
| Fonts, Rust and npm packages | Various | `THIRD_PARTY_LICENSES.md` generated | Present |

A third-party summary says Gemma 4 uses a different, more open licence. That is unverified and is not a reason to change the
model.

## 8. Feature inventory and status

### 8.1 How to read the tables

The status words are the project's own (from `feature-status.md`), counted on 2026-10-09:

| Word | Meaning | Rows |
|---|---|---|
| Specified | Written down, little or nothing built | 17 |
| Designed | A contract with data, operations, screens and tests planned | 10 |
| Built | Code exists and passes its own tests | 31 |
| Reachable | A normal user can find and use it | 45 |
| Verified | Saving, undo, recovery, permissions, export, restart, failures and an end-to-end test are proved | 65 |
| Release-ready | Performance, security, accessibility, installer, migration, docs all pass | 0 |
| Process | A rule, not a feature | 2 |
| Not applicable | Removed on purpose | 3 |

To answer your question with your own classes, this report maps them like this:

| Your class | Meaning here | Rows |
|---|---|---|
| Verified | Verified, on the development machine, in a debug build | 65 |
| Implemented but unverified | Reachable (45) and Built (31) | 76 |
| Partial | Rows whose "what is missing" cell lists a real gap, inside the two lines above | many |
| Broken | None found by reading; one unproven risk (typed text on close) | 0 proved |
| Missing | Specified and Designed rows | 27 |
| Intentionally deferred | Store and launch rows (Chapter 3) and Mac proof | 14 |

No feature is Release-ready, because that word needs an installer, a clean computer and accessibility proof.

### 8.2 Project foundation (17 rows)

| Feature | Status | What proves it | What is missing |
|---|---|---|---|
| Create, open, close, duplicate, archive, delete, recents, locate | Reachable | `core_pipeline.rs` duplicate and archive tests; journey and restart specs | **No test calls `project.delete` or `project.locate`**; clean machine |
| One writer lock; network folders refused | Reachable | `project_is_single_writer`, `network_paths_follow_the_policy...` | No end-to-end; mapped drive letters and shares behind links not refused |
| Continuous autosave, Save checkpoint, save indicator | Reachable | autosave restart test; journey step | Forced-kill matrix is built (area 3) but the row text predates it; typed-on-close gap |
| Crash detection and recovery offer | Reachable | `crash_offers_recovery...`; `RecoveryDialogs.test.tsx` | A crash end-to-end step |
| Migrations: backup first, one transaction, refuse newer | Built | migration tests; fixtures for every old version assert the backup file | Restore of the backup not asserted; disk-full case |
| Generic undo and redo | Verified | property and pipeline tests; journey | Property tests across every module |
| Recently Deleted, Activity, 30-day purge | Verified | seven purge tests; spec 25 | Clean machine |
| Project Files cabinet | Reachable | `core_pipeline.rs` file tests | End-to-end only opens the page |
| Global search (FTS5), private notes owner-only | Reachable | journey Ctrl+K step | Every new entity must register a search document |
| App shell, Home, navigation, toasts with Undo | Reachable | `AppHome.test.tsx`; journey | None known |
| Project Home production route and next-day summary | Built | one test | No next-day summary |
| Roles and capability matrix enforced in Rust | Built | `viewer_cannot_mutate` and similar | No role names in the UI; tests per new operation |
| Offline operation | Reachable | spec 72 loopback check | A network-disabled core workflow with traffic capture |
| Local logs, redacted, 14 days | Built | redaction tests | Ordinary log lines are not all redacted; logs not reachable from the UI |
| Export diagnostics bundle | Built (corrected today) | `diagnostics.rs`, `SupportDiagnostics.test.tsx` | End-to-end; manual read of the bundle |
| Windows High Contrast; no meaning by colour only | Built | contrast tests; spec 91 | A manual pass with Windows contrast themes on |
| Keyboard-only walkthrough, labelled icon buttons | Specified | none | The whole accessibility pass |

### 8.3 Idea Vault (8 rows)

| Feature | Status | Note |
|---|---|---|
| Vault views (grid, card, list, folder) and empty state | Reachable | 24 tests in `idea_vault.rs`; spec 21 |
| Add note, image, URL, file, voice note, video | Reachable | Voice recording on a clean machine and antivirus cases not tried |
| Drag-in files, partial success, unavailable files | Reachable | Failure injection cases missing; Explorer drag-in needs a manual check |
| Selection, collections, research flags, Recently Deleted | Verified | `vault_research.rs`, purge tests |
| Global Idea Vault and copy to project | Reachable | |
| Search the vault, Quick Capture | Reachable | |
| Send to Story Board | Reachable | Lineage test with Story |
| AI read and proposal tools for the Vault | Built | Eval cases missing |

### 8.4 Story Board (12 rows)

| Feature | Status | Note |
|---|---|---|
| Acts, sequences, beats, cards; board and outline; zoom | Verified | 33 tests in `story.rs`; specs 30, 31 |
| Card drawer, drag with insertion line, multi-select, parking lot | Verified | |
| Delete rules for non-empty act and linked card | Verified | |
| Build Screenplay (preview, existing screenplay, missing headings) | Verified | Asks to add a heading or leave the card out; never invents |
| Characters, relationships, timeline, sequence hub | Reachable | End-to-end for relationships, timeline, hub |
| Series, seasons, episodes | Reachable | Episodic versus series types |
| Story Board PDF export | Reachable | Golden PDF |
| Long scene description; card display settings; presets | Verified | |
| Capacity limits for notes and attachments | Reachable | |
| Card links follow scenes when identities change | Reachable | End-to-end step missing |

### 8.5 Screenplay (26 rows)

| Feature | Status | Note |
|---|---|---|
| Editor, eight element types, auto progression, override | Verified | Flow matrix test; specs 40, 42 |
| Structured model with stable ids | Built | Migration 0013 keeps ids |
| Element rule profiles (one table drives layout, Enter, FDX, Fountain) | Built | |
| Find and replace with scene or whole-script scope | Verified | |
| Focus mode, Writing Room, notes, comments | Reachable | |
| Drafts, history, compare, review rounds | Reachable | |
| Lock shooting draft, revisions | Verified | |
| Production scene numbers | Reachable | Frozen at lock; lettered inserts |
| Deterministic pagination shared by preview, print and PDF | Reachable | Tests compare preview with PDF positions on Letter and A4 |
| Keep-together rules, MORE and CONT'D | Built, Reachable | |
| Page breaks, title page, header and footer, page setup | Verified | |
| Custom elements with shortcuts | Verified | Bounded against hostile imports |
| Scene navigator columns | Verified | |
| Smart suggestions; custom shortcuts | Verified | One Enter on a character name moves to Dialogue |
| Spell check with your dictionaries | Verified on Windows | Mac adapter never run; Telugu and Hindi have no dictionary underline |
| Multilingual text through every format | Verified | Indian-script fonts embedded |
| Print preview identical to PDF | Reachable | |
| Scene Hub; moving a scene in production | Reachable | Dependency warning tests |
| Delete draft; several screenplays per project; delete a screenplay | Verified | |

### 8.6 Import, export and packages (13 rows)

| Feature | Status | Note |
|---|---|---|
| Final Draft (FDX) import with preview and warnings | Verified | 13-class fixture suite |
| FDX export valid and re-importable | Reachable | **Reopen in a licensed Final Draft is an owner task** |
| Fountain, Word, PDF (best effort), text import and export | Reachable, Built | PDF import reads exported PDFs exactly; scanned PDFs refused |
| Import report exact; failed export writes nothing | Reachable | |
| Screenplay outputs carry no OpenFrame branding | Built | |
| PDF visual regression goldens | Built | Windows renderer only |
| Exchange, review, response packages | Verified | |
| Backup, full project package, identity collision | Verified | **Manual restore drill on a second Windows account is an owner task** |
| Live LAN collaboration | Removed | |

### 8.7 Breakdown, catalog, people (10 rows)

Verified: production source and breakdown with suggestions never auto-confirmed; categories with colour, text, icon and
tooltip; efficiency features (bulk, quick reuse, filters); locations with call-sheet fields; cast and crew with private
contacts. Reachable: script change reconciliation, source phrase jump and the short report, large-project performance
(measured, filters under 20 ms), catalog, replace a location. Missing for all: a run from an installed program.

### 8.8 Visual planning (3 rows, all Reachable)

Moodboards, storyboards with a sketch tool, shot list. Each has unit tests and export tests. **No end-to-end step touches
the canvas, the sketch dialog or drag reorder.** This is the least proved production area. The AI covers them (37 tools).

### 8.9 Schedule and Day Out of Days (10 rows, all Verified)

Stripboard, list and calendar, drag into a day, break markers, day timeline, scene start time, availability warnings
(advice only), filters, stale and review state, PDF and spreadsheet export, no automatic optimiser, Day Out of Days derived
from cast and schedule with manual correction. Spec 61. Missing: an optimised performance run and the manual source.

### 8.10 Call sheets, reports, budget, templates (13 rows)

Verified: create from a day with a snapshot, header and day info, locations, scene table with starts, optional sections,
sides and six report types, Planned and Actual budget, templates as independent copies, four sample scripts, branding on
exports. Reachable: call sheet PDF (golden is a text layout, no pixel check), notes and tasks, the template list.

### 8.11 Appearance (8 rows, all Verified)

15 themes, Light, Dark, System, theme import with strict validation (57 hostile inputs), contrast checks in Rust and in
rendered text (16 themes by 2 modes by 15 screens), paper colours fixed so exports never change. Missing: Windows contrast
themes by hand.

### 8.12 Offline AI (14 rows)

| Feature | Status | Note |
|---|---|---|
| Bundled AI with pinned hashes | Verified | Windows x64 only; no installer |
| Bounded assistant, toolbox, hybrid retrieval, human-only apply | Verified | Retrieval quality unmeasured |
| Retrieval gold set and metrics | Specified | **Not built** |
| Hardware benchmark | Specified | **Not run** |
| Crash-safe Change Set apply | Verified | Killed at every boundary |
| Real-model end-to-end | Built | Index rebuild not covered; quality findings |
| No-egress test | Built | Web view processes not traced |
| Prompt injection and poisoning corpus | Built | Poisoning corpus missing |
| Coverage matrix regenerated from the registry | Built | |
| Report AI output action | Specified | Needs a support address (store policy) |
| Model update regression suite | Designed | |
| Product knowledge file | Built | Needs updating for newer features |

### 8.13 Platform and release (26 rows)

Built: CI jobs, supply chain checks, architecture lint, migration fixtures, the platform boundary crate, entitlement
interface and adapters (with fakes), the team password gate. Designed or Specified (not done): the Mac adapter run, WebView2
missing handling, clean-machine install tests, store package, WACK, privacy policy final, store listing, release notes,
signed installer and updater, licence terms, Partner Center identity. Reachable: the trial and lock screens, first-run safety
screens. These rows are mostly Chapter 3.

### 8.14 Reverse reachability: backend with no screen

The feature list checks "feature to screen". The other direction (operation to screen) was counted: of 483 operations, 464 are
called by name from the screens. Cast, crew and catalog use names built at run time (`${kind}.delete`), so they are
referenced too. **About 14 operations have no screen caller**:

- Hidden from the AI too (possibly dead code): `screenplay.merge_scene`, `screenplay.split_scene`, `screenplay.move_element`,
  `ai.change_set.get`, `ai.index_rebuild`, `search.rebuild`.
- Reachable only by the AI: `schedule.delete`, `schedule.move_day`, `templates.get`, `trash.list`,
  `screenplay.custom_elements`, `screenplay.insert_element`, `screenplay.update_element`, `screenplay.delete_element`,
  `screenplay.update_revision`.

Two readings are possible: the screens use other operations for the same job (for example `screenplay.apply_edits` for typing),
or a user-visible action is missing ("merge scenes", "split a scene"). Each needs a one-line decision: wire it, keep it
AI-only on purpose, or delete it ("clean as you go"). None blocks internal testing.

### 8.15 Nonfunctional buttons and placeholders

By search: no `todo!()`, no `unimplemented!()`, no `TODO` or `FIXME` in first-party Rust or TypeScript (counted 0). A missing
workspace shows an honest "isn't available" screen. The reachability script and end-to-end spec 95 click every canonical
route (30). No dead button was found by reading, but clicking every button on every screen was not done; the pilot and the
accessibility pass will cover it.

## 9. Chapter 1 quality report

The ten parts you asked for, A to J. Each finding names its evidence. Severity words: Blocker (can lose a writer's work or
stop the pilot), Serious (a main job fails or a promise is false), Moderate, Minor.

### 9.A What is built

Real, working subsystems (read in code and, where noted, proved by tests the logs report):

| Subsystem | What exists | Real or placeholder |
|---|---|---|
| Project storage | Folder format, single-writer lock, SQLite with WAL, migrations with safety backup, integrity check, untrusted-schema screening, recovery marker and checkpoint | Real |
| Mutation pipeline | `Store::mutate`: permission, transaction, undo capture, search update, activity, events | Real |
| Registry and IPC | 483 operations, four commands, strict argument handling | Real |
| Undo and redo | Trigger-based and generic; protects later work | Real |
| Recently Deleted | 30-day retention and automatic purge | Real |
| Search | FTS5 over all modules, owner-only private notes | Real |
| Idea Vault | Project and global; five item types; research flags; send to story | Real |
| Story Board | Acts, sequences, beats, cards, characters, timeline, episodes, Build Screenplay | Real |
| Screenplay editor | ProseMirror editor, eight elements, suggestions, shortcuts, spell check, find and replace, drafts, compare, review, lock, revisions, numbering, page setup, custom elements | Real |
| Formats | FDX, Fountain, DOCX, PDF, text; layout engine; Indian-script fonts | Real |
| Breakdown and production lists | Suggestions, categories, catalog, locations, cast and crew with private contacts | Real |
| Visual planning | Moodboards, storyboards with sketch, shot list | Real, thinly proved |
| Schedule | Stripboard, list, calendar, markers, timeline, warnings, Day Out of Days | Real |
| Call sheets, sides, reports, budget | Snapshot, refresh, finalize, PDF and spreadsheets, branded exports | Real |
| Templates and samples | Eight templates, four sample scripts | Real |
| Appearance | 15 themes, Light, Dark, System, import | Real |
| Packages and backup | Backup, project package, exchange, review, response; restore drill | Real |
| Offline AI | Bundle check, two helper engines, bounded agent, 253 tools, retrieval, Change Sets, crash-safe apply | Real on Windows x64 |
| Entitlement | Eight states, store lock in the dispatcher, Microsoft and Mac adapters, team password gate | Real code; store parts tested with fakes only |
| Platform boundary | Windows adapter complete; macOS adapter written | Windows real, Mac unrun |
| Governance | Architecture lint, migration policy, feature status, link, writing checks | Real |
| Licence server | Mac channel trial server | Real code, no deployment |

Nothing found is a demonstration or a mock-up behind a real-looking button. The one honest placeholder is a screen that
says "This workspace isn't available in this build" when a workspace folder is missing; none is missing today.

### 9.B What is not built

| Item | Evidence | Chapter |
|---|---|---|
| An installer, ever | `release.yml` never run; no `.exe` in the build folder except test builds | 1 |
| macOS: disk image target, AI engine, any Mac run | `tauri.conf.json` targets; `ai-assets.lock.json`; `mac.yml` only compiles | 1 |
| Typed text flush on window close | `sync.ts`, `lib.rs` | 1 |
| Protection for projects in synced folders | `default_projects_dir` | 1 |
| Delete and locate project tests | grep of tests | 1 |
| Retrieval and tool-choice measurement | feature row AI-003 Specified | 1 |
| Hardware benchmark results | AI-004 Specified | 1 |
| Accessibility pass | CORE-017 Specified; area 17 has 0 of 15 | 1 |
| Release-build smoke test and a gated-build test | E2E drives debug builds | 1 |
| A manual that matches the product | Manual has LAN figures 164 to 168 | 1 |
| End-user licence agreement | `LICENSE-PENDING.md` | 3 |
| Report AI output action | AI-010 Specified | 3 |
| Real Microsoft Store calls, StoreKit run, licence server deployment | Fakes only | 3 |
| Updater | None | 3 |
| Website | None | 2 |
| Next-day summary on Project Home | CORE-011 | 1 |
| Optimised performance run on joined code | Log: not run | 1 |
| "Change folder" button of the data note | Mock 194 vs operations | 3 |

### 9.C What is ready for what

Honest classes, using your four:

| Class | What | Why |
|---|---|---|
| **Ready for internal testing** (after the Blocker items in 11.1 are closed and an installer exists) | Projects, Story Board, screenplay editor, formats, drafts, breakdown, schedule, call sheets, themes, packages, the AI as an experimental assistant | Built, covered by integration and end-to-end tests on the development machine, with recovery and undo |
| **Technically implemented but not verified** | Visual planning, the team password gate in a real build, Windows spell check on other machines, backups restored on another account, Idea Vault voice notes and drag-in, WebView2 handling, the release build as a whole, every Mac path | No test outside the development machine or no test at all |
| **Validated for production** | Nothing | No release candidate, no clean machine, no accessibility or soak report |
| **Incomplete, broken or unsafe** | None found broken. Unsafe-by-gap: the close-window flush (unproven), synced folders, the temp-folder fallback | See 9.F and 11 |

### 9.D What still needs testing

By kind:

- **Unit and integration:** `project.delete`, `project.locate`, restore of a migration backup, a window-close during a save,
  disk-full during an export, the team gate with a real hash (a gated test build), reverse-reachability of the 14 unwired
  operations, Mac-gated tests (three exist and have not run on a Mac).
- **AI:** the gold set for retrieval and tool choice (120 questions, English plus Telugu and Hindi), poisoned retrieval,
  index rebuild end to end, the per-feature intent-to-tool cases, an 8 GB laptop run, a network-off run on a clean computer,
  the engine start with no Visual C++ runtime.
- **UI and end-to-end:** visual planning, relationships and timeline in Story, the call sheet PDF export step, Idea Vault
  voice note and drag-in, the window-close step, the cloud-folder step, accessibility with Narrator and VoiceOver.
- **Cross-platform:** the whole Mac side.
- **Installation:** install, first run, upgrade, uninstall and reinstall on a clean Windows 11 and a clean Mac; SmartScreen and
  Gatekeeper steps written down; installer size and time; antivirus behaviour.
- **Real-world workflows:** the 2 to 3 day pilot of [the internal test plan](../testing/internal-test-plan.md).
- **Reliability over time:** the soak run (long sessions, sleep and wake, project switching) on a release candidate.

On the **reliability of the existing tests, not just their number**: the tests are written by name after a requirement, they
cover failure and crash cases on purpose, and failures found were fixed in the product. Weaknesses: they all run on the
development machine; the end-to-end driver controls a debug build with a fixed AI setting and an isolated data root;
the visual baseline compares the program with its own earlier screenshots; ten tests are marked ignored (real model, soak,
performance); a few are timing sensitive. Nothing was found that suggests the tests are "green by weakness", but one cannot
tell from numbers alone, which is why the pilot and the clean-machine checks are in the plan.

### 9.E Percentage complete

See Section 10.

### 9.F Broken or disconnected functionality

| Finding | Evidence | Severity |
|---|---|---|
| The window can close before the last 0.4 seconds of typing is saved | `sync.ts` line 41 (400 ms); `lib.rs` lines 500 to 507 call `shutdown` on destroy; no close-request handler | Serious if confirmed |
| 14 operations with no screen caller (see 8.14) | Search of screens | Moderate |
| Mock-ups 164 to 168 show a removed feature; 155 shows roles that do not exist | `HTML_Mockups/` | Moderate for the website, none for the app |
| User manual still contains the five LAN figures | `docs/specs/README.md` | Moderate |
| README feature table was stale (fixed today) | `README.md` | Fixed |
| Three feature rows were stale (fixed today) | `feature-status.md` | Fixed |
| `tauri.conf.json` and the AI policy say "filmmakers" | Files | Minor |
| Local documentation links | `check-links.mjs`: 0 broken in 122 living documents | None |
| Internal links in the 181 mock-ups | Script: all 181 index links and all relative references resolve | None |
| External links | 48 distinct web addresses in the living documents, not checked (no network use in this audit) | Unknown |
| Windows installer needs the internet if WebView2 is missing | `tauri.conf.json` `downloadBootstrapper` | Minor (Windows 11 has it) |
| Stale worktree copies | 39 folders, 4.2 GB, git does not list them | Moderate |
| Team password gate never end-to-end tested in a gated build | E2E builds have no hash | Moderate |
| Orphan and unsupported tooling in `.claude/` | GSD, claude-mem | Minor |

No broken navigation, dead route or placeholder screen was found. The reachability spec clicks all 30 canonical routes.

### 9.G Poorly engineered code that affects reliability or maintainability

Only items that matter. The code is, on the whole, disciplined; these are the exceptions.

1. **Projects can be created in a temporary folder by accident.** `openframe-project-format/src/lib.rs` line 325:
   `documents_dir().unwrap_or_else(std::env::temp_dir)`. If Windows cannot name the Documents folder, new projects go to
   the temp folder, which the system may clean. Rare, but the loss is total. Recommendation recorded as an open owner
   decision: refuse to start with a clear message.
2. **Typing can be lost on abrupt close** (9.F). Single point: `SyncController` waits 400 ms and the shell does not wait for it.
3. **Tool choice by English word overlap.** `toolbox.rs` line 356 scores request words against tool names and descriptions;
   `retriever.rs` has an English stop-word list. For Telugu, Hindi or paraphrase, the right tool is not offered. The effect is a
   weak answer, not a wrong write (writes need approval).
4. **Silent cleanup failures in restore.** `packages/backup.rs` lines 1163, 1186, 1192 use `let _ =` when putting a replaced
   project back or removing the temporary copy. If the rollback rename fails, the person sees the original error while the
   original project sits in a hidden `.replaced-` folder. A verified safety backup exists, so data is recoverable, but nothing
   tells the person where. Moderate.
5. **Four swallowed errors in the screens** (`.catch(() => undefined)` in idea-vault index, screenplay index,
   `ScreenplayView.tsx` line 208, `screenplay/util.ts` line 156). All are "remember where I was" saves. Acceptable.
6. **Read permissions are enforced inside each handler, write permissions in one place.** `Store::mutate` checks the
   capability for every change, which is strong. Reads rely on each query remembering `actor.require(View)`. There is no
   test that calls every query as a viewer. Today the local user is always Owner, so the risk is low; it matters when
   packages and roles grow.
7. **A 150,000-line crate.** Every change relinks every integration test (46 programs). Not a bug; a cost.
8. **Two big mirrors: operations and AI tools.** Any change to a feature's operations needs matching AI catalog work. The
   generated coverage document and tests keep it honest, but it multiplies effort.
9. **`reqwest` is compiled into the Windows shell** although only Mac code uses it (`mac_inputs.rs`; the dependency in
   `src-tauri/Cargo.toml` line 43 is not limited to macOS). Larger build and attack surface for nothing on Windows.
10. **`.cargo/config.toml` forces `sccache` for every build.** A computer without sccache (such as your Mac) fails to build
    until `RUSTC_WRAPPER` is set to an empty string, which CI already does. The Mac bring-up must document this.
11. **A few hard-coded limits** with no setting: 400 ms and 3 s in the editor, 15 idle minutes and 10 minutes start for the
    engine, 7,000 characters of tool text, 8 and 12 steps. Reasonable defaults; tune after the benchmark.
12. **Test-only code paths in production sources** (the data-root switch, deterministic AI, hand-set licence state). They are
    ignored by release builds and unit tests prove it. Acceptable.

Production unwrap and expect calls outside tests: about 60 (counted with a script that skips `#[cfg(test)]`), concentrated
in `openframe-security/src/lib.rs` (9), `openframe-import-export/src/xml.rs` (7), `fountain.rs` (5) and `heuristics.rs` (5).
Reading a few showed they guard values proved just before. They are the first place to look if a crash is ever reported.

### 9.H Temporary fixes that may fail later

| Fix | Where | Acceptable? | What to do |
|---|---|---|---|
| Direct commands that bypass the model | `modules/ai/commands.rs` | Yes, it makes simple things reliable | Keep; add cases as the pilot shows misses |
| "Fall back to the draft in scope" when the model invents a draft name | `ai/tools.rs` | Yes | Watch for wrong-draft complaints |
| Product guide dropped after facts were found | `orchestrator.rs` line 899 | Yes, a workaround for a 1B model misrouting | Revisit if the model changes |
| Test-only environment switches | `lib.rs` | Yes | Keep the unit tests |
| Debug fault injection | `store.rs` line 1068 and others, `cfg(debug_assertions)` | Yes, compiled out of release | Verify once in a release build |
| App-local Visual C++ runtime | AI bundle | Yes, decided | Prove on a clean machine |
| Placeholder `REPLACE_*` store identity | `packaging/msix/store.config.json` | Yes until Chapter 3 | Release check fails closed |
| Draft 1 labels with dashes | Fixed 2026-10-09 | Done | |
| Temporary migration numbers | Gone | Done | |
| Writing baseline ratchet (39 hits left in docs, 202 in crates) | `writing-standard-baseline.json` | Yes, but a debt | Clean when touching files |
| Windows-line-ending churn | Many commits | Needs a permanent fix | `.gitattributes` is present; keep `eol=lf` and check editors |
| Mac code without a Mac | whole Mac side | Not acceptable at release | Chapter 1 bring-up |
| Stale status text | feature rows | Not acceptable | Generate the README table and row text from one source, or add a freshness check |
| `AGENTS.md` untracked old copy | root | No | Owner decides |

### 9.I Better engineering for features that already work

Only changes with a clear benefit; no rewrites of stable code.

1. **Debug level of the development profile** (`Cargo.toml` `debug = 1`): setting it to 0 would remove about 18 GB and speed
   linking. One-line change; needs approval and a before and after measurement.
2. **Join test programs** (46 in the application crate) into a handful. Same coverage, much less linking and disk.
3. **Limit `reqwest` to macOS** in the shell's `Cargo.toml` with a target-specific dependency.
4. **A close-request handler** that awaits all editors' flush (the most valuable small change).
5. **One shared "pending save" registry** in the screens: editors register a flush function and the window close, the
   workspace switch and the project switch all await it. Replaces the current per-editor unmount flushes.
6. **A per-query viewer test** generated from the registry (every query refused for a user without View).
7. **Single source for status words**: have a script print the README table and the feature-row counts, so no one types them.
8. **A startup warning for synced folders** (see Chapter 1).
9. **Make the AI tool selection language-neutral** by indexing tool descriptions with the same embedding model, or by a fixed
   menu of intents; decide after measuring.
10. **Retire or wire the 14 unwired operations.**

### 9.J High-level and low-level design: improvements

**High-level design (how the parts depend on each other).**

```text
 screens --> of_invoke --> registry --> modules --> persistence --> SQLite
                              |            |
                              |            +--> import-export (formats, layout)
                              |            +--> search (derived index)
                              |            +--> ai crate (engines) <-- bundle files
                              +--> entitlement (lock, gate)
                    platform (OS differences) used by persistence, ai, project-format, shell
```

What is good: dependencies point one way (domain at the bottom, shell at the top); the shell holds no rules; the webview has no
power; derived data (search, AI index) is separate from truth; every write has one path.

What could be simpler or safer:

1. **Do not let the AI module keep growing as a mirror of the product.** 28,000 lines is a quarter of the brain. Consider
   generating tool definitions from operation metadata (names, descriptions, schemas) and writing by hand only the
   resolvers and previews. Benefit: fewer places to forget; cost: a design change, so only after the pilot.
2. **Split `openframe-application`** into a few crates along the module lines (screenplay and story; production and
   schedule; packages; ai). Benefit: faster builds and clearer ownership. Risk: large mechanical change. Do it only if build
   times remain painful after the profile and test-program changes.
3. **Make the "save pending" idea explicit** (9.I item 5) so that closing, switching and quitting share one rule.
4. **Keep the registry as the only door**, and add an automated check that no screen string names an operation that does not
   exist (the architecture lint covers part of this; a full cross-check of names would catch typos and the 14 orphans).
5. **A thin "project location" service** that owns where projects live, warns for synced and network folders, and replaces
   the scattered folder choices.

**Low-level design (modules, functions, tables, tools).**

1. The `Store` API (`read`, `mutate`, `with_writer`) is small and well guarded. `with_writer` is the escape hatch (used for
   backup and checkpoint); keep its callers on the lint allow-list.
2. Large files that change often (`schedule/ops.rs`, `story/board.rs`, `packages/import.rs`) could be split by operation
   family; do it when touching them, not as a project.
3. Tables: 81 project tables with consistent columns (id, timestamps, soft-delete). The migration discipline is excellent.
   One watch item: `sys_` tables are untracked by undo on purpose; any new such table needs the same thought.
4. Tools: 253 tools with a 7,000-character budget means at most a few dozen are shown per request. The picker's quality sets
   the assistant's quality, so it deserves its own tests (part of the gold set).
5. Errors: stable codes and human messages are consistent. Keep a test that no message contains a path or SQL.

## 10. How complete is Chapter 1, and how that number was made

### 10.1 Three numbers, not one

You asked for functional completion, testing completion and release readiness, separately. They measure different things,
so one number would mislead.

### 10.2 The method

Source: the 160 product rows of `feature-status.md`. Rows that belong to Chapter 3 (store packages, payment, listings,
certification, legal) and Chapter 2 (the website) are left out. That leaves **142 rows** for Chapter 1.

Each row has an importance word. A "Must ship" row counts three times and a "Should ship" row once, because losing a Must
ship feature hurts more.

Each status gets a **credit** for implementation and a **credit** for verification:

| Status | Implementation credit | Verification credit | Reason |
|---|---|---|---|
| Specified | 0 | 0 | Only written down |
| Designed | 0.10 | 0 | A plan exists |
| Built | 0.60 | 0.20 | Code passes its own tests; no user path proof |
| Reachable | 0.85 | 0.40 | A user can use it; not fully proved |
| Verified | 1.00 | 0.85 | Proved end to end on the development machine; 0.15 is held back until an installer and a clean computer prove it |

The score is the weighted average. The credits are judgments, not facts, so the result was recomputed with kinder and
stricter credits.

### 10.3 The results

| Measure | Base | Kinder credits | Stricter credits | Weights ignored |
|---|---|---|---|---|
| Functional implementation | 80 percent | 85 | 73 | 82 |
| Testing and verification at feature level | 51 percent | 62 | 40 | 55 |

So: **functional completion is about 80 percent (range 73 to 85)**, and **feature-level verification is about 50 percent
(range 40 to 62)**.

Of the 105 Must-ship rows in Chapter 1: 36 Verified, 39 Reachable, 21 Built, 2 Designed, 7 Specified.

### 10.4 Internal release readiness

This is not a percentage of features. It is a checklist of the things that must be true before real writers use the build.
None can be counted until an installer exists.

| Gate | Met? |
|---|---|
| 1. A release installer builds for Windows | No (never built) |
| 2. It installs, runs, upgrades and uninstalls on a clean Windows 11 | No |
| 3. The AI answers with the network off on that clean machine | No |
| 4. A disk image builds for the Mac and installs on a clean Mac | No (no disk image setting, no Mac AI engine, never run) |
| 5. Data-loss risks are closed (close-window flush, synced folders, delete and locate tests) | No |
| 6. A release-build smoke test passes (with and without the team password) | No |
| 7. Performance run within budget on an 8 GB laptop | No (not run on joined code) |
| 8. Keyboard and screen-reader pass done | No |
| 9. Owner has used both builds on own laptops | No |
| 10. Tester pilot finished, Blockers fixed and re-checked | No |

**0 of 10 gates are met.** That is expected at this point, and it is why the work after feature building is the longest part
for a desktop product.

### 10.5 The plan-box view

`npm run plan:status` counts boxes in the build plan: overall 128 of 306 (41.8 percent); the "current goal" boxes (those not
tagged "Later: store publishing") 127 of 223 (57.0 percent). Boxes are not equal in effort: 79 were ticked in one day after
the end-to-end run, and the remaining open boxes include the hard ones (installers, Mac, AI quality, accessibility).

### 10.6 A single blended number

If you need one figure for "how far from writers using it for three days": **about 60 percent, give or take 10 points.** It is
the average of the box view (57 percent) and the feature view (functional 80 and verification 51 average to about 65), and
then lowered slightly because the installer and Mac streams are large and have no head start. If the Mac AI engine, the
close-window fix and the clean-machine installs go smoothly, the real figure moves up fast; if the Mac web view shows layout
problems or the AI proves unusable on an 8 GB laptop, it moves down.

### 10.7 What this number does not say

It does not say the product is 60 percent good. It says 60 percent of the proving is done. Most remaining effort is
verification, installation and the Mac, not new features.

## 11. Risks, ranked

Severity is the harm if it happens. Likelihood is how likely it is that a tester meets it.

### 11.1 Blocks the pilot (fix or decide before testers start)

| Risk | Severity | Likelihood | What reduces it |
|---|---|---|---|
| Typed text lost on abrupt close | High (lost work) | Medium (people close windows) | Close-request flush; end-to-end step |
| Project in a OneDrive-synced Documents folder gets a locked or damaged database | High | High on Windows 11 with default backup | Detect, warn, local default; manual test |
| No installer, no clean-machine proof | High (nothing to test) | Certain | Build with the owner's word; clean machine run |
| Mac cannot run the AI; no disk image | High for Mac testers | Certain | Pin an engine; add `dmg`; Mac bring-up |
| Team password gate untested in a real build | Medium (testers locked out) | Medium | Gated-build test; decide whether to keep |
| Projects created in the temp folder if Documents is unknown | High (total loss) | Very low | Refuse to start |

### 11.2 Serious, fix during the pilot period

| Risk | Severity | Likelihood | What reduces it |
|---|---|---|---|
| AI too slow or unhelpful on an 8 GB laptop | Medium (feature disappoints) | High | Benchmark; message that it is experimental |
| AI requests in Telugu or Hindi fail quietly | Medium | High for Indian-language users | Measure; decide |
| Antivirus quarantines the AI engine or the unsigned installer | Medium | Medium | Document; test on two antivirus setups |
| WebView2 missing on an old or offline machine | Medium | Low on Windows 11 | Offline installer path |
| Visual Studio runtime missing for the AI engine | Medium | Low (app-local files decided) | Clean-machine test |
| Mac web view layout differs from Chromium | Medium | Medium | Mac checklist |
| SmartScreen or Gatekeeper scares a tester | Low (not a defect) | Certain | Install guide |
| Restore replaces a project and the rollback fails silently | Medium | Very low | Report the hidden folder in the error |
| Users edit a project in two windows or computers | Medium | Low | Lock exists; test over a sync client |

### 11.3 Can wait until Chapter 3

Store policy fit for the trial and price; licence server deployment; signing and notarization; updater; the end-user
licence agreement and counsel review; "Report AI output"; a full beta; localization.

### 11.4 Data integrity, corruption and recovery: summary

Strong: transactions on every change; WAL with full sync; integrity check on open; schema screening; verified safety
backup before an upgrade and before a replace; crash marker and checkpoint; interrupted apply undone; atomic file writes;
forced-kill tests at five points; hostile file corpus; soft delete with 30-day retention.

Gaps: the close-window and synced-folder cases above; the restore-on-another-account drill; a disk-full export case on a
real small disk; backup restore not asserted by the migration test; two-window use.

### 11.5 Security summary

Good: the webview cage, one door, argument limits, path confinement, hostile-file limits, loopback-only AI with a key,
hash-checked AI files, human-only apply, redacted logs, no telemetry, supply chain checks (`cargo deny`, audit).
Open: no encryption at rest; internal build password hash can be attacked offline; the per-user install folder is writable
by the user, so hash checks do not stop same-user malware; the web view processes were not traced for network use; the
second security review (area 17) is not done; the unmaintained font crates have a dated exception until 2027-01-06.

### 11.6 Privacy and commercial-release risks

No telemetry and no network use is a real strength. Commercial risks: the Gemma terms need the user agreement to carry the use
restrictions; the unsigned builds look untrustworthy until signed; Apple and Microsoft rules for trials and prices must be
read before building payment; a licence server for the Mac channel contradicts "fully offline" during a trial.

## 12. Why builds and storage grew, and the rules from now on

### 12.1 The facts (measured on 2026-10-09)

| Place | Size |
|---|---|
| `.build-cache/target` | 34 GB (debug: 34 GB, release: 183 MB) |
| of which `debug/deps` | 31 GB: debug files (.pdb) 18 GB in 231 files; 184 test and tool programs 6.3 GB; libraries 5.8 GB |
| `debug/incremental` | 1.7 GB |
| `.build-cache/ai-bundle` | 1.1 GB (hard links to the AI files) |
| `.claude/worktrees` | 4.2 GB in 39 folders |
| `.ai-assets` | 1.1 GB (needed; not in Git) |
| `graphify-out` | 268 MB |
| `node_modules` | 282 MB |
| `.git` | 79 MB |
| Disk | 952 GB total, 498 GB free |

The 273 GB of 2026-10-07 and the 29 to 41 GB per worktree target of 2026-10-08 are from the logs and memory notes.

### 12.2 Why Rust output is large

1. Rust makes a program out of many compiled pieces (one per package, 632 here), and keeps all of them.
2. Debug builds keep information for debuggers. On Windows that information goes into `.pdb` files, one per program. With
   `debug = 1` each test program gets a 150 MB file.
3. Every integration test file is compiled and linked into its own program that contains the whole 150,000-line crate.
   There are 46 for the brain crate.
4. Different settings (debug, release, perf, test) each make a separate copy of the same code.
5. Incremental build data (1.7 GB) speeds rebuilds but is itself large.

### 12.3 Why this project had hundreds of gigabytes

Each parallel agent in its own worktree got its own build folder. Eight worktrees at 30 to 40 GB is 300 GB. The agents also
each ran full builds at the same time on a 16 GB machine, which swapped and made builds take hours. The sccache cache was
configured but, because the build folder path was part of its key, it rarely hit (measured on 2026-10-08: 0 of 30 hits with a
per-worktree variable, 22 of 30 with flags). Its statistics today show no requests.

### 12.4 What is already done

One shared build folder; a build queue (one heavy build at a time, 4 jobs); `rust-lld` for development linking; the Defender
exclusions; a policy file; a rule against `rm -rf` through junctions after the 1,088-file incident.

### 12.5 What is still wasteful, and the fix

| Waste | Fix | Saves |
|---|---|---|
| Debug files for every test program | `debug = 0` for the development and test profiles | about 18 GB |
| 46 test programs | Join into a few per crate | several GB, much less linking |
| Dead worktree copies | Review table, then remove | 4.2 GB |
| `reqwest` built on Windows without use | Make it macOS-only | a little build time |
| 631 other packages | Leave | |
| Unused branches | Review and delete merged ones | tidiness |

Estimated result: the build folder falls from 34 GB to about 12 GB and a full test link from many minutes to a few.

### 12.6 The policy (final, in `development-storage-policy.md` sections 7 and 8)

1. Worktrees only when parallel isolation is truly needed. Default: none.
2. No parallel agents unless you ask; then at most two, with a file-ownership map.
3. One build folder, built through the queue; never a `target` inside a worktree.
4. Test by size of change; full tests once before a merge; end-to-end once per set of areas.
5. Finish, test, merge into `main`, verify on `main`, then remove the worktree.
6. Before removing: commits preserved, working tree clean, branch has no unique commits, node_modules junction unlinked first.
7. Dead folders git does not list are compared with their branch before removal.
8. Alarms at 10 GB and 50 GB; check sizes at the start and end of a session.
9. Keep what is valuable: source, commits, evidence, user data, `.ai-assets`.

### 12.7 Incremental development for small changes

- Run only the crate and test file you touched, through the queue.
- Do not rebuild the desktop shell to test brain logic: use `cargo test -p openframe-application --test <file>`.
- For screens, use `npm --workspace apps/desktop run test -- <name>` (Vitest) and the dev server with hot reload.
- Do not run `npm run verify` for each small change; run `npm run check:governance` (seconds) and the focused tests.
- Run the end-to-end suite only when a set of areas is merged, and use `--only=<spec>` while fixing one spec.

## 13. Documentation audit and the new structure

### 13.1 The volume

Counted on 2026-10-09 (before this report): 148 Markdown files, 70,401 lines, 3.8 MB under `docs/`, which is roughly one
million tokens. No session can or should read that. The size by folder:

| Folder | Files | Lines | Role |
|---|---|---|---|
| `specs` | 14 | 31,757 | Product and engineering specifications |
| `archive` | 9 | 8,723 | Superseded plans and notes |
| `engineering` | 30 | 8,691 | Implementation documents |
| `spec-digest` | 9 | 7,345 | Historical summaries of the specifications |
| `release` | 35 | 3,318 | Launch register and evidence |
| `audits` | 2 | 2,198 | Dated audits |
| `implementation` | 6 (7 with the handoff) | 1,754 | Plan, status, decisions, logs |
| `adr` | 26 | 1,648 | Decisions |
| `website` | 3 | 1,381 | Website plan and two prompts |
| `store` | 2 | 252 | Store drafts |
| New today: product, roadmap, testing, architecture | 9 | about 970 | The chapter-based layer |
| This report | 1 | about 4,000 | The one deep explanation |

Plus the root files (README 10 KB, CONTRIBUTING 13.5 KB, DEVELOPMENT 19 KB, CLAUDE 12.5 KB).

### 13.2 What each document is

Classes: **Current** (authoritative now), **Reference** (useful detail, not the first thing to read), **Verbose** (right
content, too long to read whole), **Outdated** (contradicts the code or a later decision), **Historical** (kept for the
record), **Removable after approval** (no longer relevant).

#### Root

| File | Class | Note |
|---|---|---|
| `README.md` | Current (rewritten today) | Release-state declaration; was stale |
| `CONTRIBUTING.md` | Current | The 15 invariants and the review contract; first lines still say "filmmakers' only copy of their work" (fine as a sentence about trust; wording change optional) |
| `DEVELOPMENT.md` | Reference | Windows setup and commands; no Mac section; tree diagram line fixed today |
| `CLAUDE.md` | Current (rewritten today) | Short instructions, session-start read order |
| `LICENSE-PENDING.md` | Current until the agreement exists | |
| `THIRD_PARTY_LICENSES.md` | Current (generated) | |
| `AGENTS.md` (untracked) | Outdated, removable after approval | An old copy of CLAUDE.md; says the `.dmg` is signed |

#### `docs/` new layer (today)

| File | Class |
|---|---|
| `README.md` | Current (rewritten): map by chapter and by kind |
| `product/vision-and-scope.md` | Current: audience, chapters, claims rule |
| `roadmap/README.md`, `chapter-1`, `chapter-2`, `chapter-3` | Current: the plans |
| `testing/README.md`, `internal-test-plan.md`, `bug-report-template.md` | Current |
| `architecture/README.md` | Current |
| `reports/2026-10-09-...-audit.md` | Dated snapshot (this report) |
| `implementation/handoff.md` | Current, rewritten each session |
| `archive/README.md` | Current: index of history |
| `development-storage-policy.md` | Current (amended today) |

#### `docs/specs` (14 files, 31,757 lines)

| File | Lines | Class | Note |
|---|---|---|---|
| Mega PRD | 5,745 | Current, verbose | What and why; LAN parts superseded |
| Functional Specification | 5,838 | Current, verbose | Behaviour and acceptance; 13 LAN sections superseded |
| UX/UI Specification | 4,560 | Current, verbose | Screens; light-only rule amended by ADR-0014; overlay AI panel replaced by docked panel |
| Domain and Data | 1,955 | Current | Objects and identities |
| Screenplay and Production Depth (Revision 1) | 3,355 | Current, newest | Wins for the areas it covers |
| Security, Privacy, Data Ownership | 3,468 | Current, verbose | 22 LAN sections superseded; telemetry wording governed by ADR-0021 |
| Offline Collaboration | 1,723 | Current in part | Packages normative; live LAN superseded |
| Import and Export | 1,471 | Current | |
| AI Specification | 609 | Current | |
| Local AI Runtime and Model Management | 275 | Outdated in part, already bannered | Describes the download and manifest design; its banner (2026-10-08) says ADR-0025 replaced it, and that text is history |
| Agentic AI and RAG spec | 2,128 | Current | Section numbers cited in code |
| ESD Technical Architecture | 388 | Current with a status note | |
| Engineering Package Index | 215 | Current with amendments | Some "locked decisions" are under amendment |
| `README.md` | 27 | Current | Status of each spec |

These are the longest documents and the most expensive to read. They are lookups: use `graphify query` or `grep` for a
section. A future cleanup could add a short "what changed since this was written" table at the top of each; not needed now.

#### `docs/adr` (26 files)

25 decision records plus the register. **Current.** Nine contain superseded text that is marked in place. The register is
the right entry point. The ADRs for the chapters' decisions (0017, 0018, 0022, 0023, 0024, 0025) are all consistent after the
2026-10-08 amendments. One cosmetic point: ADR-0017's title still says "three release files"; its body carries an inline
note that ADR-0023 and ADR-0024 changed this to four, and the register shows the amendment.

#### `docs/engineering` (30 files, 8,691 lines)

| Files | Class |
|---|---|
| 11 physical database, 12 project file format, 13 command architecture, 14 IPC contract | Current reference |
| 16 search, 17 autosave undo recovery, 18 retrieval index, 20 threat model, 21 performance budgets | Current reference |
| 22 QA strategy, 23 release and migration, 24 Windows packaging, 25 CI/CD, 26 observability | Current reference; 22 and 24 predate the bundled AI and the internal builds in places |
| 27 Microsoft Store, 36 entitlement and trial, 38 Mac App Store trial | Chapter 3 reference |
| 28 cross-platform plan (1,283 lines) | Verbose plan; state is in 37 |
| 29 desktop HLD and LLD (1,011 lines) | Verbose; target architecture; partly done |
| 30 appearance, 32 screenplay contract, 33 production contract | Current contracts |
| 31 AI-assisted development controls (1,003 lines) | Rationale for controls; verbose |
| 34 UX laws checklist, 35 writing standard | Current |
| 37 platform boundary, 39 packages restore drill | Current |
| `ai-tool-coverage.md` (961 lines) | Generated by a test; never edit |
| `AGENT_BRIEF.md` | Historical, partly outdated; cited by DEVELOPMENT.md |
| `SECURITY_REVIEW_2026-09-30.md` | Evidence snapshot; cited by test comments |

#### `docs/implementation` (7 files)

| File | Class | Note |
|---|---|---|
| `masterplan.md` (about 650 lines) | Current | The detailed checklist. Verbose but structured; chapter map added today |
| `feature-status.md` (296 lines, very long lines) | Current | The inventory. Three rows were stale and are fixed |
| `owner-decisions.md` | Current | Seven rows added, eight open rows added today |
| `progress-log.md` | Current for the newest entries; Historical below "Earlier entries" | |
| `reconciliation-log.md` (440 lines) | Reference | Conflicts and name maps; new section today |
| `handoff.md` | Current | New |

#### `docs/release`, `store`, `website`, `audits`, `archive`, `manual`

| Path | Class |
|---|---|
| `release/launch-readiness-register.md` (1,528 lines) | Chapter 3 reference, verbose |
| `release/evidence/` (M0 to M14, release, two session folders) | Evidence; folders named with old codes, most M2 to M14 are empty indexes |
| `store/listing.md`, `store/privacy-policy.md` | Chapter 3 drafts with placeholders |
| `website/README.md`, `prompts/` | Chapter 2 notes and unverified prompts |
| `audits/` (two dated audits) | Historical evidence |
| `archive/` (6 plan files, 2 notes) | Historical; indexed |
| `manual/*.docx`, `*.pdf` | Needs rebuilding; source not in the repository as text |

### 13.3 The structure that fits this project

```text
Why we do it          docs/product/vision-and-scope.md
What to do next       docs/roadmap/  (one page per chapter)
How it works          docs/architecture/README.md  (+ the engineering contracts)
What is true now      docs/implementation/feature-status.md + handoff.md
How we prove it       docs/testing/  + docs/release/evidence/
Why we decided        docs/adr/
Rules for people      CLAUDE.md, CONTRIBUTING.md, development-storage-policy.md, writing standard
What was decided      docs/implementation/owner-decisions.md
Reference             docs/specs/, docs/engineering/
History               docs/archive/ (index), spec-digest/, audits/, progress-log older entries
```

This is not a fixed count of files. It follows the six questions a person asks: why, what next, how, is it true, why that
decision, and what happened before.

### 13.4 What was done today and what was deliberately not done

Done: the new layer above; the documentation map rewritten by chapter and by kind; the archive index; owner decisions and
the build plan updated; three stale rows and the README table corrected; the spec-digest marked historical; the storage
policy extended; `CLAUDE.md` rewritten; a handoff file created.

Not done, and why:

- **No file was moved or deleted.** `spec-digest`, `AGENT_BRIEF.md` and `SECURITY_REVIEW_2026-09-30.md` are cited by scripts,
  tests or ADRs, so moving them means editing code comments and the writing check's exempt list. That belongs in a change
  with a green test run. The archive index lists exactly what to move and what to update.
- **The specifications were not edited.** They are long, authoritative and mostly right. Superseded parts already carry
  banners (the Local AI Runtime spec, the LAN sections, the Engineering Package Index's amended decisions).
- **The user manual was not touched.** Its source is not in the repository as text.
- **`AGENTS.md`, worktree folders and the extra tooling were left alone** until you decide.

### 13.5 Contradictions between documents, code and your vision

| Contradiction | Resolution |
|---|---|
| Audience "filmmakers" in code, documents, installer text; your words say writers, screenwriters and production teams | Vision page; README and CLAUDE.md updated; app text change waits for approved wording |
| "AI optional" (your first brief) versus "AI bundled in every installer" (your later message and ADR-0025) | Bundled is the decision; settled |
| `CLAUDE.md` said the `.dmg` is signed and notarized | Unsigned for now (ADR-0024 amendment); fixed |
| "Mac later on a real Mac" | You have a Mac now; moved into Chapter 1 |
| "Not installed on purpose: claude-mem" versus enabled in your settings | Recorded; your decision |
| README table said core features were in development | Rewritten from the feature status file |
| Feature rows CORE-004, CORE-005, CORE-015 | Corrected with evidence |
| Europe price 79 euros (first message) versus 89 euros | 89; recorded |
| 2,999 or 3,000 rupees | Asked in the open questions |
| Store beta of 8 to 15 people for 3 to 4 weeks (area 17) versus your 2 to 3 day pilot | The pilot is Chapter 1; the long beta is Chapter 3 |
| Mock-ups with removed features | Barred from the website |
| ADR-0017 title "three release files" versus ADR-0024 "four" | Inline amendment note in ADR-0017; the register shows it |
| The Local AI Runtime spec describing a download | Already bannered as superseded by ADR-0025 |
| "Session 1, 2, 3 with nine areas each" | No such grid exists; 26 areas, lanes A to G, milestones M0 to M16 |

### 13.6 Link and reference check

`node scripts/check-links.mjs` reports OK for 122 living documents (relative links). Evidence and archive folders are exempt
by design and may point to renamed files. The 181 mock-ups have no broken internal links. 48 external web addresses in the
documents were not checked (no network use). References to code paths in documents (for example "line 407 of lib.rs") are
not checked by any script and will drift; the new documents name files, and line numbers only in this dated report.

### 13.7 How to keep the documents true

1. One source per fact; link, do not copy.
2. Generate numbers (plan status, feature counts, AI coverage); never type them.
3. Rewrite the handoff each session; append to the log.
4. When a change lands, update the rows listed in `docs/README.md` section 8 in the same change.
5. Dated reports stay dated; they are not maintained.
6. Mark history; do not delete it.
7. Ask a reader to check one document per week against the code (the audit found stale text within four days of writing).

## 14. The Claude Code setup, context rot and a simpler working method

### 14.1 What is configured

| Layer | What is there | Loaded when |
|---|---|---|
| User instructions | `~/.claude/CLAUDE.md` (231 bytes: a graphify note) | Every session, every project |
| Project instructions | `C:\OpenFrame\CLAUDE.md` (rewritten today, 12.5 KB) | Every session in this project |
| Old copy | `C:\OpenFrame\AGENTS.md` (untracked, 13 KB, for another tool) | Not by Claude Code; stale |
| Memory | 10 files in `~/.claude/projects/C--OpenFrame/memory/` | The index every session; files when relevant |
| User settings | Default model Haiku, effort high, output style Concise, 11 plugins, six plugin sources, a permission list with many entries from unrelated Python work | Every session |
| Project settings | `.claude/settings.json` (git-ignored): hooks for graphify and GSD | Every session here |
| Plugins | figma, ecc, frontend-design, impeccable, superpowers, claude-code-setup, security-guidance, caveman, agent-skills, claude-security, claude-mem | Every session; each adds skills, agents, commands and hooks |
| GSD framework | `.claude/agents` (33), `.claude/commands/gsd`, `.claude/hooks` (12), `.claude/get-shit-done`; installed 2026-10-06 from an npm package marked "no longer supported" | Hooks run on Write, Edit and Bash |
| Session brief | `.claude/session-2-brief.md` (git-ignored; instructions to the eight agents of 2026-10-07) | Only if an agent was told to read it |
| Worktrees | `.claude/worktrees/` (39 dead folders) | Never loaded; costs disk |
| Knowledge graph | `graphify-out/` (22,062 nodes) with a hook that reminds Claude to query it first | On Bash, Grep, Read, Glob |

### 14.2 What each concept does in OpenFrame (plain words)

- **CLAUDE.md** is the project's standing orders. It said: what we are building, the control documents, the security
  standard, writing standard, git flow, storage rules, session start and end. It was 14.8 KB and included history ("found on
  2026-10-05 ..."), which is context that does not help the next task.
- **Memory** holds your preferences and lessons: no push, no installer build, E2E once at the end, production-grade bar,
  build strategy. Memory is good for things the repository cannot show (your preferences), bad for things it can (plans).
- **Skills** are saved procedures such as "brainstorm before building" or "review Rust code". They are listed in every
  session's context, so a large number of skills uses context before any work starts. This session's list has several
  hundred names.
- **Subagents** are helper Claudes. In OpenFrame they did the area work in parallel, each with its own worktree.
- **Hooks** are automatic checks. Useful ones here: the graphify reminder; the commit validator. Costly one: a "fact-forcing
  gate" that blocks the first write to every file until facts are stated (27 refusals in this session).
- **Plugins** bring all of the above in bundles. Several overlap (superpowers, agent-skills and ecc all offer planning,
  review and testing workflows).
- **Worktrees** are the Git feature behind parallel agents (Section 3.9).
- **MCP servers** connect to outside tools (GitHub, Figma, Notion, Vercel, Slack, Gmail, Calendar and others). Most need
  sign-in and are unused by this project.

### 14.3 Repeated or contradictory instructions found

| Item | Problem | Fix applied or proposed |
|---|---|---|
| `CLAUDE.md` versus `AGENTS.md` | Two copies; the second is stale (signed `.dmg`) | Rewrote CLAUDE.md; AGENTS.md waits for your word to delete |
| "Four files, `.dmg` signed" versus "unsigned for now" | Same file contradicted itself | Fixed |
| "claude-mem not installed" versus enabled | Instruction does not match reality | Recorded; your decision |
| Memory "Mac later" versus your Mac now | Stale | Memory updated |
| Rules repeated in memory and CLAUDE.md (no push, no installer) | Duplication | Kept short in both; memory is the owner's voice, CLAUDE.md is the rule |
| Long history inside CLAUDE.md | Context cost | Moved out; history lives in the log and archive |
| Output style "Concise" plus caveman mode (very short replies) versus your wish for simple, step-by-step teaching | Compression fights explanation | Switch caveman off for explanation and documents; keep it for status lines |
| Default model set to the smallest model for all work | A deep audit or design task benefits from a stronger model | Your choice per task |

### 14.4 Which tools are genuinely useful here

| Tool | Verdict | Reason |
|---|---|---|
| graphify | Keep | Cheap orientation; the hook reminds Claude |
| superpowers (brainstorming, verification, debugging) | Keep, use one workflow at a time | Good habits |
| ecc reviewers (rust, typescript, silent failures, tests) | Keep | The plan asks for them before ticking a box |
| security-guidance, ecc security review | Keep | Parsers, paths, AI, licence |
| caveman | Optional | Saves tokens in chat; hurts teaching |
| agent-skills | Review | Overlaps superpowers and ecc |
| impeccable, frontend-design | Keep for Chapter 2 | Website design |
| figma | Keep only if you use Figma for the website | |
| claude-security | Keep for a later scan | |
| claude-mem | Your decision | It records tool calls into a database; the earlier decision was no |
| GSD (33 agents, hooks) | Recommend removing | Not part of the workflow, never mentioned in plans, runs hooks on every edit, package unsupported |
| Many unrelated MCP connectors | Leave unauthorised | No cost until used |

Nothing was removed. Removal is your decision, made with the list above.

### 14.5 What should always need a human approval

- Building any installer or package. Pushing to GitHub. Opening or merging a pull request. Changing repository settings.
- Deleting worktrees, branches, build folders, or anything under user data.
- Editing a shipped migration. Changing a frozen file.
- Changing the AI model, its engine or its approval rule.
- Signing, notarization, payment, price, licence terms.
- Setting or reading the team password.
- Marking an owner decision as decided.

Routine and safe without asking: reading, local commits on a feature branch, running focused tests through the queue,
editing documents on a documentation branch, running the governance scripts.

### 14.6 What caused context rot in this project

1. **Instructions that grew.** CLAUDE.md reached 15 KB and kept past events. A new session paid for them and some were wrong.
2. **Documents that lag.** A session reading a stale row believes it. (This audit made that mistake once; Section 0.5.)
3. **Sessions that ran very long.** Session 2 spanned two days and many agents; summaries replaced detail.
4. **Many places for the same fact.** Status in the README, feature rows, the log, memory and the plan.
5. **Whole-document reading.** 5,800-line specifications read to "get oriented".
6. **A huge tool and skill list** loaded into every session.
7. **Parallel agents** each with their own partial view, merged later.
8. **Counts typed by hand**, which drift.

### 14.7 The operating model now in place

1. **A fixed read order** at the start: CLAUDE.md, the handoff page, the chapter page, one work item. Written in CLAUDE.md.
2. **One handoff page**, rewritten (not appended) at the end of each session: state, next three items, blockers, "do not".
3. **One work item per session**, small, with objective, files, verification and acceptance (the chapter pages).
4. **Truth in the code.** Documents are checked against code before action; numbers come from scripts.
5. **Short instructions.** CLAUDE.md about 150 lines; history in the archive and log.
6. **Memory only for what the repository cannot show.** Pruned when stale.
7. **Orientation by query.** `graphify query`, `grep`, the feature row; not wholesale reading.
8. **A session size rule.** After about 60 tool calls on one topic, write the handoff and continue fresh. Compact at a clean
   boundary, never in the middle of an edit.
9. **No parallel agents by default.** If you ask for them, a file-ownership map first.
10. **Keep the enabled tools few.** Review plugins every month; remove those unused.

### 14.8 Proposed next steps for the setup (all need your yes)

1. Remove the GSD framework folders and its hooks from `.claude/`, or keep them knowingly.
2. Decide on claude-mem.
3. Delete the untracked `AGENTS.md`.
4. Add an exemption for documentation paths to the fact-forcing gate, or switch the gate off for documentation work.
5. Trim the user permission list of unrelated Python entries.
6. Keep one planning plugin family (superpowers or ecc), not three.
7. Set the default model by task: a stronger model for design and audits, the fast one for mechanical edits.

## 15. The plan from here

The full plan is in [the Chapter 1 page](../roadmap/chapter-1-complete-test-validate.md), the testing strategy in
[testing/README.md](../testing/README.md) and the pilot in [internal-test-plan.md](../testing/internal-test-plan.md). This
section is the summary with the reasoning.

### 15.1 What is completed work that needs no more attention

Story Board and Idea Vault core; the screenplay engine and formats; themes; schedule and Day Out of Days; call sheets,
budget and templates; packages and backup logic; the AI safety path; the migration and recovery machinery; the governance
scripts. These need only regression protection and the clean-machine proof.

### 15.2 Partly done, needs integration or proof

Visual planning; the team password gate; the Mac side; the accessibility story; AI quality; performance on small laptops;
the manual; the 14 unwired operations; project delete and locate.

### 15.3 Failed or insufficiently tested

Nothing failed in the last full run (30 spec files, 158 tests, 0 failed, per the log). Insufficient: everything outside the
development machine.

### 15.4 Important missing work

An installer; the Mac engine and disk image; the close-window fix; the synced-folder protection; the gold-set measurement;
the end-user licence agreement (Chapter 3).

### 15.5 Repeated or redundant tasks

The per-branch end-to-end runs (stopped); the many documents that restate the plan (now linked); the digests (historical);
eight agents each running clippy (stopped).

### 15.6 Tasks that belong to later chapters

All store, payment, signing, listing, certification, launch and long-beta boxes (83 "Later" boxes), "Report AI output",
the licence server deployment, the updater.

### 15.7 The work items in dependency order

Size: S under half a day, M one to two days, L three to five days of focused work (guesses).

| # | Work item | Stream | Size | Needs first | Verification | Accepted when |
|---|---|---|---|---|---|---|
| 1 | Flush typing before the window closes | Protect work | S | none | New end-to-end step: type, close within 100 ms, reopen; Rust shutdown-during-save test | Passes 20 times on Windows and Mac |
| 2 | Projects in synced folders | Protect work | M | owner wording | Unit test with fake sync root; 10-minute manual typing test on a OneDrive laptop | Warning shows for four clients; no lock error |
| 3 | `project.delete` and `project.locate` tests | Protect work | S | none | Two integration tests, one Home step | Tests fail when the code is broken on purpose |
| 4 | Migration backup restores | Protect work | S | none | Open each backup, compare rows | Fails on an empty backup |
| 5 | Documents folder missing | Protect work | S | owner choice | Test of the chosen behaviour | Never writes to temp |
| 6 | Prove the daily-use Reachable rows | Core | L | 1 to 3 | One end-to-end step each, restart and kill checks | Rows read Verified with test names |
| 7 | Visual planning end-to-end | Core | M | none | One spec for three tabs | Canvas, sketch and drag reorder covered |
| 8 | The 14 unwired operations | Core | S | none | Decision per operation | Each wired, kept AI-only on purpose, or removed |
| 9 | Keyboard and screen-reader pass | Core | M | 6 | Checklists with Narrator and VoiceOver | Findings fixed or listed |
| 10 | Performance run on joined code | Core | M | none | `npm run perf` and an 8 GB laptop run | Within budgets |
| 11 | Replace "filmmakers" in product text | Core | S | owner sentence | Search finds no old wording | Text approved and changed |
| 12 | Gold set and metrics | AI | L | none | 120 questions, English plus Telugu and Hindi, real model | Report in evidence; owner approves thresholds |
| 13 | Non-English decision | AI | M | 12 | Numbers | Owner decides keep, add, or menu |
| 14 | Hardware benchmark | AI | M | none | `ai-benchmark.mjs` on three computers | Minimum hardware published |
| 15 | Remaining AI proofs | AI | M | 12 | Index rebuild step, poisoning corpus, intent-to-tool cases | Added to evidence |
| 16 | Team password decision | Windows | S | owner | n/a | Decided |
| 17 | Release-build smoke test | Windows | M | owner's word to build | Spec on the release executable, gated and ungated | Passes |
| 18 | Build and clean-machine install | Windows | M | 1 to 5, 16, owner's word | Checklist on two machines | All steps pass; evidence saved |
| 19 | Mac toolchain and sccache override | Mac | S | none | `cargo check` and `npm run dev` on the Mac | Steps in DEVELOPMENT.md |
| 20 | Pin the macOS AI engine | Mac | M | owner approval | Hash lock; AI state "Verified" on the Mac | AI answers on the Mac |
| 21 | Add the `dmg` target | Mac | S | 19 | Image builds unsigned | Installs after Gatekeeper steps |
| 22 | Mac behaviours checklist | Mac | M | 19, 20 | Spell check, dialogs, shortcuts, quit, folders | Checklist complete |
| 23 | Mac test automation choice | Mac | S | 22 | Trial of the WebdriverIO service or a written manual suite | Decision recorded |
| 24 | Pilot kit | Pilot | S | 18, 21 | Installers with checksums, guide, template | Sent to testers |
| 25 | Pilot run and fix loop | Pilot | L | 24 | Reports, severities, fixes, re-checks | Blockers closed; summary archived |
| 26 | Lower the debug level; join test programs | Hygiene | M | owner approval | Before and after size and time | About 18 GB saved, same tests pass |
| 27 | Review dead worktrees and old branches | Hygiene | S | owner approval | Table per folder | Nothing removed before approval |
| 28 | A freshness check for status text | Hygiene | M | none | Script prints README and row counts from one source | Drift fails a check |

Items 1 to 5 come first because they protect writers' work; 6 to 11 and 12 to 15 can follow in either order; 16 to 18 are the
Windows route; 19 to 23 the Mac route and can start at the same time as 6; 24 and 25 need both installers; 26 to 28 run beside
the rest, with 26 and 27 early because they save hours.

### 15.8 Required for internal testing, versus safe to wait

| Required | Can wait |
|---|---|
| Items 1 to 5, 16 to 25 | Signing and notarization |
| The Mac route if a Mac tester takes part | Store packages and payment |
| Accessibility basics (item 9) for the main workspaces | Updater |
| Performance on an 8 GB laptop | End-user licence agreement text (needed before public release) |
| An honest "experimental" label for the AI | Report AI output |
| The pilot install guide | Long beta, localization |

### 15.9 The testing strategy in short

- Layers: unit, integration, migration fixtures, kill tests, hostile files, Vitest, governance, real runtime, end-to-end,
  performance, installer on a clean computer, the tester pilot.
- Rule: small changes get small tests; the full set once before a merge; end-to-end once per set of areas.
- Weak spots to close: the release build, window close, synced folders, delete and locate, accessibility, retrieval quality, the
  Mac, the gated build.
- Mac: manual checklist first; automation after.
- Evidence: a result file for every claim; "not run" written when not run.

### 15.10 The tester pilot in short

Four to eight testers (writers, screenwriters, a producer, a scheduler; at least one on a Mac, one on an 8 GB laptop, one
who writes in an Indian language), three days: write and import; plan and organise; stress and finish. Every report uses a
template and gets a severity; Blockers stop that tester until fixed; fixes are re-checked in the next build; installation
warnings are logged as restrictions, not defects. The details and install guides are in the plan.

### 15.11 Chapter 2 in short

A small static site on Cloudflare Pages: Home, Features (Verified only), Tutorials, Onboarding, Support, Privacy, Terms,
Updates. The screens are captured from the real program with the demo project so they are faithful; mock-ups 155 and 164 to
168 are never used. Starts when the screens are stable.

### 15.12 Chapter 3 in short

Read the stores' current rules first (Microsoft allows apps to use their own payment system; Apple's trial guideline allows a
price-tier-zero non-consumable trial; prices come from store price tiers). Wire the real calls behind the existing
interfaces, write the licence agreement with counsel, sign, notarize, build packages, validate, submit. No Chapter 3 code
until Chapter 1 is done.

## 16. Decisions and questions for you

### 16.1 Decisions only you can make

| # | Question | My recommendation |
|---|---|---|
| 1 | Should testers keep the team password gate? | Remove it for the pilot if the files go only to people you trust; keep it if you want a barrier. Whatever you choose, test that build |
| 2 | Default folder for new projects, and the wording for the OneDrive warning | Offer a local folder when Documents is synced; never move existing projects |
| 3 | The new sentence for "filmmakers" in the product text | "A desktop studio for writers, screenwriters and production teams" |
| 4 | Apple silicon AI engine: processor or Metal | Benchmark both on your Mac, choose the faster that fits 8 GB |
| 5 | Final prices: 3,000 or 2,999 rupees | Check the store price tiers first (Chapter 3) |
| 6 | What to do with non-English AI requests | Decide after the gold-set numbers |
| 7 | Delete `AGENTS.md`, remove GSD, decide on claude-mem | Delete AGENTS.md and GSD; claude-mem your call |
| 8 | Approve lowering the debug level and joining test programs | Yes; it saves about 18 GB and link time |
| 9 | Review the 39 dead worktree folders and 31 branches | Yes, with a table first |
| 10 | Say when the first internal installer may be built | After items 1 to 5 |
| 11 | Who are the pilot testers, and when | Your list; two to three days each |
| 12 | Support address, domain, company email | Needed for Chapter 2 |

### 16.2 What I need to know from you (questions)

1. Do you want the app text itself ("filmmaking" in the installer description and the AI policy) changed now, or only
   after the pilot?
2. For the Mac pilot, is one Apple silicon Mac enough, and what macOS version is it?
3. Will any tester use a Windows laptop with 8 GB of memory?
4. Should the AI be shown to testers as "experimental" in the interface?
5. Do you want me to continue with items 1 to 5 next, one at a time?

## 17. A guided tour of the code

This section follows real actions through the real files, then walks each crate, the database and the list of operations.
Line numbers are from 2026-10-09 and will drift; file names will not.

### 17.1 Six actions followed from click to disk

#### Action 1: you type a letter in the screenplay

1. **The editor shows it.** `apps/desktop/src/workspaces/screenplay/editor/ScreenplayEditor.tsx` (987 lines) is a
   ProseMirror editor. ProseMirror changes the page in the browser at once, which is why typing feels instant.
2. **The editor marks itself dirty.** `SyncController.markDirty()` in `editor/sync.ts` sets a flag and starts a 400 ms timer.
   Every new key restarts the timer.
3. **The timer fires.** `flush()` reads the page, compares it with the last saved copy (`diffSnapshots`) and gets a short
   list of edits: "element 17: text changed to ...".
4. **The edits are sent.** The `send` function in `ScreenplayEditor.tsx` (line 613) calls
   `call("screenplay.apply_edits", { draftId, ops })`.
5. **The bridge.** `ipc/client.ts` calls Tauri's `invoke("of_invoke", ...)`.
6. **The shell.** `of_invoke` (`lib.rs` line 65) moves the work to a background thread and calls `core.dispatch`.
7. **The registry.** `registry.rs` line 482 finds `screenplay.apply_edits`, checks the argument shape, asks the entitlement
   service, and calls the handler.
8. **The module.** `modules/screenplay/edit.rs` (`apply_edits`, line 870) validates each edit (is the draft locked? is the
   element type allowed? is the text too long?) and calls `Store::mutate` with a "typing" coalesce key.
9. **The pipeline.** `store.rs` line 433: permission `Edit`, a write transaction, row changes, undo capture, search update,
   commit.
10. **The answer.** The new sequence number goes back to the editor, which records it as the last saved state.
11. **The event.** `DataChanged` goes to every screen that reads screenplay tables, such as the scene navigator and the page
    count. Your own editor ignores its own echo.

What can go wrong and what happens:

| Case | Result |
|---|---|
| The draft is locked | Rust refuses with `permission.locked_draft`; the editor puts the old text back and shows a message |
| The disk is full or the drive was removed | `storage.*` error; the editor keeps your text, shows "error" status and retries every 3 seconds; the save indicator says so |
| The program is killed between key and timer | Up to 400 ms of typing is lost (the case this audit adds to the plan) |
| The program is killed after the commit | Nothing is lost; the crash marker offers recovery on the next open, and the autosaved state is already in the database |
| Two edits race | The single write connection serialises them |

#### Action 2: you create an act on the Story Board and then undo it

1. A dialog collects the title. `useCommand("story.create_act")` in the screen calls the operation.
2. The same path as Action 1 reaches `modules/story/` which calls `Store::mutate` with the label "Create act".
3. In the transaction: a new row in `story_act` with a UUIDv7 id and a position; a search document; an Activity entry.
4. The triggers recorded the insert; `push_undo` stores one undo step "Create act".
5. The event tells the board to refresh; a toast appears with an **Undo** button.
6. **Undo** calls the undo operation (`history` module). `Store::undo` takes the actor's latest undo step and reverses the
   recorded row changes in a new transaction. If someone else changed the same rows since, it refuses with
   `undo.conflict` instead of overwriting.
7. The act disappears from the board when the next event arrives. A redo step now exists.

#### Action 3: you ask the assistant "Create a task to call the location manager"

1. The AI panel (`apps/desktop/src/ai/`) calls `call("ai.ask", { text, scope })`.
2. `modules/ai/orchestrator.rs` `ask` (line 1270) validates length (2,000 characters), checks the bundle state, makes sure the
   engines are running (starts them if needed), builds the scope and calls `run_agent` (line 817).
3. `run_agent` offers tools for this request. The words "create" and "task" match the task proposal tool.
4. `commands::direct_call` recognises a plain command ("Create a task to ...") and skips the model.
5. `toolbox::run_tool_unlocked` runs `propose_task`, which checks the person's permission, builds the operation list
   (`tasks.create` with a title) and returns a `ChangeSetDraft` with a preview.
6. The loop ends, the draft is stored as a Change Set (state: Proposed), and the panel shows **Proposed Changes**:
   "Create task: call the location manager".
7. Nothing in the project has changed. You press **Apply Changes**.
8. `accept` (`change_set.rs` line 472) requires a local human actor. In one transaction it marks the Change Set accepted and
   writes the journal row (state "applying").
9. Each operation is dispatched through the registry with the normal checks. The last step marks the Change Set applied and
   closes the journal. The whole thing is one undo step ("AI: create task").
10. If the program died after step 8, the next project open compensates by undoing the applied operations and shows a note in
    Activity.

If the request had been "Rename Arjun to Arun in all scenes", the model would have to pick a character tool and the rename
tool; the preview would list affected scenes; the stale check would refuse it if a scene changed after the proposal.

#### Action 4: the power fails while you write, and you reopen the project

1. While the project was open, `recovery/session.json` existed (written in `open_at`, `project.rs` line 828).
2. After the power cut it still exists. Committed changes are in `project.sqlite` and its `-wal` file; SQLite replays the
   log on the next open.
3. You open the project. `open_at` calls `previous_unclean_session` and sees the marker.
4. The integrity check passes. The recovery offer is built: the time of the last activity, whether `checkpoint.sqlite`
   exists and its time.
5. The screen shows the recovery dialog (`RecoveryDialogs.tsx`). `describeRecovery` decides the wording: "newer" only if
   there are changes after the last confirmed save.
6. You choose **Keep my latest work** (nothing to do; it is already there) or **Return to last saved version**
   (the checkpoint copy replaces the working database; the newer state is kept in the project's backups folder).
7. A new marker is written and the session goes on.

#### Action 5: you export the screenplay to PDF

1. The export dialog (`features/interchange/ExportScreenplayDialog.tsx`) lists drafts, options (title page, revision marks)
   and a file name. The native save dialog gives a path; the path is passed to Rust as text.
2. `screenplay.export` validates the destination (`openframe_security::validate_output_file`: not a protected OpenFrame
   file, not on a network path, not a reserved name, parent exists).
3. `openframe-import-export` builds the document model, runs `layout::layout` (one engine for the page view, the print
   preview and the PDF), embeds subset fonts for Indian scripts, and writes the PDF bytes.
4. `exports::deliver` (`exports/mod.rs` line 378) writes to a temporary file and renames it over the destination (atomic),
   so a failure leaves the previous file untouched. It records an Activity entry and remembers the output path so the
   screen can later "show in folder" without sending a path back.
5. The result (pages, size) returns; a toast offers **Show in folder**, which calls `of_reveal_path` with kind `exported`;
   Rust accepts it only because it remembers writing that file.

#### Action 6: you back up a project and restore it on another computer

1. **Create backup.** The backup dialog calls `packages.create_backup` with a destination. It is a long-running task: Rust
   returns a task id at once, the screen shows progress from `Task` events, and a cancel flag is polled.
2. The backup builder copies the database through SQLite's online backup, adds the assets and a manifest (project id, app
   version, schema version, counts, a hash), zips them with limits, writes `*.partial`, flushes and renames.
3. On the other computer, **Restore Backup or Package** (Home) calls `packages.import_project`.
4. The package is inspected without extraction: manifest first, version checks (a package from a newer app is refused),
   entry names and sizes (zip-slip, zip bomb, duplicate and short-name collisions are rejected).
5. It is extracted to a staging folder, the database is integrity-checked and schema-screened, and the identity is compared
   with any existing project of the same id.
6. If there is a collision you choose: open as a copy with a new identity, or replace (after a verified safety backup of the
   existing project, `backup.rs` around line 1120).
7. The restored project is renamed into place and opened like any project. Private notes belong to the person restoring.
8. Any failure removes the staging folder and leaves everything as it was (tested for interrupted and corrupt backups).

### 17.2 The crates, one by one

#### `openframe-domain` (794 lines): the shared vocabulary

`auth.rs`: five roles (Owner, Editor, Commenter, Viewer, Export-only) and 14 capabilities (View, Edit, SoftDelete,
PermanentDelete, Comment, ResolveComments, LockOrFinalize, ManageProject, ManagePermissions, Export, CreatePackage, Import,
ApplyChangeSet, UseAi). `Role::allows` is a plain table. `ActorOrigin` says where a request came from: Local, Exchange (a
package) or Ai. `error.rs`: `AppError` and about 30 builders. `ids.rs`: UUIDv7 identities. `enums.rs`: statuses and kinds.
No file access, no database, so it is safe everywhere.

#### `openframe-persistence` (1,604 lines): SQLite rules

`lib.rs`: connection policy, integrity and foreign key checks, the untrusted-schema screen, online backup, checkpoint.
`migrate.rs`: plan (fresh, up to date, upgrade, too new), apply in one transaction, checksums, plain error mapping.
`undo.rs`: installs the capture triggers and replays steps. `rows.rs`: validated helpers for updates and ordering.

#### `openframe-project-format` (525 lines): the folder

Layout paths, the manifest, the session marker, stale staging cleanup and the single-writer lock (a file lock through
`fs4`; a second OpenFrame sees "this project is open elsewhere"). Also the default folders (the Documents fallback noted in
the audit).

#### `openframe-security` (2,000 lines): hostile input

`lib.rs`: `confine` (a relative path may not escape its root), `validate_relative`, reserved Windows names, network
and device paths, output file checks, `web_link` (only http and https), redaction of user names, paths, emails and tokens in
logs. `archive.rs`: safe ZIP reading and writing with limits on entries, declared and real sizes, duplicates, short names and
links.

#### `openframe-platform` (2,432 lines): the operating system

`dirs.rs` (folders), `atomic.rs` (the one file-replace used everywhere), `process.rs` (Job Object or process group, no console
window), `power.rs` (sleep and wake), `shell.rs` (open, reveal, open link), `spelling.rs` (system dictionaries), `keys.rs`
(key labels), `drives.rs` (network drive detection), `paths.rs` (long paths), `identity.rs` (package identity), `exists.rs`
(fast file check). The only place allowed to contain operating-system code.

#### `openframe-entitlement` (5,782 lines): trial, licence, team password

`lib.rs`: states and rules. `adapters.rs`: one adapter per channel. `notices.rs`: trial reminders and one-time screens.
`mac/`: the Mac App Store policy, bridge contract, server contract and 1,600 lines of attack tests (all with fakes).
`team_gate/`: the Argon2id password gate and its build-time hashing.

#### `openframe-import-export` (17,697 lines): formats, layout, fonts

`fdx.rs`, `fountain.rs`, `docx.rs`, `pdf_import.rs`, `text.rs`, `heuristics.rs` (reading); `layout.rs` (1,303 lines, the page
engine), `pdf_export.rs`, `pdfgen.rs`, `pdf_fonts.rs`, `fonts.rs` (writing and fonts); `report.rs` (1,545 lines, every
production document); `xml.rs` (bounded XML); `rules.rs` (the element rule table also read by the editor); `samples.rs`.

#### `openframe-search` (2,443 lines): the derived index

`db.rs` (the disposable file), `docs.rs` (documents and chunks), `embed.rs` (the embedding interface), `vector.rs`
(`sqlite-vec`), `graph.rs` (relations), `rank.rs` (reciprocal rank fusion).

#### `openframe-ai` (5,144 lines): the engines

`bundle.rs` (the lock file, hashing, states), `supervisor.rs` (start, health, restart, idle stop, log redaction),
`manager.rs` (the façade used by the brain), `client.rs` (the loopback HTTP client), `engines.rs` (safety records about
stray engines), `store.rs` (small writable records), `selection.rs` and `hardware.rs` (suitability hints), `rt.rs` (a
private async runtime), `model.rs` (the model adapter interface).

#### `openframe-application` (150,467 lines): the brain

`core.rs` (AppCore: databases, session, tasks, events), `registry.rs` (operations and metadata), `store.rs` (the pipeline),
`schema.rs` (the migration lists), `events.rs`, `util.rs`, `modules/` (Section 6.3).

#### `openframe-test-support` (989 lines)

`TestEnv`, a fake embedder, stress helpers. Used only by tests.

#### The Tauri shell (2,867 lines)

`lib.rs` (startup, four commands, logging), `asset_scope.rs` (what the picture protocol may read), `distribution.rs`
(channel detection, WebView2 check), `platform/` (Mac StoreKit, revalidation; empty stubs on Windows), `build.rs` (embeds the
team password hash, compiles the Swift bridge on Mac).

### 17.3 The database, table by table

All tables use text UUIDv7 ids, created and updated times, and soft-delete columns where they apply. Names by migration:

| Migration | Tables |
|---|---|
| 0001 core | project, project_member, season, episode, deleted_item, asset, project_file_folder, project_file, snapshot, project_note, task, comment, private_note, template, sys_activity, sys_undo, sys_view_state, search_doc |
| 0002 idea vault | vault_folder, vault_collection, vault_item, vault_item_collection, vault_item_tag |
| 0003 story | story_act, story_sequence, story_beat, story_scene_card, story_character, story_character_relationship, story_character_card_link, story_attachment |
| 0004 screenplay | screenplay, screenplay_draft, screenplay_scene, screenplay_element, review_round, screenplay_history_point, screenplay_character_link, sys_screenplay_sync |
| 0005 production | production_source, catalog_item, catalog_alias, breakdown_element, breakdown_dismissed, production_scene_state, location, location_photo, cast_member, crew_member |
| 0006 visual | moodboard, moodboard_item, storyboard, storyboard_panel, shot |
| 0007 schedule | shooting_schedule, shooting_day, schedule_strip, schedule_marker, schedule_warning_ack, call_sheet, side, production_report, budget_snapshot, budget_line |
| 0008 AI | ai_request, ai_result, ai_tool_invocation, change_set |
| 0009 packages | sys_package_log, sys_import_session, review_queue_item, exchange_review_record |
| 0010 performance | indexes only |
| 0011 AI apply journal | sys_ai_apply_journal |
| 0012 story card and vault research | columns only (long description, research flag) |
| 0013 screenplay engine | screenplay_custom_element, screenplay_page_preset, screenplay_revision, screenplay_lock_baseline; rebuilt screenplay_element |
| 0014 production depth | breakdown_category_style, availability_window; contact columns |
| 0015 schedule timeline | dood_override; day time columns |
| 0016 call sheets and budget | columns only (actual amount, call sheet status words, document version) |

Global Idea Vault (`vault.sqlite`): deleted_item, asset, sys_activity, sys_undo, sys_view_state, search_doc, and the five vault
tables; 4 migrations. App database (`app.sqlite`): sys_settings, sys_local_profile, sys_recent_project, sys_global_template,
sys_appearance_pref, sys_user_theme; 2 migrations.

Rules: `sys_` tables are system tables not tracked by undo. No `ON DELETE CASCADE` (deletes are explicit and soft). No BLOB
columns in canonical tables (files live in `assets/`, referenced by id and path). Display numbers are never identity.

### 17.4 The 483 operations, by module

This is everything the brain can do, as registered names (`module.action`). "AI-reachable" operations are those a tool
may use; "hidden" ones never reach the assistant, each for a stated reason. Names come straight from the generated
coverage document.

#### Appearance (8 operations: 0 AI-reachable, 8 hidden)

Hidden from the assistant:

```text
appearance.check_theme, appearance.delete_theme, appearance.export_theme, appearance.get,
appearance.import_theme, appearance.rename_theme, appearance.set_mode, appearance.set_theme
```

#### Breakdown (20 operations: 13 AI-reachable, 7 hidden)

AI-reachable:

```text
breakdown.accept_many, breakdown.add_element, breakdown.apply_suggested_update, breakdown.mark_reviewed,
breakdown.reject, breakdown.remove_element, breakdown.scene, breakdown.scene_changes, breakdown.scenes,
breakdown.set_archived, breakdown.set_complete, breakdown.suggest, breakdown.update_element
```

Hidden from the assistant:

```text
breakdown.accept, breakdown.categories, breakdown.quick_reuse, breakdown.reset_category_styles, breakdown.reuse,
breakdown.set_category_style, breakdown.source_anchor
```

#### Budget (10 operations: 10 AI-reachable, 0 hidden)

AI-reachable:

```text
budget.add_line, budget.create, budget.delete_line, budget.delete_snapshot, budget.get, budget.get_snapshot,
budget.mark_reviewed, budget.save_snapshot, budget.update, budget.update_line
```

#### Call Sheets (11 operations: 10 AI-reachable, 1 hidden)

AI-reachable:

```text
callsheets.add_section, callsheets.create, callsheets.delete, callsheets.finalize, callsheets.get,
callsheets.list, callsheets.new_revision, callsheets.refresh, callsheets.refresh_preview, callsheets.update
```

Hidden from the assistant:

```text
callsheets.attach
```

#### Cast & Crew (31 operations: 27 AI-reachable, 4 hidden)

AI-reachable:

```text
cast.assign, cast.create, cast.delete, cast.list, cast.set_archived, cast.update, crew.create, crew.delete,
crew.list, crew.set_archived, crew.update, notes.create, notes.delete, notes.get, notes.list, notes.set_pinned,
notes.update, tasks.create, tasks.delete, tasks.list, tasks.set_done, tasks.update, templates.create,
templates.delete, templates.get, templates.list, templates.update
```

Hidden from the assistant:

```text
cast.set_photo, tasks.relatable_types, templates.apply, templates.copy
```

#### Comments & Private Notes (11 operations: 11 AI-reachable, 0 hidden)

AI-reachable:

```text
comment.create, comment.delete, comment.list, comment.reopen, comment.reply, comment.resolve, comment.update,
private_note.create, private_note.delete, private_note.list, private_note.update
```

#### Diagnostics (2 operations: 0 AI-reachable, 2 hidden)

Hidden from the assistant:

```text
diagnostics.export_bundle, diagnostics.preview
```

#### Exports (11 operations: 0 AI-reachable, 11 hidden)

Hidden from the assistant:

```text
breakdown.export_report, budget.export_summary, callsheets.export_pdf, moodboard.export_pdf,
reports.export_report, schedule.export_dood, schedule.export_schedule, shot.export_list, sides.export_pdf,
story.export_board, storyboard.export_sheet
```

#### Files (12 operations: 8 AI-reachable, 4 hidden)

AI-reachable:

```text
files.create_folder, files.delete, files.delete_folder, files.list, files.move, files.rename,
files.rename_folder, files.update_notes
```

Hidden from the assistant:

```text
files.add, files.asset, files.export_copy, files.relink
```

#### History (7 operations: 3 AI-reachable, 4 hidden)

AI-reachable:

```text
history.activity, trash.list, trash.restore
```

Hidden from the assistant:

```text
history.info, history.redo, history.undo, trash.purge
```

#### Idea Vault (28 operations: 19 AI-reachable, 9 hidden)

AI-reachable:

```text
vault.add_tags, vault.add_to_collection, vault.create, vault.create_collection, vault.create_folder,
vault.delete, vault.delete_collection, vault.delete_folder, vault.get, vault.list, vault.move_to_folder,
vault.overview, vault.remove_from_collection, vault.remove_tag, vault.rename_collection, vault.rename_folder,
vault.send_to_story, vault.set_pinned, vault.update
```

Hidden from the assistant:

```text
vault.add_files, vault.copy_to_store, vault.ingest_bytes, vault.purge, vault.relink, vault.restore,
vault.set_research_flag, vault.story_targets, vault.trash_list
```

#### Import & Export (7 operations: 0 AI-reachable, 7 hidden)

Hidden from the assistant:

```text
interchange.draft_scenes, interchange.export_preview, interchange.sources, screenplay.export,
screenplay.import_apply, screenplay.import_preview, screenplay.samples
```

#### Licence and trial (8 operations: 0 AI-reachable, 8 hidden)

Hidden from the assistant:

```text
entitlement.buy, entitlement.notice, entitlement.record_notice, entitlement.restore_purchase,
entitlement.start_trial, entitlement.status, team_gate.status, team_gate.unlock
```

#### Offline AI (11 operations: 0 AI-reachable, 11 hidden)

Hidden from the assistant:

```text
ai.ask, ai.change_set.accept, ai.change_set.get, ai.change_set.recheck, ai.change_set.reject, ai.diagnostics,
ai.history, ai.index_rebuild, ai.index_status, ai.status, ai.stop_runtime
```

#### Packages (28 operations: 4 AI-reachable, 24 hidden)

AI-reachable:

```text
packages.log, packages.records, packages.review_queue, packages.sessions
```

Hidden from the assistant:

```text
packages.apply_import, packages.backup_preview, packages.cancel_import, packages.create_backup,
packages.exchange_preview, packages.exchange_scenes, packages.exchange_sources, packages.export_exchange,
packages.export_project, packages.import_project, packages.inspect_package, packages.open_exchange,
packages.queue_attach, packages.record_delete, packages.record_view, packages.review_comment,
packages.review_delete_comment, packages.review_export_response, packages.review_get, packages.review_list,
packages.review_open, packages.session, packages.session_view, packages.undo_import
```

#### Production (30 operations: 25 AI-reachable, 5 hidden)

AI-reachable:

```text
catalog.add_alias, catalog.create, catalog.delete, catalog.find_matches, catalog.get, catalog.list,
catalog.remove_alias, catalog.replace_in_scenes, catalog.set_archived, catalog.update, locations.create,
locations.delete, locations.get, locations.list, locations.remove_photo, locations.replace,
locations.set_archived, locations.set_status, locations.update, production.apply_update, production.drafts,
production.overview, production.preview_update, production.set_source, production.source
```

Hidden from the assistant:

```text
availability.set, catalog.set_image, locations.add_photos, locations.move_photo, production.references
```

#### Project (26 operations: 6 AI-reachable, 20 hidden)

AI-reachable:

```text
project.deleted_items, project.get_settings, project.home, project.members, project.set_status,
project.update_settings
```

Hidden from the assistant:

```text
app.cancel_task, app.info, app.save_state, app.set_display_name, project.close, project.create, project.current,
project.delete, project.duplicate, project.list_recent, project.locate, project.open, project.recovery_state,
project.remove_recent, project.rename, project.resolve_recovery, project.save, project.set_archived,
project.set_last_location, project.set_pinned
```

#### Schedule (31 operations: 29 AI-reachable, 2 hidden)

AI-reachable:

```text
schedule.acknowledge_change, schedule.add_marker, schedule.confirm_removal, schedule.create, schedule.create_day,
schedule.daily, schedule.delete, schedule.delete_day, schedule.delete_marker, schedule.dood,
schedule.duplicate_day, schedule.get, schedule.move_day, schedule.move_marker, schedule.move_strip,
schedule.move_strips, schedule.reconcile, schedule.set_day_date, schedule.set_day_notes, schedule.set_day_target,
schedule.set_day_times, schedule.set_off_day, schedule.set_status, schedule.set_strip_estimate,
schedule.set_strip_pages, schedule.set_strip_start, schedule.suggestions, schedule.update_marker,
schedule.update_settings
```

Hidden from the assistant:

```text
schedule.decide_warning, schedule.set_dood_override
```

#### Screenplay (59 operations: 40 AI-reachable, 19 hidden)

AI-reachable:

```text
screenplay.characters, screenplay.clear_character_link, screenplay.compare, screenplay.complete_review,
screenplay.create, screenplay.create_scene, screenplay.custom_elements, screenplay.delete_draft,
screenplay.delete_element, screenplay.delete_scene, screenplay.document, screenplay.history_points,
screenplay.insert_element, screenplay.locate, screenplay.lock_draft, screenplay.lock_summary,
screenplay.move_scene, screenplay.new_draft, screenplay.overview, screenplay.page_setup, screenplay.pagination,
screenplay.rename, screenplay.rename_draft, screenplay.replace_all, screenplay.restore_draft,
screenplay.restore_history_point, screenplay.review_rounds, screenplay.revision_changes,
screenplay.revision_pages, screenplay.scene_hub, screenplay.set_character_link, screenplay.set_current_draft,
screenplay.start_review, screenplay.start_revision, screenplay.story_reference, screenplay.update_element,
screenplay.update_review, screenplay.update_revision, screenplay.update_scene, screenplay.update_title_page
```

Hidden from the assistant:

```text
screenplay.apply_edits, screenplay.create_custom_element, screenplay.delete, screenplay.delete_custom_element,
screenplay.delete_page_preset, screenplay.merge_scene, screenplay.move_element, screenplay.page_view,
screenplay.renumber_scenes, screenplay.restore_omitted_scene, screenplay.save_page_preset,
screenplay.set_scene_number, screenplay.set_view_state, screenplay.split_scene, screenplay.title_page_preview,
screenplay.unlock_draft, screenplay.update_custom_element, screenplay.update_page_setup, screenplay.view_state
```

#### Search (2 operations: 1 AI-reachable, 1 hidden)

AI-reachable:

```text
search.query
```

Hidden from the assistant:

```text
search.rebuild
```

#### Sides & Reports (10 operations: 10 AI-reachable, 0 hidden)

AI-reachable:

```text
reports.delete, reports.generate, reports.get, reports.list, reports.save, sides.create, sides.delete, sides.get,
sides.list, sides.preview
```

#### Story (59 operations: 52 AI-reachable, 7 hidden)

AI-reachable:

```text
story.apply_order, story.assign_story_day, story.board, story.build_preview, story.build_screenplay, story.card,
story.card_to_beat, story.character, story.character_usage, story.characters, story.clear_character_image,
story.convert_beat, story.create_act, story.create_beat, story.create_card, story.create_character,
story.create_episode, story.create_relationship, story.create_season, story.create_sequence, story.delete_act,
story.delete_character, story.delete_episode, story.delete_items, story.delete_relationship, story.delete_season,
story.delete_sequence, story.duplicate_episode, story.duplicate_items, story.link_character_card, story.move_act,
story.move_episode, story.move_items, story.move_season, story.order_preview, story.park_items,
story.relationships, story.remove_attachment, story.restore_from_parking, story.series,
story.set_character_archived, story.set_time_note, story.timeline, story.unlink_character_card, story.update_act,
story.update_beat, story.update_card, story.update_character, story.update_episode, story.update_relationship,
story.update_season, story.update_sequence
```

Hidden from the assistant:

```text
story.add_attachment, story.card_display, story.set_card_display, story.set_character_image, story.set_collapsed,
story.set_current_episode, story.view_state
```

#### Visual Planning (49 operations: 37 AI-reachable, 12 hidden)

AI-reachable:

```text
moodboard.add_link, moodboard.add_note, moodboard.add_vault_image, moodboard.create, moodboard.delete,
moodboard.delete_items, moodboard.get, moodboard.list, moodboard.rename, moodboard.update_item,
moodboard.update_notes, shot.copy_planning, shot.create, shot.create_panel, shot.delete, shot.duplicate,
shot.get, shot.list, shot.move, shot.reorder, shot.update, storyboard.add_panel, storyboard.create,
storyboard.create_shot_from_panel, storyboard.delete, storyboard.delete_panels, storyboard.get,
storyboard.link_shot, storyboard.list, storyboard.move_panel, storyboard.rename, storyboard.reorder_panel,
storyboard.set_scene, storyboard.update_panel, visual.mark_scene_reviewed, visual.scene_characters,
visual.scenes
```

Hidden from the assistant:

```text
moodboard.add_image_data, moodboard.add_images, moodboard.arrange_item, moodboard.move_items, moodboard.reorder,
moodboard.resize_item, moodboard.suggestions, shot.attach_panel, shot.detach_panel, shot.set_reference_image,
storyboard.set_panel_visual, visual.vault_images
```

#### Writing (12 operations: 0 AI-reachable, 12 hidden)

Hidden from the assistant:

```text
app.reset_shortcuts, app.set_shortcuts, app.shortcuts, writing.check_spelling, writing.dictionary,
writing.dictionary_add, writing.dictionary_remove, writing.navigator_columns, writing.proofing,
writing.set_proofing, writing.spelling_suggestions, writing.suggestion_sources
```

### 17.5 The screens, folder by folder

File names with their line counts (without test files). A workspace is one folder under `apps/desktop/src/workspaces`.

#### workspaces/activity (1 files, 92 lines)

```text
index.tsx 92
```

#### workspaces/breakdown (9 files, 1573 lines)

```text
dialogs.tsx 455, index.tsx 288, ElementsPanel.tsx 221, ScriptPane.tsx 145, SceneList.tsx 140, highlight.ts 111,
ReviewViews.tsx 105, categoryStyle.tsx 75, filterPref.ts 33
```

#### workspaces/call-sheets (12 files, 3127 lines)

```text
CallSheetEditor.tsx 1003, shared/ScheduleDialogs.tsx 690, shared/ScheduleViews.tsx 416, shared/Board.tsx 313,
CallSheetList.tsx 225, shared/Strip.tsx 147, shared/ScheduleFilterBar.tsx 128, shared/scheduleFilter.ts 96,
shared/calendar.ts 37, shared/KeepAnywayDialog.tsx 32, shared/open-call-sheet.ts 27, index.tsx 13
```

#### workspaces/files (3 files, 779 lines)

```text
index.tsx 464, FileDetails.tsx 177, fileUtils.ts 138
```

#### workspaces/idea-vault (10 files, 3088 lines)

```text
dialogs.tsx 863, index.tsx 740, ItemDrawer.tsx 495, model.ts 276, ItemViews.tsx 217, ItemCard.tsx 200,
DeletedView.tsx 105, LeftPanel.tsx 95, api.ts 80, media.ts 17
```

#### workspaces/notes (4 files, 596 lines)

```text
TaskEditor.tsx 217, index.tsx 210, NoteEditor.tsx 134, useDebouncedSave.ts 35
```

#### workspaces/production (20 files, 7086 lines)

```text
tabs/moodboards.tsx 963, tabs/storyboards.tsx 961, tabs/shots.tsx 946, tabs/cast-crew.tsx 568,
tabs/catalog.tsx 474, tabs/budget.tsx 432, tabs/locations.tsx 418, tabs/sides.tsx 274, tabs/dood.tsx 266,
components/SourceDialogs.tsx 251, tabs/schedule.tsx 235, tabs/daily.tsx 206, tabs/reports.tsx 205,
visual/SketchDialog.tsx 185, tabs/overview.tsx 176, components/depth.tsx 160, visual/shared.tsx 133,
components/fields.tsx 100, index.tsx 67, visual/canvas.ts 66
```

#### workspaces/project-home (2 files, 387 lines)

```text
index.tsx 316, StatusMenu.tsx 71
```

#### workspaces/screenplay (23 files, 7046 lines)

```text
editor/ScreenplayEditor.tsx 988, ScreenplayView.tsx 787, panels.tsx 655, pagesetup.tsx 376, lock.tsx 312,
drafts.tsx 305, misc.tsx 302, editor/commands.ts 300, editor/suggest.ts 292, editor/model.ts 289,
ReviewView.tsx 283, editor/spelling.ts 276, index.tsx 273, picker.tsx 237, editor/suggestPlugin.ts 221,
SceneNavigator.tsx 190, CompareView.tsx 184, editor/sync.ts 171, editor/schema.ts 168, util.ts 158, api.ts 148,
editor/rules.ts 71, editor/find.ts 60
```

#### workspaces/settings (10 files, 1905 lines)

```text
Templates.tsx 389, Shortcuts.tsx 345, SupportDiagnostics.tsx 240, Writing.tsx 237, OfflineAi.tsx 178,
ProjectSettingsForm.tsx 157, index.tsx 125, Licence.tsx 111, BreakdownCategories.tsx 94, About.tsx 29
```

#### workspaces/story (15 files, 4641 lines)

```text
Characters.tsx 674, BoardView.tsx 472, index.tsx 437, drawers.tsx 425, BuildDialog.tsx 364, Episodes.tsx 354,
OutlineView.tsx 300, dialogs.tsx 299, items.tsx 238, model.ts 233, dnd.tsx 232, Timeline.tsx 176,
CardDisplay.tsx 175, actions.ts 155, ui.ts 107
```

#### workspaces/trash (1 files, 183 lines)

```text
index.tsx 183
```

#### app (21 files, 2661 lines)

```text
home/AppHome.tsx 648, shell/Shell.tsx 420, shortcuts.ts 270, shell/SaveStatus.tsx 199,
shell/RecoveryDialogs.tsx 179, home/NewProjectDialog.tsx 170, stores.ts 129, shell/SearchOverlay.tsx 127,
home/format.ts 86, toast.tsx 86, home/DisplayNameDialog.tsx 58, routes.tsx 53, App.tsx 46,
useRememberLocation.ts 44, home/homeState.ts 28, shell/ErrorBoundary.tsx 27, home/testUtils.tsx 24,
navigate.ts 21, useAppEvents.ts 21, shell/extensions/packages.tsx 14, shell/WorkspaceUnavailable.tsx 11
```

#### features/appearance (3 files, 478 lines)

```text
AppearancePanel.tsx 289, ThemeImportDialog.tsx 174, ThemePreview.tsx 15
```

#### features/export (4 files, 754 lines)

```text
buttons.tsx 355, ExportDialog.tsx 277, model.ts 108, index.ts 14
```

#### features/interchange (6 files, 1302 lines)

```text
ImportScreenplayDialog.tsx 495, ExportScreenplayDialog.tsx 458, model.ts 208, PrintPreview.tsx 87, api.ts 48,
index.ts 6
```

#### features/licensing (4 files, 962 lines)

```text
LicenceScreens.tsx 679, rules.ts 131, licence.ts 92, TrialCorner.tsx 60
```

#### features/packages (11 files, 1963 lines)

```text
ImportExchangeDialog.tsx 357, ReviewViewer.tsx 259, QueueAndHistory.tsx 249, ExportExchangeDialog.tsx 239,
BackupDialogs.tsx 211, api.ts 178, OpenProjectDialog.tsx 177, parts.tsx 96, store.ts 75, index.ts 74,
PackagesOverlay.tsx 48
```

#### features/team-gate (1 files, 131 lines)

```text
TeamGate.tsx 131
```

#### ai (4 files, 775 lines)

```text
Conversation.tsx 287, model.ts 236, ProposedChanges.tsx 129, AiPanel.tsx 123
```

#### design-system (1 files, 404 lines)

```text
index.tsx 404
```

#### theme (3 files, 385 lines)

```text
engine.ts 291, useAppearance.ts 49, BrandLogo.tsx 45
```

## 18. The tests, file by file

Counted on 2026-10-09. Each Rust integration test file is its own program. The first line of the file's own comment is shown
as its purpose.

### openframe-ai (5 test files)

| File | Lines | Purpose |
|---|---|---|
| `bundled.rs` | 624 | The bundled Offline AI end to end (ADR-0025): AI works on first launch with no download and no setup, the pinned hashes are checked before the first e |
| `real_runtime.rs` | 130 | Validation with the REAL bundled runtime and models (ignored by default; needs the ~1.1 GB of pinned files staged by `node scripts/prepare-ai-assets.m |
| `sidecar_log_redaction.rs` | 172 | The sidecar log never receives secrets or personal data (log redaction test, Security baseline E7). |
| `soak_sidecar.rs` | 400 | Sidecar soak (build plan item soak tests and failure injection). |
| `supervisor.rs` | 566 | Sidecar supervisor lifecycle against a fake loopback server. |

### openframe-application (46 test files)

| File | Lines | Purpose |
|---|---|---|
| `ai.rs` | 1233 | Offline AI Assistant acceptance tests (FSD §42, FSD-AI-001..027, AI-AC-001..025; Local AI Runtime spec §10–§14), driven through the public operation r |
| `ai_agent.rs` | 1050 | Bounded agent loop, composite Change Sets and the human-only approval boundary (agentic spec §5, §7, §29–§30, §40–§43; contracts C3/C4). |
| `ai_apply_recovery.rs` | 392 | Crash-safe application of multi-operation AI Change Sets (ADR-0019; build plan item crash-safe AI changes, forced stop at every apply step; feature AI |
| `ai_index_stress.rs` | 239 | index shutdown stress: intelligence index shutdown stress (AI/RAG spec §32–§33, CONTRIBUTING §2 invariant 11). Repeated open, close, switch and rebuil |
| `ai_real_model.rs` | 210 | The assistant with the REAL bundled model (ignored by default; ADR-0025). Needs the pinned files staged by `node scripts/prepare-ai-assets.mjs`: |
| `ai_retrieval.rs` | 1264 | Local intelligence index + hybrid retrieval (AI/RAG spec §8–§22, §31–§33, §42; contract C2). Driven through the real mutation pipeline and the public |
| `ai_retrieval_perf.rs` | 384 | §34 performance measurements for the intelligence index and hybrid retrieval on a generated, feature-length fixture (deterministic content). |
| `ai_stale_proposals.rs` | 752 | stale proposal tests: stale-proposal tests per domain (AI spec §20, AI-AC-014, ADR-0013, ADR-0019; CONTRIBUTING §2 invariant 2). |
| `ai_tool_coverage.rs` | 314 | Operation inventory and AI tool coverage (agentic spec §6.1, §27, §28, §39, §49.3–4). |
| `ai_toolbox.rs` | 1781 | Toolbox (contract C3) tests: universal read and proposal tools, strict arguments, permission filtering, privacy and the approval boundary |
| `appearance.rs` | 850 | Appearance and themes (build plan area "Appearance and themes"; ADR-0014; features THEME-001 to THEME-008): preferences in the app database, My Themes |
| `comments.rs` | 498 | Comments (FSD §23, §94; FSD-SCRIPT-029..032) and Private Notes (FSD-SCRIPT-033; Security §8, §10.2) through the public operation registry. |
| `core_pipeline.rs` | 866 | End-to-end tests of the core pipeline through the public operation registry (the same path the UI, exchange-package imports and AI Change Sets use). |
| `diagnostics.rs` | 637 | Support diagnostics bundle (build plan item export diagnostics, feature CORE-015, ADR-0021) end-to-end through the operation registry: the bundle is u |
| `e2e_flows.rs` | 1193 | Cross-module end-to-end flows, driven through the public operation registry exactly as the desktop UI does (every call is `op name + JSON args`). |
| `entitlement.rs` | 891 | Licence and free-trial status in the app (build plan area "Payment and free trial in all three channels", items "Shared entitlement interface" and "No |
| `exports.rs` | 1866 | Document exports (FSD §20.7–20.8, §31.5, §32.7, §33.6, §38.9, §58, §60, §109.6, §110–111, §122–123; Import/Export §3, §6) end-to-end through the |
| `failure_injection.rs` | 1216 | failure injection failure injection, storage and process (build plan item failure injection; doc 22 §7). |
| `forced_kill.rs` | 648 | Forced-kill matrix (build plan item forced stop tests; feature CORE-003, CORE-004, CORE-005, XFER-008, XFER-012). |
| `idea_vault.rs` | 1377 | Idea Vault acceptance tests (FSD §5, §88; FSD-IDEA-001..020) through the public operation registry. |
| `interchange.rs` | 866 | Screenplay import/export through the application layer (FSD §19–20, §60, §117, §122–123; FSD-SCRIPT-038..050; Import/Export spec §4–§5, §23, §25). |
| `migration_failures.rs` | 502 | Migration failure fixtures (build plan item migration failure cases; doc 23 §4 rule 7; feature CORE-005). |
| `migration_fixtures.rs` | 961 | Migration fixture corpus (build plan item migration test files; feature CORE-005, PLAT-005; doc 23 §4 rule 7). |
| `notes.rs` | 821 | Notes, Tasks and Templates (FSD §59, §106, §113, §158, §159) through the public operation registry. |
| `packages.rs` | 2939 | Backup, Full Project Package and Exchange/Review/Response package acceptance tests (FSD §44.5–44.8, §45, §47, §50, §118, §121; FSD-COL-001/002, |
| `pdf_regression.rs` | 401 | PDF regression for the six production documents (masterplan area "Call sheets, budget, templates, samples and exports", box "PDF regression tests"): c |
| `perf.rs` | 1444 | Performance budgets (docs/engineering/21-performance-budgets.md) measured on the "Feature-120" stress project built by `openframe_test_support::stress |
| `production.rs` | 1576 | Production module acceptance tests: Production Source, Breakdown, Suggestions, Catalog, Locations, Cast & Crew (FSD §26–30, §53–54, §96–100, |
| `production_depth.rs` | 1434 | Breakdown, catalog, locations, cast and crew depth (masterplan area 13; Production depth contract sections 2, 3 and 9; depth spec sections 23, 24 and  |
| `property_invariants.rs` | 990 | Property tests for the core editing invariants (build plan item property tests). |
| `recently_deleted_purge.rs` | 388 | Recently Deleted keeps items for 30 days, then removes them (owner decision 2026-10-07, ADR-0012 amendment; build plan area 9 box "Recently Deleted 30 |
| `schedule.rs` | 1705 | Shooting Schedule, Daily View, Call Sheets, Sides, Reports and Budget: end-to-end through the operation registry (FSD §35–39, §56–58, §104–105, |
| `schedule_depth.rs` | 1170 | Shooting schedule depth (masterplan area "Shooting schedule and Day Out of Days"; contract docs/engineering/33-production-depth-contract.md section 4; |
| `screenplay.rs` | 1508 | Screenplay workspace acceptance tests (FSD §15–17, §21–24, §92–95; FSD-SCRIPT-001..037 except import/export), through the public operation |
| `screenplay_delete.rs` | 482 | Deleting a whole screenplay (owner decision 2026-10-08, ADR-0012 amendment; masterplan area 11 box "Several screenplays per project"). A screenplay go |
| `screenplay_engine.rs` | 1174 | Screenplay engine through the operation registry (the same path as the UI): page setup and presets, title page, header and footer, explicit page |
| `screenplay_formats.rs` | 383 | Screenplay file formats through the operation registry (the same path as the UI): |
| `security_review.rs` | 507 | Regression tests for the 2026-09-30 internal security review (docs/engineering/SECURITY_REVIEW_2026-09-30.md). Each test names its finding ID. |
| `soak.rs` | 1234 | soak tests soak suite (build plan item soak tests; docs/engineering/22-qa-test-strategy.md §7). |
| `story.rs` | 1612 | Story workspace acceptance tests (FSD §7–14, §18, §25, §89–91; FSD-STORY-001..030), driven through the public operation registry like the UI. |
| `story_cards.rs` | 991 | Story Board card redesign and hardening (Story Board and Idea Vault area of the build plan; Screenplay and Production Depth Specification sections 21  |
| `team_gate.rs` | 198 | Team password gate of the internal builds through the application core (ADR-0024; build plan area 20 box "Team password gate"). The hash is made here  |
| `trial_first_run.rs` | 675 | Trial reminders, the buy button and the once-only screens after a purchase (owner mocks 182 to 198; `docs/engineering/36-entitlement-and-trial.md` sec |
| `vault_research.rs` | 197 | Idea Vault research flags (FSD 107.5 and 107.6; mock 045; feature row "Selection, context menu, collections, research flags, Recently Deleted"). |
| `visual.rs` | 1419 | Visual planning: Moodboards, Storyboards, Shot Lists (FSD §31–34, §55, §101–103). Tests go through the public operation registry, like the UI does. Sc |
| `writing_comfort.rs` | 711 | Writing comfort acceptance tests (build plan area "Writing comfort"; features SCRIPT-004, SCRIPT-017, SCRIPT-018, SCRIPT-019, SCRIPT-020), through the |

### openframe-import-export (16 test files)

| File | Lines | Purpose |
|---|---|---|
| `docx.rs` | 228 | DOCX import/export (FSD §19.5, §20.6; IEX-007) and archive safety. |
| `element_rules.rs` | 25 | The element rule table is shared with the editor: this test writes `apps/desktop/src/workspaces/screenplay/editor/elementRules.json` from |
| `fdx.rs` | 158 | Final Draft import/export (FSD §19.7, §20.4; IEX-005). |
| `fdx_suite.rs` | 323 | FDX round-trip suite (Screenplay and Production Depth Specification §37; feature XFER-003): thirteen script classes, each with a source file, the expe |
| `fonts_manifest.rs` | 69 | Supply chain of the bundled PDF fonts (ADR-0016): every font file the app compiles in is listed in `fonts/fonts.json` with its SHA-256, size, version  |
| `fountain.rs` | 176 | Fountain import/export: golden output and round-trip stability (IEX-006, FSD §117). |
| `hostile_corpus.rs` | 1672 | hostile file collection hostile-input corpus for the screenplay parsers (FDX, Fountain, DOCX, PDF). |
| `multilingual.rs` | 181 | Multilingual round trip (ADR-0016; feature SCRIPT-UNI-001): a script that mixes Telugu, Hindi, Tamil, Malayalam, Kannada and English, with conjuncts,  |
| `pdf.rs` | 316 | Screenplay PDF export (FSD §20.3, §123) and best-effort PDF import (FSD §19.4, IEX-004): our own PDF parses back with the same scene count. |
| `pdf_golden.rs` | 557 | Golden PDF image-compare regression for screenplay PDF export (build plan item PDF image comparison, feature XFER-010; ADR-0008 deterministic PDF). |
| `performance.rs` | 126 | A feature-length (120-page) screenplay imports and exports in a few seconds. |
| `report.rs` | 303 | Generic report documents: PDF (Helvetica, tables, page breaks), CSV, XLSX. |
| `samples.rs` | 212 | The four OpenFrame-owned sample scripts as regression fixtures (masterplan area "Call sheets, budget, templates, samples and exports", box "Sample scr |
| `screenplay_engine.rs` | 800 | Screenplay engine: page setup (Letter and A4, font size, spacing), one page layout shared by the preview and the PDF, keep-together rules, dialogue |
| `security_parsers.rs` | 295 | Regression tests for the 2026-09-30 security review: hostile XML (PARSE-01) and PDF (PARSE-02) input must fail fast and safely : never hang, exhaust m |
| `txt.rs` | 142 | TXT and pasted screenplay text (FSD §19.6, Import/Export §4.8, §4.11). |

### openframe-platform (0 test files)

| File | Lines | Purpose |
|---|---|---|

### openframe-security (2 test files)

| File | Lines | Purpose |
|---|---|---|
| `confinement_and_network.rs` | 200 | Regression tests for the 2026-10-05 security baseline (docs/release/evidence/M0/M0.28-security-baseline.md): E3 / link and junction safety (write dest |
| `security_paths.rs` | 268 | Regression tests for the 2026-09-30 security review (docs/engineering/SECURITY_REVIEW_2026-09-30.md): user-chosen path validation (PATH-01..04), shell |

### End-to-end specs (`tests/e2e/specs`, 30 files)

| File | Lines | Purpose |
|---|---|---|
| `01-journey.test.mjs` | 434 | The main desktop journey, in ONE running app, as a filmmaker would do it: first launch → project → Idea Vault → Story (act, sequence, cards, keyboard |
| `02-restart.test.mjs` | 56 | Quit and relaunch the real app on the same data (01-journey.test.mjs ran first): the project is listed on Application Home, reopens, and everything |
| `10-home.test.mjs` | 125 | @e2e-data-root: own Application Home and Project Home (build plan item end-to-end specs per workspace): create a project, change its status on |
| `15-trial-first-run.test.mjs` | 227 | @e2e-data-root: own Trial, purchase and first-run safety screens (owner mocks 182 to 198; feature rows PLAT-009 and |
| `16-about.test.mjs` | 88 | @e2e-data-root: own Settings > Project settings > About (ADR-0023, ADR-0024; build plan area 18 box "Internal |
| `20-idea-vault.test.mjs` | 105 | @e2e-data-root: own Idea Vault (build plan item end-to-end specs per workspace): capture an idea, give it a title in its drawer (autosave), undo |
| `21-idea-vault-flow.test.mjs` | 165 | @e2e-data-root: own @e2e-fake-media |
| `25-recently-deleted-purge.test.mjs` | 204 | @e2e-data-root: own Recently Deleted keeps items for 30 days, then removes them for good (owner decision 2026-10-07, |
| `30-story.test.mjs` | 176 | @e2e-data-root: own Story (build plan item end-to-end specs per workspace): build an act, a sequence and two Scene Cards; edit a card in its |
| `31-story-board-flow.test.mjs` | 283 | @e2e-data-root: own Story Board, full flow (build plan: Story Board and Idea Vault, "Story Board end-to-end test"): |
| `40-screenplay.test.mjs` | 170 | @e2e-data-root: own Screenplay (build plan item end-to-end specs per workspace): start a new screenplay, write a scene with the Enter flow |
| `41-screenplay-engine.test.mjs` | 267 | @e2e-data-root: own Screenplay engine (Screenplay engine and file formats area): write with the element flow, |
| `42-writing-comfort.test.mjs` | 281 | @e2e-data-root: own Writing comfort (build plan area "Writing comfort"; SCRIPT-001, SCRIPT-004, SCRIPT-017 to SCRIPT-020): |
| `43-several-screenplays.test.mjs` | 160 | @e2e-data-root: own Several screenplays per project (ADR-0012 amendment of 2026-10-08, masterplan area 11 box |
| `50-breakdown.test.mjs` | 138 | @e2e-data-root: own Breakdown (build plan item end-to-end specs per workspace): import a two-scene screenplay, choose it as the Production |
| `51-breakdown-depth.test.mjs` | 250 | @e2e-data-root: own Breakdown depth (masterplan area "Breakdown, catalog, locations, cast and crew"): category colour, |
| `60-appearance.test.mjs` | 141 | @e2e-data-root: own Appearance in the real app (build plan area "Appearance and themes"; features THEME-001 to THEME-006): |
| `61-schedule.test.mjs` | 351 | @e2e-data-root: own Shooting schedule and Day Out of Days (masterplan area "Shooting schedule and Day Out of Days"): |
| `62-call-sheets-exports.test.mjs` | 291 | @e2e-data-root: own Call sheets, budget, templates, samples and exports (masterplan area "Call sheets, budget, |
| `70-packages.test.mjs` | 179 | @e2e-data-root: own Packages and backup (build plan area "Packages and backup", features XFER-011 and XFER-012): |
| `72-ai-assistant.test.mjs` | 188 | @e2e-data-root: own Offline AI assistant with the REAL bundled model (ADR-0025; AI spec; agentic RAG spec §5, §21, |
| `73-ai-workspaces.test.mjs` | 109 | @e2e-data-root: own The AI Assistant entry points in every workspace with the REAL bundled model (ADR-0025; AI |
| `74-ai-change-sets.test.mjs` | 124 | @e2e-data-root: own AI Change Sets with the REAL bundled model (ADR-0025; agentic spec §7, §40; FSD §42.7–§42.15; |
| `75-ai-crash-safe-apply.test.mjs` | 91 | @e2e-data-root: own Crash safety of AI work with the REAL bundled model (ADR-0019 crash-safe Change Set apply; |
| `76-ai-bundle-integrity.test.mjs` | 129 | @e2e-data-root: own @e2e-ai-bundle: own |
| `80-notes-tasks.test.mjs` | 103 | @e2e-data-root: own Notes & Tasks (build plan item end-to-end specs per workspace): write a project note (autosave), undo and redo the edit, |
| `85-project-pages.test.mjs` | 142 | @e2e-data-root: own Project pages (build plan item end-to-end specs per workspace): Project settings (edit, save, undo, redo), Templates (create, |
| `90-visual-baseline.test.mjs` | 336 | Visual baseline capture (build plan item visual baseline, feature THEME-002) and visual regression (build plan item screen visual comparison). Runs af |
| `91-appearance-themes.test.mjs` | 238 | Every built-in theme in Light and Dark on the 15 representative screens, plus a relaunch (build plan area "Appearance and themes", items "Visual tests |
| `95-reachability.test.mjs` | 173 | Reachability test generated from the feature status file (build plan item reachability test). `scripts/check-reachability.mjs` reads every feature row |

### Screen tests (Vitest), counted by folder

| Folder | Test files |
|---|---|
| ai | 2 |
| api | 1 |
| app | 3 |
| app/home | 2 |
| app/shell | 2 |
| design-system | 1 |
| features/appearance | 1 |
| features/export | 1 |
| features/interchange | 2 |
| features/licensing | 1 |
| features/packages | 2 |
| features/team-gate | 1 |
| ipc | 1 |
| styles | 2 |
| theme | 1 |
| workspaces/breakdown | 2 |
| workspaces/call-sheets | 3 |
| workspaces/idea-vault | 4 |
| workspaces/production | 6 |
| workspaces/screenplay | 13 |
| workspaces/settings | 6 |
| workspaces/story | 3 |
| workspaces/trash | 1 |

## 19. The decisions, one line each

From `docs/adr/`. Status is as written in each record on 2026-10-09.

| ADR | Title | Status |
|---|---|---|
| 0001 | Tauri v2 + React/TypeScript + Rust | Accepted (implemented) |
| 0002 | rusqlite (bundled SQLite with FTS5 + online backup API) instead of SQLx | Accepted (implemented). Supersedes the "DB access: SQLx" row of ESD §2. |
| 0003 | `.openframe` project folder format and single-writer lock | Accepted (implemented) |
| 0004 | Single allow-listed IPC dispatcher `of_invoke`, operation registry, asset access resolved in Rust | Accepted (implemented) |
| 0005 | Generic trigger-based undo/redo with stale check | Accepted (implemented) |
| 0006 | Local-AI-only, managed `llama.cpp` sidecar with signed manifests | Accepted and implemented. Decision 3 (model policy) is superseded by ADR-0013: there is one |
| 0007 | LAN collaboration removed; collaboration is file-based through packages | Accepted: product-owner decision, 2026-09-30. Supersedes the earlier engineering plan |
| 0008 | Deterministic PDF output via `pdf-writer` and the PDF base-14 fonts | Accepted. Implementation in progress (export module; `pdf-writer` 0.13 and `lopdf` 0.36 are |
| 0009 | Permission matrix: decisions for cells the Security spec leaves open | Accepted (implemented in `crates/openframe-domain/src/auth.rs`, `Role::allows`) |
| 0010 | Editor-focused undo in the screenplay editor vs global undo | Accepted. The foundation part is implemented: the shell ignores Ctrl+Z/Y inside editable targets |
| 0011 | UUIDv7 identities and no stored display numbers | Accepted (implemented: `crates/openframe-domain/src/ids.rs`, all migrations, `persistence::rows`) |
| 0012 | Resolutions of conflicts between specifications | Accepted. Items 1–4 are implemented in the foundation. Items 5–9 are binding on the modules in |
| 0013 | One local model (Gemma 3 1B), local embeddings, hybrid retrieval with a derived context graph, a universal toolbox and a bounded agent loop | Accepted and implemented (merged on `master`, 30 Sept 2026). Supersedes ADR-0006 decision 3 |
| 0014 | Appearance and theme system (Light, Dark, System, optional themes) | Accepted 2026-10-07 (owner answer in chat: follow the 15 themes, keep the default as it is, and let the user c |
| 0015 | Screenplay production numbering, revisions and locked pages | Accepted 2026-10-07 (owner answer in chat). Amends ADR-0011 (item 3) and answers the first open question in AD |
| 0016 | PDF output with embedded subset fonts and complex-script shaping | Accepted 2026-10-07 (owner answer in chat). Amends ADR-0008 (decision 2 and the "Known limitation"). |
| 0017 | One codebase, two platform adapters, three release files | Accepted 2026-10-06 by the owner. Amends the Engineering Package Index locked decision "Initial OS" and supers |
| 0018 | Distribution, licensing and commerce model | Accepted in part 2026-10-06 by the owner: paid app with a 7-day free trial, no credit card to start, and the M |
| 0019 | Crash-safe application of multi-operation AI Change Sets | Accepted 2026-10-05 (owner decision: crash-safe AI changes; the owner delegated the choice to the recorded rec |
| 0020 | Offline AI runtime packaging per distribution channel | Accepted 2026-10-08 (owner, in chat: "AI engine inside each package"). Amends ADR-0006 for every |
| 0021 | Diagnostics, telemetry and analytics policy | Accepted 2026-10-05 (owner decision: diagnostics, telemetry and analytics; the owner delegated the choice to t |
| 0022 | What the Mac App Store build locks when the trial ends | Accepted 2026-10-07 (owner answer in chat: "add the Apple payment gate to buy the full version and stop |
| 0023 | Microsoft Store trial, internal Windows installer, no price in the app | Accepted 2026-10-07 (owner answers in chat the same day). Decision 3 extended to an internal Mac `.dmg`, with  |
| 0024 | Four release files, internal team builds behind a password | Accepted 2026-10-07 (owner answers in chat the same day). |
| 0025 | Offline AI bundled in every package | Accepted 2026-10-08 (owner, in chat). Permanent and cross-platform. |

## 20. The engineering documents, one line each

| File | Lines | Title |
|---|---|---|
| `11-physical-database.md` | 160 | 11: Physical Database (SQLite) Specification |
| `12-project-file-format.md` | 133 | 12. Local Project File Format |
| `13-command-architecture.md` | 135 | 13: Rust Domain & Command Architecture |
| `14-ipc-contract.md` | 103 | 14: Tauri IPC Frontend ↔ Backend Contract |
| `16-search.md` | 83 | 16: Search & Retrieval |
| `17-autosave-undo-recovery.md` | 121 | 17: Autosave, Undo, Recovery & Backup |
| `18-retrieval-intelligence-index.md` | 229 | 18: Retrieval & Local Intelligence Index |
| `20-security-threat-model.md` | 145 | 20: Security Threat Model & Secure Storage |
| `21-performance-budgets.md` | 280 | 21. Performance and resource budgets |
| `22-qa-test-strategy.md` | 156 | 22: QA & Acceptance Test Strategy |
| `23-release-migration.md` | 144 | 23: Release, Migration & Compatibility |
| `24-windows-packaging-updater.md` | 87 | 24: Windows Packaging, Signing & Updater |
| `25-ci-cd.md` | 95 | 25: CI/CD & Build Reproducibility |
| `26-observability.md` | 71 | 26: Observability: Logs, Diagnostics & Crash Reporting |
| `27-microsoft-store.md` | 331 | 27: Microsoft Store (MSIX) distribution |
| `28-cross-platform-compatibility-security.md` | 1284 | OpenFrame Studio: Strong Windows + macOS Compatibility and Security Plan |
| `29-desktop-hld-lld.md` | 1012 | OpenFrame Studio: Optimal HLD & LLD for the Desktop Application |
| `30-appearance-themes.md` | 180 | 30: Appearance and themes (engineering) |
| `31-ai-assisted-development-controls.md` | 1004 | OpenFrame Studio: Vibe Coding Gaps and How to Cover Them |
| `32-screenplay-engine-contract.md` | 169 | 32: Screenplay engine contract (proposed) |
| `33-production-depth-contract.md` | 208 | 33: Production depth contract (proposed) |
| `34-ux-laws-checklist.md` | 38 | 34: UX laws checklist for OpenFrame UI work |
| `35-writing-standard.md` | 69 | Writing standard |
| `36-entitlement-and-trial.md` | 486 | Entitlement and trial |
| `37-platform-boundary.md` | 180 | Platform boundary |
| `38-mac-app-store-trial.md` | 499 | Mac App Store trial and purchase |
| `39-packages-restore-drill.md` | 65 | 39. Packages and backup: restore drill |
| `AGENT_BRIEF.md` | 147 | OpenFrame Studio: Module Engineering Brief (read fully before coding) |
| `SECURITY_REVIEW_2026-09-30.md` | 145 | Internal security review: 2026-09-30 |
| `ai-tool-coverage.md` | 962 | AI tool coverage: operation inventory |

## Appendix A: audit coverage and continuation notes

This table says exactly what was inspected, how deeply, and what is left. "Read" means I opened the file and followed the
logic. "Counted" means I measured it with a command or search. "Listed" means I only looked at names or headings.
Nothing was built, run or tested.

| Area | How | Depth | What remains |
|---|---|---|---|
| Git history | Every commit subject (215), branch divergence, tags, authors, day counts | Deep | The 80 `master` commits and 31 old branches were counted, not read |
| Progress log, owner decisions, build plan | Read: dashboard, all log entries, decisions, first 125 lines and areas 3, 8, 17, 20 | Deep | The other 22 areas' boxes |
| Feature status | Read in full; parsed by script for counts and the estimate | Deep | Rows were not re-proved |
| ADRs | Read 0001, 0004, 0006, 0013, 0019 (part), 0024 (part), 0017 (grep); titles and status of all 25 listed | Partial | 0002, 0003, 0005, 0007 to 0012, 0014 to 0016, 0018, 0020 to 0023, 0025 bodies |
| Engineering documents | Titles and sizes of all; read the AI coverage head, writing standard, storage policy | Partial | Bodies of 11 to 33 |
| Specifications | Listed; README read; headings of the archived plan | Listed | All bodies (31,000 lines) |
| Tauri shell | `lib.rs` lines 1 to 620, capabilities, `tauri.conf.json`, bundled-AI config, platform stub, `mac.yml` | Deep for startup and IPC | `distribution.rs` beyond WebView2, `asset_scope.rs` body, `build.rs`, Swift bridge |
| Registry and core | `dispatch`, metadata flags, argument shape check, `AppCore` config, shutdown, `local_actor` | Deep | Registration helpers, task system |
| Mutation pipeline | `store.rs` open and mutate fully; outline of the rest | Deep | Undo step internals, soft delete bodies |
| Persistence | `migrate.rs` first 230 lines, `lib.rs` first 130 lines | Partial | `undo.rs`, `rows.rs`, backup code |
| Project lifecycle | `open_at`, `close_current`, `save_checkpoint`; function list | Deep for open and close | create, duplicate, delete, locate, rename bodies |
| Packages | Restore replace path (80 lines); op list | Partial | Export builders, import session, exchange |
| Screenplay, story, production, schedule, visual, notes, exports, comments, vault modules | Sizes and operation names only | Counted | All bodies (about 80,000 lines) |
| AI | Orchestrator constants, policy text, `run_agent`, change set outline, `require_human`, tool selection, catalog sample, supervisor config, bundle state, hardware, manager launch settings, lock file head | Deep for control flow | Retrieval ranking bodies, assembler, indexer, tool resolvers, catalog bodies |
| Import-export, search, entitlement, security, platform | Module purpose lines and sizes | Listed | Bodies; Mac code; team gate tests |
| Frontend | `client.ts`, `query.ts`, `routes.tsx`, `stores.ts`, `ErrorBoundary.tsx`, `sync.ts` and flush points, `RecoveryDialogs.tsx` function; counts of silent catches, console use, `aria` attributes; file sizes | Partial | All components, the design system, accessibility behaviour |
| SQL migrations | Table names per migration | Listed | Column definitions, triggers, indexes |
| Tests | Counts, file lists, three tests read | Counted | Test quality file by file |
| Build and CI | `Cargo.toml`, `.cargo/config.toml`, scripts list, `cargo-queued.mjs` head, `mac.yml`, `package.json` scripts, build folder sizes | Deep | `ci.yml`, `release.yml` bodies |
| Claude setup | Project and user settings, hooks list, memory (all), `session-2-brief.md` head, `AGENTS.md` head, GSD folders listed | Deep | Plugin contents |
| Mock-ups | Index, links, names | Partial | Visual comparison with the app (needs a person) |
| Documents' links | `check-links.mjs` run: OK | Deep | External links |
| Store rules | Four web searches | Light | Official pages not reached; verify before building |

### Continuation notes for a later pass (only if wanted)

1. Read `modules/screenplay/edit.rs` and `lock.rs` for edge cases in the writing path.
2. Read `modules/packages/import.rs` (3,500 lines) for hostile-package handling beyond the tests.
3. Read `openframe-persistence/src/undo.rs` for trigger correctness and large transactions.
4. Read `apps/desktop/src/workspaces/screenplay/editor/` fully (rules, commands, suggest, find) beside the tests.
5. Run (with approval) `cargo test -p openframe-application --test ai_tool_coverage` and the governance scripts to confirm
   the counts quoted here.
6. Compare a dozen mock-up screens with screenshots from a running build.

## Appendix B: glossary (plain words)

| Word | Meaning |
|---|---|
| Actor | The person or process asking for something (local user, imported package, AI) |
| ADR | Architecture decision record: one page that says what was decided and why |
| Agent (AI) | A loop in which the model chooses a tool, the program runs it, and the result goes back to the model |
| Agent (Claude Code) | A helper Claude working on a separate task |
| App-local | Files placed inside the app's own folder instead of the system |
| Asset | A picture, audio, video or file stored in a project's `assets` folder |
| Backup package | A ZIP of a whole project made on purpose |
| Branch | A separate line of work in Git |
| Bundle (AI) | The folder of AI files shipped inside the installer |
| Capability | A permission such as Edit or Export; also, in Tauri, what the page may do |
| Change Set | A reviewable list of changes proposed by the AI or an import |
| Checkpoint | A copy of the database taken at a clean close or on Save |
| CI | Continuous integration: automatic checks on a server |
| Command (Tauri) | A Rust function the page may call |
| Command (OpenFrame) | An operation that changes data |
| Commit | A saved snapshot in Git; also a finished database transaction |
| Context window | How much text a model can consider at once |
| CSP | Content security policy: rules for what a web page may load or contact |
| Crate | A Rust package |
| Debug build | A build made for developers; larger and slower |
| DTO | A plain data shape sent between Rust and TypeScript |
| E2E | End-to-end test: drives the real program |
| Embedding | Numbers that stand for the meaning of a text |
| Event | A message from Rust to the screens, such as "these tables changed" |
| FTS5 | SQLite's full-text search |
| Fixture | Prepared sample data for tests |
| GGUF | The file format for model weights used by llama.cpp |
| Gatekeeper | macOS's check of downloaded apps |
| Handler | The function that runs one operation |
| Hook | An automatic script that runs around a tool use in Claude Code |
| IPC | Inter-process communication: how the page and Rust talk |
| Junction | A Windows folder shortcut that behaves like the real folder |
| Linker | The tool that joins compiled pieces into one program |
| llama.cpp | A program that runs language models on ordinary computers |
| Migration | A numbered script that changes the database layout |
| MCP | A way to connect Claude to outside services |
| Notarization | Apple's scan and record of a signed Mac app |
| Operation | A named action the brain can do, such as `story.create_act` |
| Package (OpenFrame) | A file made to share work or make a backup |
| Plugin | A bundle of skills, agents, commands and hooks for Claude Code |
| Profile (Cargo) | A set of compiler settings |
| Proposal | A change the AI prepared but did not apply |
| Query | An operation that only reads |
| RAG | Retrieval-augmented generation |
| Registry | The list of all operations with their metadata |
| Release build | The optimised build customers get |
| Role | Owner, Editor, Commenter, Viewer or Export-only |
| sccache | A cache of compiled Rust pieces |
| Sidecar | A helper program started by the main program |
| SmartScreen | Windows's reputation check of downloaded programs |
| Soft delete | Marking as deleted but keeping, so it can be restored |
| Stale | Out of date compared with the current data |
| Store (Rust) | The object that owns a database and the one write path |
| Target folder | Where Rust puts compiled output |
| Token | A piece of a word that a model reads |
| Transaction | Changes that happen all together or not at all |
| Trigger (SQL) | A rule inside the database that runs on every change |
| UUIDv7 | A time-ordered unique id |
| WAL | Write-ahead log: SQLite's crash-safe way of saving |
| WebView2 | The browser engine Windows apps use |
| Worktree | A second working folder for another Git branch |

## Appendix C: commands you will actually use

| Command | What it does | Cost |
|---|---|---|
| `npm run check:governance` | Architecture rules, migrations, feature status, links, writing | Seconds |
| `node scripts/plan-status.mjs` | Build plan counts | Seconds |
| `node scripts/check-feature-status.mjs` | Feature counts and table checks | Seconds |
| `node scripts/check-links.mjs` | Broken relative links | Seconds |
| `node scripts/check-writing-standard.mjs` | Dashes and banned words | Seconds |
| `node scripts/cargo-queued.mjs test -p <crate> --test <file>` | One Rust test file through the queue | Minutes |
| `npm --workspace apps/desktop run test` | Screen tests | Under a minute |
| `npm --workspace apps/desktop run typecheck` | TypeScript check | Under a minute |
| `npm run lint` | Clippy and ESLint | Minutes |
| `npm run verify` | Everything except end-to-end | Long |
| `npm run test:e2e` | The end-to-end suite (about 20 minutes, builds a debug app, drives windows) | Heavy |
| `npm run perf` | Performance suite | Minutes |
| `npm run bindings` | Regenerate the TypeScript types | Minutes |
| `npm run ai:assets` | Verify and stage the AI files | Seconds |
| `npm run bundle` | Build the Windows installer | **Do not run without the owner's word** |
| `graphify query "<question>"` | Ask the code graph | Seconds |
| `du -sh .build-cache .claude/worktrees` | Check sizes | Seconds |
| `git log --oneline -20` | Recent history | Instant |
| `git worktree list` | Registered worktrees | Instant |

## Appendix D: where to look when something happens

| You want to know | Look at |
|---|---|
| Why a button does nothing | The screen file under `workspaces/`, then the operation name in `registry` metadata |
| Why an error message appeared | The error code in the toast; search the code for it |
| Why text was not saved | `editor/sync.ts`, then `modules/screenplay/edit.rs`, then `store.rs` |
| Why a project needs recovery | `recovery/session.json`, `open_at` in `project.rs` |
| Why the AI answered badly | `modules/ai/orchestrator.rs`, `toolbox.rs` tool selection, the retrieval documents |
| Why the AI is off | The bundle state in Settings, Offline AI; `bundle.rs` |
| Why a build is slow or huge | Section 12 and `development-storage-policy.md` |
| Why a feature is marked as it is | The row in `feature-status.md`, then the test it names |
| What a decision was | `docs/adr/README.md` |
| What the owner decided | `owner-decisions.md` |
| What to do next | `docs/implementation/handoff.md` |

## Appendix E: sources used for this report

Files read in the repository: `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`, `CONTRIBUTING.md` (part), the documentation map,
the build plan (parts), `feature-status.md`, `owner-decisions.md`, `progress-log.md`, `reconciliation-log.md` (head), the
storage policy, the writing standard, ADR-0001, 0004, 0006, 0013, 0017, 0019, 0024, the specification and website READMEs,
the archive's plan headings, `Cargo.toml`, `.cargo/config.toml`, `tauri.conf.json`, `capabilities/default.json`,
`tauri.bundled-ai.conf.json`, `package.json`, `.github/workflows/mac.yml`, the Rust and TypeScript files named in the text,
the scripts named in the text, the memory files and the Claude settings.

Web: Tauri WebDriver documentation (macOS not supported by `tauri-driver`), Gemma Terms of Use summaries, Microsoft Store
commerce policy summaries, Apple trial guideline summaries. These four searches did not reach the official pages in full.
Verify before acting.

Commands run (all read-only): `git log`, `git branch`, `git worktree list`, `du`, `ls`, `grep`, `wc`, `node` scripts that
read files, `sccache --show-stats`, `graphify query`, and the four Node check scripts (links, plan status, feature status,
writing standard). No build, test, installer, push, deletion or source change was made.

## Appendix F: every commit on `main`, oldest first

Date, short hash and subject (215 commits). The first three are large snapshots; the finer history of 2026-09-30 is on the `master` branch.

```text
2026-09-30 10:12  25d27a6  OpenFrame Studio: local-first desktop filmmaking studio (Tauri v2 + React + Rust + SQLite)
2026-09-30 11:48  f9d4660  Security review and hardening: path validation, safe package/database import, parser limits, AI safeguards, release key gate
2026-10-01 20:35  d30b588  Offline AI agent (Gemma 3 1B, hybrid RAG, toolbox, human-only Change Sets), AI tool coverage, and updated user manual
2026-10-06 19:34  4fcbe4b  Foundation: fix the thirteen failing resilience tests with product fixes, add the platform boundary and entitlement crates
2026-10-06 19:34  af55520  Checks: architecture lint rule R6 for the platform boundary, Mac target in cargo-deny, writing baseline lowered
2026-10-06 19:38  4ee49ef  CI: quote the governance job name so the workflow file parses
2026-10-06 20:12  2d78efb  Migrations: commit the AI apply journal migration and the migration record
2026-10-06 20:23  ec76a59  Documents: build plan with descriptive names, progress log, evidence for the foundation session, entitlement and platform boundary documents
2026-10-06 20:31  b99e64e  Evidence: store the verify log as a text file so it is not ignored by the log pattern
2026-10-06 20:52  2daeae0  Mac App Store trial: server-authoritative 7-day trial and lifetime unlock behind the entitlement interface
2026-10-06 20:54  df97a0c  Licence server: evict the least recently seen address instead of clearing the rate limiter
2026-10-06 20:56  3289605  Licence server: O(1) rate limiter eviction with two generations
2026-10-06 21:10  c960a56  Entitlement: a path component that is a file means no record on every operating system
2026-10-06 21:13  462b9e8  Rules: develop, test and build locally; push to GitHub only when the owner asks
2026-10-06 22:28  3327db5  End-to-end fixes: Drafts drawer keeps its dialogs, Home refreshes after Rename, plain draft titles, deterministic visual capture
2026-10-06 22:32  598cf5a  Evidence: final end-to-end run green (53 of 53), two more test infrastructure boxes ticked, dashboard 42 of 281
2026-10-07 00:47  91ec2e9  Owner decisions 2026-10-07: themes, production numbers, Indian-script PDF fonts, templates, samples, product questions
2026-10-07 01:41  ddb01e7  Appearance core: theme data, strict theme importer, contrast check and app migration 0002
2026-10-07 01:56  4eefb5e  Story Board and Idea Vault: long scene description, Card Display Settings, card limits, lineage links, research flags
2026-10-07 02:01  ea3911c  Mockups: remove every writing standard hit and lower the mockups baseline to 0
2026-10-07 02:02  dd1795b  Merge mockup writing cleanup: 1311 em dash hits removed from HTML_Mockups
2026-10-07 02:03  9b8119a  Add the four OpenFrame-owned sample scripts under templates/samples
2026-10-07 02:04  dc56d9b  Writing check: skip the two website prompts that list banned phrases as words to avoid; living-docs baseline 671 to 665
2026-10-07 02:05  5d1325b  Merge four OpenFrame-owned sample scripts (Example Feature, Short, Scene, FDX Interchange Test)
2026-10-07 02:14  90be753  Themes in the UI: one token layer, Settings > Appearance, theme import and the new logo
2026-10-07 05:34  fa513c4  Appearance tests, docs and Windows High Contrast states
2026-10-07 05:34  edac25e  Screenplay engine: migration 0013, element rule table, page setup with A4, header and footer, page breaks, custom elements, production numbers and loc
2026-10-07 05:35  712b1c3  Trial chip, reminders, Buy routing and the first-run safety screens
2026-10-07 05:35  e9e6df4  Temporary placeholder migrations 0012 to 0014 for the schedule branch
2026-10-07 05:37  d345bca  Writing comfort: suggestions, custom shortcuts, navigator columns, proofing, find scope, element flow matrix
2026-10-07 05:41  770f212  Review fixes: safe assistant edits of long descriptions, accessible card facts, migration test fixtures for schema 12
2026-10-07 05:45  6ec4428  Paper test writes its screenplay through the editor operations; AI tool coverage lists the hidden appearance operations
2026-10-07 05:46  8065dcf  Writing comfort: regenerate the AI tool coverage matrix for the writing operations, format the new test
2026-10-07 05:49  416e74d  Application Home: Restore Backup or Package entry
2026-10-07 05:51  e99da38  Temporary placeholder migrations 0012 to 0015 for the call sheets branch
2026-10-07 06:00  406c1c3  Merge commit 'edac25e' into area/11b-formats-fonts
2026-10-07 06:11  748bfc4  Packages UI and Home: remove em dashes from user text and comments
2026-10-07 06:18  fa6b941  AI tool coverage: list the new hidden licence and trial operations
2026-10-07 06:29  a3dfd0b  Security: bound the cost of importing custom elements from hostile files
2026-10-07 11:47  7db2758  Formats and fonts: save in-progress font embedding and export work
2026-10-07 11:47  d31c5aa  Session 1 fixes: save in-progress persistence, AI and platform handoff fixes
2026-10-07 11:47  b51778b  Screenplay engine: save in-progress engine work
2026-10-07 11:47  afe5892  Packages and backup: save in-progress package work
2026-10-07 11:47  515b0db  Writing comfort: save in-progress change
2026-10-07 11:47  f493bd1  Schedule and Day Out of Days: save in-progress schedule work
2026-10-07 11:47  e5771ad  Breakdown, cast and crew: save in-progress breakdown work
2026-10-07 11:47  46fd8b9  Call sheets and exports: save in-progress budget, template and export work
2026-10-07 11:47  39592d2  Themes and logo: save in-progress theme work
2026-10-07 11:50  103bc06  Restore LF line endings in the Story Board and Idea Vault files
2026-10-07 12:13  6096a01  Themes: finish the security review round and verify the slice
2026-10-07 12:15  d2cca87  Merge area 12 Appearance and themes: fifteen built-in themes, Light, Dark and System, strict theme import, My Themes and the new logo
2026-10-07 12:25  2200db4  Coverage table without em dashes, Scene Card link row, E2E references for Story and Vault rows
2026-10-07 12:26  eadd8d0  Merge area 9 Story Board and Idea Vault: long scene description, card display settings and presets, card limits, links that follow the scene, research
2026-10-07 12:28  954d397  Merge session-2 (area 12 themes, new logo, colour rule T1) into the trial and first-run branch
2026-10-07 12:43  18d91b0  Record three owner decisions: Mac trial end lock, default theme contrast, 30-day purge plan box
2026-10-07 12:46  fc68dba  Writing check: lower the baselines after the area 9 and area 12 merges (app 265, crates 503, scripts and CI 34, living docs 64)
2026-10-07 12:48  f507b4f  Record ADR-0023: the Microsoft Store runs the trial with the Mac's full block, the Windows installer is an internal team build, and the app never show
2026-10-07 12:51  d0bc42d  Merge session-2 into the screenplay engine branch
2026-10-07 12:51  6354cf4  Merge session-2 into the formats and fonts branch
2026-10-07 13:05  5ee87dc  Security: bound custom paragraph types in the Final Draft parser and prove a refused import changes nothing
2026-10-07 13:08  8d86858  Record ADR-0024: four release files, with internal .exe and .dmg builds for the team behind a password asked on every launch
2026-10-07 13:09  216cd5b  Shadows: side panels use a shadow token and colour rule T1 stops treating navy as neutral
2026-10-07 13:20  f0a317d  Line endings: keep the merge-touched files in LF like the rest of session-2
2026-10-07 13:20  3874457  Screenplay editor: page breaks and custom elements in every keyboard path, and current IPC bindings
2026-10-07 14:04  a885f64  Store lock in Rust, Microsoft Store licence seam, internal installer build, and the review fixes on the licence screens
2026-10-07 14:05  fb5d9f1  Merge trial and first-run screens: store lock in the Rust core, Microsoft Store seam, internal installer build, no price in the app
2026-10-07 14:05  884ffb2  OpenFrame Default passes 4.5 to 1 in Light mode: three colours raised with the smallest change
2026-10-07 14:07  6ec83e1  Merge session-2 (areas 9 and 12, ADR-0022 to ADR-0024) into writing comfort
2026-10-07 14:08  c494f28  Merge session-2 (store lock in Rust, trial screens, shadow token, default theme contrast) into writing comfort
2026-10-07 14:13  d58ef39  Screenplay engine: finish the part A slice with its gates, and align older tests with ADR-0015
2026-10-07 14:13  cc61c48  Merge session-2 (trial screens) into the screenplay engine branch
2026-10-07 14:15  b0f9f13  ADR-0012: mark the no auto-purge line superseded by the 30-day purge decision
2026-10-07 14:16  505fedb  Page setup tests: type the operation callbacks so the desktop typecheck passes
2026-10-07 14:17  fe317c1  Writing comfort: keep suggestions and spelling cheap per keystroke on a long script
2026-10-07 14:19  7c215a3  Line endings: restore LF
2026-10-07 14:19  093b2fa  Line endings: restore LF
2026-10-07 14:32  dfd2ce6  Writing comfort: every shortcut hint follows Settings > Keyboard shortcuts
2026-10-07 14:33  bc949a0  Writing comfort: E2E case for shortcuts from Home, plugin typing budget in the performance document, feature rows
2026-10-07 14:33  3b57fbf  Merge session-2 (ADR-0012 purge line marked superseded) into writing comfort
2026-10-07 14:34  af27897  Writing comfort: the shortcut error line uses the theme's error colour
2026-10-07 14:35  fd7e6f8  Writing comfort: regenerate the AI tool coverage matrix after the session-2 merges
2026-10-07 14:41  bed2a3b  Import: a file with more custom styles than a screenplay can hold imports as action instead of being refused
2026-10-07 14:41  524ef73  Merge area 11 part A Screenplay engine: custom elements, page setup, production numbering, page breaks and the bounded custom element import
2026-10-07 14:41  4556670  Merge the screenplay engine branch (part A, with session-2 and the trial screens) into the formats and fonts branch
2026-10-07 14:45  ba712c8  Governance test: the migration gap case uses a number far past the latest migration
2026-10-07 14:50  8ba7b68  Embedded Indian-script fonts in every PDF export, a checked font manifest, and DOCX script hints
2026-10-07 14:54  90780b2  Exports: prove Telugu and Hindi print in every PDF export kind
2026-10-07 14:57  44cb7b2  Merge area 10 Writing comfort: smart suggestions, custom keyboard shortcuts, navigator columns, proofing with operating system dictionaries, the eleme
2026-10-07 14:59  36ac07d  Merge session-2 into the breakdown branch
2026-10-07 14:59  71e9aa9  FDX round-trip suite for thirteen script classes, and import fidelity fixes it found
2026-10-07 14:59  5b592b3  Merge branch 'session-2' into area/11b-formats-fonts
2026-10-07 14:59  d4eba45  Merge branch 'area/11a-screenplay-engine' into area/11b-formats-fonts
2026-10-07 15:08  5bd187d  Shortcuts: the page break is an editable command and Ctrl+Alt+1 to 9 stay with custom elements; navigator follows page setup
2026-10-07 15:09  1262a61  Navigator uses the screenplay page setup, so session-2 compiles again after joining areas 10 and 11
2026-10-07 15:09  989247d  Merge area 11 part A follow-up: the governance test uses its own migration gap fixture
2026-10-07 15:10  ddc713f  Merge branch 'session-2' into area/13-breakdown
2026-10-07 15:12  ca8d80d  PDF fonts: Indian scripts print at nine tenths of the page size, with a reviewed raster golden
2026-10-07 15:15  a045fc8  Formats: multilingual round trip and the Final Draft migration definition of done through the registry
2026-10-07 15:16  ff91f02  Docs: format fidelity table and embedded font notes in contract doc 32; feature status for the formats rows
2026-10-07 15:18  9bf6401  E2E spec: Telugu and Hindi in the screenplay PDF, and Insert page break on a remapped key
2026-10-07 15:20  e6f29d2  Breakdown, catalog, locations, cast and crew: finish and verify the saved work
2026-10-07 15:23  fe684c4  Breakdown: a damaged saved scene filter falls back to all scenes
2026-10-07 15:25  dfb2de8  Breakdown: declare the category colour property so no theme falls back to a browser default
2026-10-07 15:33  153e63c  Breakdown performance: record the measured filter, reuse and reference budgets
2026-10-07 15:39  e4b1f20  Part B gates: clippy fixes and older tests aligned with embedded fonts and reserved shortcuts
2026-10-07 15:40  160f6e4  Merge branch 'session-2' into area/11b-formats-fonts
2026-10-07 15:42  c00c683  Merge area 11 part B Formats and fonts: embedded Indian-script fonts in every PDF export, the 13-class FDX suite, import fidelity and the editable pag
2026-10-07 15:45  5cf5ae5  Migration tests: teach the schema 10 fixtures about project migration 0014
2026-10-07 15:49  eb778ab  Normalize the saved session 1 files to LF line endings
2026-10-07 15:50  0a6dfe3  Merge branch 'session-2' into fix/session-1-handoffs
2026-10-07 16:52  788a505  Merge area 13 Breakdown, catalog, locations, cast and crew: category looks, bulk actions, source phrases, locations and people with private contacts, 
2026-10-07 16:59  e334f41  Schedule branch: convert the files the saved work wrote with Windows line endings back to LF
2026-10-07 16:59  1ebcbdd  Production depth contract: keep its existing CRLF line endings so the diff shows only the schedule rows
2026-10-07 17:00  4a51d96  Merge session-2 into the schedule branch: areas 9 to 13, the trial screens and ADR-0022 to ADR-0024
2026-10-07 21:44  2cc3b0d  Add development storage policy: shared cargo target dir, worktree hygiene
2026-10-08 10:17  1c96f8d  Schedule branch: save the uncommitted schedule work as it stood
2026-10-08 10:17  d56d977  Session 1 handoffs: save the uncommitted work as it stood
2026-10-08 10:22  79a660a  Record the owner decisions of 2026-10-08 and bring the control documents up to date
2026-10-08 10:23  11fcf3f  Merge session-2 into the schedule branch: owner decisions of 2026-10-08 and the storage policy
2026-10-08 10:24  d6fbeb3  Merge branch 'session-2' into fix/session-1-handoffs
2026-10-08 10:27  8c3fda5  Packages and backup: restore LF line endings on the files the in-progress commits saved with CRLF
2026-10-08 10:29  4c72593  Merge session-2 into area/16-packages
2026-10-08 11:26  6f3410d  Make the merged session-2 code pass rustfmt and clippy
2026-10-08 11:26  3f74d13  Finish the session 1 handoffs: team password gate, purge and About
2026-10-08 11:31  d1ddb75  Packages carry production numbers and custom elements; hostile package reader tests
2026-10-08 11:36  a16e9a2  Packages: review round trip in the E2E spec, viewer tests, format and drill docs
2026-10-08 11:40  bff4d25  Architecture lint: allow the Full Project Package scrub in packages/coverage.rs
2026-10-08 11:45  0a985b4  Supply chain: dated cargo deny exception for the unmaintained font crates
2026-10-08 11:52  9c7b9f9  Bindings: regenerate ProjectManifest after the comment fix in project-format
2026-10-08 11:54  bc67c9b  Evidence: area 16 Packages and backup, session 2 run of 2026-10-08
2026-10-08 11:58  c6d3ee8  Owner decisions 2026-10-08: one build folder per worktree, restored private notes belong to the person restoring
2026-10-08 11:59  5a20f12  Merge branch 'session-2' into fix/session-1-handoffs
2026-10-08 11:59  ea2b5da  Merge branch 'session-2' into area/16-packages
2026-10-08 13:00  9e744fb  Build tools: sccache and rust-lld for dev and test builds, Microsoft linker for releases
2026-10-08 13:01  0dd1e47  Plan: one test binary per crate (area 5) and Defender exclusions for build tool folders (owner task)
2026-10-08 13:08  020e9f0  Finish the shooting schedule and Day Out of Days area: search after undo, correction notes, tests and docs
2026-10-08 13:18  f9a4a19  Owner task done: Defender exclusions for the sccache, cargo and rustup folders
2026-10-08 13:23  cda7753  Merge branch 'fix/session-1-handoffs' into session-2
2026-10-08 14:01  e81c1d5  Packages: a restored backup's private notes belong to the person restoring it
2026-10-08 14:08  6ddf039  Allow several screenplays per project and name the screenplay everywhere
2026-10-08 14:09  fe6d7e2  AI sidecar soak test: count only the fake servers this test started
2026-10-08 14:10  b139bd8  Area 14 evidence, and comment fixes in the files the schedule branch touched
2026-10-08 14:22  db4a244  Merge branch 'area/16-packages' into session-2
2026-10-08 14:22  c1a81c2  Owner decisions: screenplays can be deleted to Recently Deleted, episodes may hold several screenplays
2026-10-08 14:26  d5088e5  Merge branch 'session-2' into area/14-schedule
2026-10-08 14:30  1ff346e  Storage policy: unlink a worktree's node_modules junction before removing it
2026-10-08 14:30  9653c10  Merge session-2 into the several screenplays branch
2026-10-08 14:48  e94748a  Packages: coverage rules for the schedule timeline and Day Out of Days corrections
2026-10-08 15:02  6b4f596  Merge branch 'area/14-schedule' into session-2
2026-10-08 15:03  5ef0c93  Let a whole screenplay go to Recently Deleted and come back
2026-10-08 15:15  dddf595  Merge session-2 into the call sheets branch
2026-10-08 15:16  2cb5ba5  Build queue: one memory-heavy cargo build at a time across all worktrees
2026-10-08 15:17  634ab48  Build queue: Prettier formatting
2026-10-08 15:24  0097907  Merge session-2 (area 14 Schedule and Day Out of Days) into the several screenplays branch
2026-10-08 15:24  f5ad6d7  Merge branch 'area/11-several-screenplays' into session-2
2026-10-08 15:58  8be3687  Call sheets: full content, Draft / Needs refresh / Final, and a versioned source
2026-10-08 15:58  18878dd  Budget Planned and Actual, OpenFrame branding on production exports, PDF regression
2026-10-08 15:59  49b49bf  Built-in templates as independent copies, and the four sample scripts shipped in the app
2026-10-08 16:09  911335f  Call sheets: prove edits never touch other modules, keep typed actors and addresses
2026-10-08 16:10  d3af142  Call sheets: test Add Section permissions, search, undo and the Final refusal
2026-10-08 16:10  8237773  End-to-end spec for call sheets, budget, templates, samples and exports; contract doc 33
2026-10-08 16:40  d0e91ba  Area 15 evidence, migration downgrade harnesses for schema 16, writing baseline
2026-10-08 16:42  634648f  Record the 2026-10-08 owner decisions: internal builds first, Offline AI bundled
2026-10-08 16:42  8cc2eec  Merge branch 'area/15-callsheets-exports' into session-2
2026-10-08 16:50  2a646f1  Build queue: pass the target folder and job count as flags so sccache can hit
2026-10-08 17:02  b00fda2  Ignore the offline AI build inputs folder (.ai-assets, ADR-0025)
2026-10-08 17:14  c853632  AI tool coverage: regenerate after joining all session 2 areas
2026-10-08 17:15  63a486e  Pin the bundled Offline AI files and stage them for every package build
2026-10-08 17:52  560f677  Load Offline AI from the app resources and remove the download flow
2026-10-08 18:04  1822312  E2E specs for every AI feature with the real bundled model
2026-10-08 18:15  fdd792a  Bring the documents in line with the bundled Offline AI
2026-10-08 18:16  2d55adf  Real-model test: a proposal whose target changed is stale and never applied
2026-10-08 18:17  c21d5ea  Merge session-2 into area/08-bundled-offline-ai
2026-10-08 18:25  55ede7f  Harden the bundled AI after the security review
2026-10-08 18:41  7a68f81  Evidence for the bundled Offline AI; lower the writing baseline
2026-10-08 18:42  fd05d31  Merge branch 'area/08-bundled-offline-ai' into session-2
2026-10-08 18:50  900f245  Owner decisions: app-local VC++ runtime for the AI engine, CPU engine only, CI AI files decided at the next push
2026-10-08 19:42  6eca5bb  E2E: write character cues the way the suggestion list now works
2026-10-08 20:29  d9391c0  Store lock: answer the team password question while the app is locked
2026-10-08 20:29  48bc8ef  App start: show the splash only until the first answer about the open project
2026-10-08 20:47  641174e  Show files from disk through one helper on the app's own protocol; voice notes know their length
2026-10-08 20:47  00d0dfd  Screenplay editor: Enter goes into an empty next element instead of stacking another
2026-10-08 20:47  86eba05  Toasts: the action is a button
2026-10-08 20:47  1a16c7c  Stripboard strips: a fixed dark text colour on the fixed paper colours
2026-10-08 20:47  5f8df79  Screenplay layout: save a switch at once and never drop a waiting change
2026-10-08 20:48  0502145  E2E runner: fake media devices that reach the webview, and clean driver hand-overs
2026-10-08 20:48  d9178dc  E2E: align five specs with the screens they test
2026-10-08 20:50  2509587  Shortcut recorder: hold the keys from the moment it shows
2026-10-08 20:50  80c4b48  E2E: writing comfort and breakdown depth follow the screens as a user does
2026-10-08 20:51  969e2cf  E2E: call sheets and exports spec follows the screens and operations
2026-10-08 21:06  df0f241  E2E: fill Windows file dialogs reliably, and follow the package screens
2026-10-08 21:33  df1ba55  Assistant: plain task commands need no model; grounded drafts; named search hits
2026-10-08 21:33  772b666  AI Assistant: close the panel when it opens Offline AI settings
2026-10-08 22:01  529f7d9  Project close: finish the clean close before the app shows Home, and let quitting wait for it
2026-10-08 22:01  14195ce  Assistant: the product guide never replaces project facts already found
2026-10-08 22:08  6cdf41d  Readable counts in the Story Outline and Files, in every theme
2026-10-08 22:08  a500d6d  Dialog footers wrap instead of crushing their note
2026-10-08 22:41  9416b14  E2E: wait for Application Home itself and for the answer to the request just sent
2026-10-08 22:41  2c5268e  E2E: AI specs check the chat engine and wait for the scope line
2026-10-08 22:41  dbd55e4  E2E: reachability covers the routes the feature status file now lists
2026-10-08 22:41  0e6d035  Visual baseline: capture the default theme in Light mode and refresh the 15 screens
2026-10-08 23:16  7a8df83  Session 2 end-to-end evidence, and a lower writing standard baseline
2026-10-08 23:38  7e74f07  Dock the AI Assistant panel beside the page
2026-10-08 23:38  8d6b53e  Screenplay: one Enter after a new character name creates it and moves on
2026-10-09 00:17  922a7c8  Docked AI panel: nothing cut off at the smallest window; spec fixes
2026-10-09 00:18  d5cc2bb  Remove dashes from text OpenFrame shows or writes itself
2026-10-09 00:44  4fdd7d3  Visual baseline: refresh the Production Overview screen for the new wording
2026-10-09 01:04  86d7f3f  Evidence: docked AI panel, one-Enter character flow and dash sweep, final full E2E green
2026-10-09 01:06  df4853c  rustfmt: re-wrap the lines the dash sweep shortened (formatting only)
2026-10-09 01:15  d75140d  Screenplay: one Enter on a known name from the list also moves on to Dialogue
2026-10-09 01:36  173f064  Evidence: one Enter on a known name, gates and final full E2E green
2026-10-09 01:50  fbed4bc  Plan and status: tick session 2 boxes and move features with evidence
2026-10-09 01:52  e137540  Reconciliation log: correct the Verified count in the fbed4bc message (62, not 64)
2026-10-09 01:52  c8ea7da  Merge session-2 into main: session 2 complete (areas 8 to 18 work, bundled offline AI, session 1 handoffs)
2026-10-09 01:59  0acbd5e  Progress log: local merge of session-2 into main and the merged-branch cleanup
```

## Appendix G: the short version, for the first five minutes of any session

1. Chapter 1 is the only job. The next work is protecting writers' work (close-window flush, synced folders, delete and
   locate tests), then proving the core, measuring the AI, building the Windows installer, bringing up the Mac, and the
   2 to 3 day pilot.
2. Read `CLAUDE.md`, then `docs/implementation/handoff.md`, then the Chapter 1 page, then one work item.
3. The code is the truth. Check a document against the code before trusting it.
4. Do not push, build an installer, or delete worktrees without the owner's word.
5. Explain in simple words. Put details in documents.

