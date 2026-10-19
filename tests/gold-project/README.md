# Low Tide Radio: the gold reference project

OpenFrame Studio's reference film project. One original feature film taken from the first idea
to the shoot paperwork, built through the same core operations the app's buttons call, then
exported in every format, imported, restored and compared. Use it to check a change against a
real production, to test import and export, and to see every workspace full of real data.

## The film

**Low Tide Radio** (feature, drama, English and Malayalam, 44 scenes, about 55 pages).
Written and directed by Meenakshi Varma for Kayal Pictures, Fort Kochi.

> When the coast radio station where her mother vanished is scheduled for demolition, a
> burned-out sound engineer returns to her Kerala fishing village with seven days to restore a
> water-damaged reel that may prove her mother's last broadcast saved the fleet instead of
> dooming it.

| Character | Who | Played by |
|---|---|---|
| ANJALI | Anjali Thomas, 34, dialogue editor in Bengaluru | Nimisha Raghavan |
| APPACHAN | Thomas Kurian, 71, her grandfather, early dementia | K. P. Gopinathan |
| MARIAMMA | Her mother, coast radio operator in 1999 | Divya Mathew |
| YOUNG ANJALI | Anjali at eleven, 1999 | Ameya Suresh |
| FATHIMA | Childhood friend, runs the tea shop and the radio club | Sabitha Rasheed |
| RAHUL | Fathima's nephew, 16, builds transmitters | Aditya Nair |
| SURESH | Demolition site manager, son of skipper Varghese | Sajan Kuriakose |
| JOE | Joseph D'Cruz, station chief in 1999 | Lal Fernandez |
| LATHA | Inspector Latha Menon, coastal police | Reshmi Pillai |
| VARGHESE | Skipper of the Little Flower, 1999 | Thampi Cherian |

Day players: PORT OFFICER, STUDIO PRODUCER, OLD FISHERMAN, GUARD, MEENU, HARBOUR CLERK, NURSE.
All people, companies and contacts are invented; phone numbers and `.example` addresses are not real.

## What the project holds

- **Idea Vault**: treatment, research notes, a quote, two links, a research PDF, reference frames,
  a linked palette (relinked after the share moved), the director's voice memo (spoken with the
  macOS `say` tool), a sketch, folders, collections, tags, research flags, a Global Idea Vault note
  copied in, and items sent to the Story Board.
- **Story**: three acts, seven sequences, 44 scene cards with headings, descriptions, notes and Story
  Days, beats, the Parking Lot, fourteen characters (one archived) with pictures and relationships, card links,
  attachments and display settings.
- **Screenplay**: built from the cards, written scene by scene from `content/screenplay.fountain`
  (every element type, a custom Logbook Entry element, Malayalam dialogue in scene 10 with English
  subtitles as notes), title page and page setup, drafts (First, Second, Shooting Draft locked with
  frozen numbers, Blue Revision with scene 23A, an omitted scene 22 and revision marks), the
  writer's own Fountain file imported as a draft (with dual dialogue), a producer's review round,
  comments, private notes, and a short teaser trailer screenplay.
- **Production**: the shooting draft as Production Source, the breakdown of every scene in all
  eleven categories with a shared catalog, 19 cast entries (17 roles, one alternate, one archived),
  28 crew (one archived), 17 locations with every practical note and the nearest hospital,
  availability, one rejected location replaced, the production update to the blue pages.
- **Visual planning**: three moodboards, four storyboards (drawn sketches and imported frames),
  shot lists with lens, movement, angle and sound notes.
- **Schedule**: 16 calendar days (14 shooting days, 2 off days), meals, travel and company moves,
  day times, page counts and estimates, Day Out of Days with corrections, warnings decided.
- **Paperwork**: a finalized call sheet for every shooting day and blue-page revisions of each,
  sides, all six production reports, a budget in rupees with planned and actual amounts,
  10 percent contingency and a frozen snapshot.
- **Workspace**: project notes, tasks linked to records, templates (built-in, personal and
  project), files (copied and linked), history with undo and redo, Recently Deleted with restores
  and a purge, search, appearance theme, keyboard shortcuts, proofing languages and dictionary.
- **Offline AI** (when the build inputs are present): the real Gemma 3 model installed with the
  app's own import path, real questions, three proposals (applied and undone, rejected, applied
  and kept), and the AI report.
- **Packages**: a backup, the full project package, exchange packages for script review, story,
  breakdown, shots, schedule and the call sheet, and the producer's review done on a second
  installation and sent back as response packages.

## Rebuild it

```sh
RUSTC_WRAPPER= cargo run -p openframe-gold-project            # about 1.5 minutes with the AI
RUSTC_WRAPPER= cargo run -p openframe-gold-project -- --no-ai # without the Offline AI
```

`RUSTC_WRAPPER=` turns off sccache on a Mac without it. The AI phase needs
`.ai-assets/models/*/model.gguf` and `.build-cache/ai-runtime/macos-arm64/` (git-ignored build
inputs); without them it is skipped and COVERAGE.md says so. Each run starts from an empty data
root in `.work/` (git-ignored, about 1.4 GB with the AI models) and rewrites `outputs/`,
`COVERAGE.md` and `VERIFY.md`. Media files in `content/media/` are drawn once by the program and
kept, so their hashes stay stable.

## Open it in the app

File > Restore Backup or Package, choose `outputs/packages/Low Tide Radio.ofproject` and confirm.
The project opens with its own identity. `outputs/packages/Low Tide Radio (final).ofbackup` holds
the same state as a backup.

The real-app check opens it the same way and visits all thirteen workspaces:

```sh
cd apps/macos && swift build && swift build --product OpenFrameE2EDriver
E2E/run.sh .build/debug/OpenFrameStudio gold/
```

## Folders

| Path | What it is |
|---|---|
| `content/story.json` | Premise, logline, synopsis, title page, characters, relationships, acts, sequences, the 44 scene cards, beats, Parking Lot |
| `content/screenplay.fountain` | The full screenplay (Fountain), the source the program writes into the app |
| `content/breakdown.json` | The 1st AD's breakdown: page counts, time estimates and elements for every scene |
| `content/production.json` | Cast, crew, locations, the schedule, budget, notes and tasks |
| `content/coverage-notes.json` | Why the unused operations are unused, and what the product cannot do |
| `content/feature-map.json` | Each user-facing feature row of the feature status file mapped to its operations |
| `content/media/` | Frames, sketches, headshots, props, the voice memo and small documents, all made locally |
| `outputs/screenplay/` | PDF (clean, revision-marked, Act Three only), Final Draft, Fountain, Word, plain text |
| `outputs/story/`, `breakdown/`, `budget/`, `schedule/`, `callsheets/`, `sides/`, `reports/`, `visual/` | Every other export the app offers |
| `outputs/packages/` | Backup, full project package, exchange and response packages |
| `outputs/support/`, `outputs/ai/`, `outputs/settings/`, `outputs/files/` | Diagnostics bundle, AI report, exported theme, an exported project file |
| `outputs/coverage-counts.json` | Calls per operation, for regression checks |
| `app-screens/` | The real Mac app with the restored package, one picture per workspace (scenario `gold/open-every-workspace`) |
| `COVERAGE.md` | Every operation, used or not and why; feature coverage; what is broken or unsupported |
| `VERIFY.md` | The package import, backup restore, crash recovery and reopen compared with the original |

The program is `crates/openframe-gold-project`. It records every call; COVERAGE.md is written from
those counts, never by hand.
