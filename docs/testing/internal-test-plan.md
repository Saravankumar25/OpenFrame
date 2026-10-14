# Internal test plan: writers and production teams, 2 to 3 days

> Written 2026-10-09. This is the plan for the first real-world test of the internal Windows `.exe` and macOS `.dmg`. The
> testers are working writers, screenwriters and production people, not developers. They install OpenFrame on their own
> computers and use it as their main tool for 2 to 3 days. Nothing here starts before the owner says the installers may be
> built and gives the invitation list.

## 1. Purpose and limits

- Find the problems that tests on the development machine cannot find: installation, other computers, real scripts, real
  habits, real mistakes.
- Check that a person can finish the main workflows without help.
- Check that no work is ever lost.
- Not a marketing event and not a price test. The build has no trial, no purchase and no price.

Installation warnings are not defects. The builds are not signed or notarized, so Windows SmartScreen and macOS Gatekeeper
warn on first run. Testers are told this in advance and given the steps below.

## 2. Who

- 4 to 8 testers: at least two screenwriters, one script supervisor or producer, one person who schedules shoots, and at
  least one on a Mac. Include at least one tester who writes in Telugu, Hindi or another Indian language.
- At least one tester on an ordinary laptop with 8 GB of memory.
- The owner is the first tester on the owner's own Windows and Mac laptops, one day earlier.
- One person (the owner or a delegate) collects reports and answers questions.

## 3. Before the pilot (the owner and the team)

1. Build the installers with the owner's word; record the version, the build date and the SHA-256 of each file.
2. Install on two clean computers (one Windows, one Mac) and run the checklist in section 7. Fix blockers first.
3. Decide the team password question. If the gate stays, send the password separately from the installer.
4. Prepare a small set of safe sample projects (the four sample scripts) so a tester can try things without risking their
   own work on day one.
5. Send each tester: the installer, its checksum, the one-page install guide (section 5), this plan's daily tasks, the bug
   report template, and the support contact.

## 4. Rules for testers

- Keep your own copy of anything important. OpenFrame saves continuously and keeps recovery points, but this is a test
  build. Use Create Backup before large changes and keep the backup file somewhere else.
- Work on real tasks, but start with a copy of a script, not the only copy.
- Do not send us your script. If you want to show a problem, describe it or use a sample script.
- If something looks wrong, stop and write it down. Do not try to fix it by deleting files.
- If the program crashes or you see "recovery", choose the option that keeps your latest work, then report it.

## 5. Install guide (one page for testers)

### Windows 11

1. Download the installer and check the SHA-256 value we sent (right-click the file, or `Get-FileHash` in PowerShell).
2. Double-click the installer. If a blue "Windows protected your PC" window appears, click **More info**, then **Run
   anyway**. This appears because the build is not signed yet.
3. Finish the installation. It installs for your user only and needs no administrator rights.
4. Start OpenFrame Studio. If asked, type the team password.
5. Allow the first start a moment: the AI engine starts when you first use the assistant.

### Apple silicon Mac

1. Download the disk image and check the checksum (`shasum -a 256 file.dmg` in Terminal).
2. Open the disk image and drag OpenFrame Studio to Applications.
3. The first time, macOS may say it cannot check the app for malicious software. Open **System Settings, Privacy and
   Security**, scroll down and click **Open Anyway** next to OpenFrame Studio (older macOS versions: Control-click the app,
   choose **Open**, then **Open**). The exact steps depend on your macOS version; the owner confirms them on the Mac before
   the pilot.
4. Start the app. If asked, type the team password.

### Uninstall

Windows: Settings, Apps, OpenFrame Studio, Uninstall. Mac: drag the app to the Bin. Your projects in
`Documents\OpenFrame` stay; delete them yourself only if you are sure.

## 6. Daily tasks

Each day takes about 90 minutes of focused testing plus normal use of the app for real work.

### Day 1: get started and write

1. Install and start the app. Note how long it took and anything confusing.
2. Create a project. Choose a name. Close it. Reopen it from Home.
3. Open the Screenplay. Type five scenes: headings, action, a character speaking, dialogue, a parenthetical, a
   transition. Use Tab and Enter the way you do in your usual tool.
4. Press Ctrl+Z several times and Ctrl+Y (Command on Mac). Check nothing odd happens.
5. Close the window right after typing. Reopen. Is every word there?
6. Switch spell check on and type a few misspelled words. If you write in Telugu or Hindi, type a paragraph.
7. Import a script you already have (Final Draft, Fountain, Word or PDF) as a copy. Check scenes, character names,
   dialogue and title page.
8. Export it back out to PDF and to Final Draft. Open the files in your usual tool.

### Day 2: plan and organise

1. In the Story Board make acts and scene cards. Build a screenplay from the cards. Change the order and check the warning.
2. Use the Idea Vault: add a note, an image, a web link, a voice note. Send one item to the Story Board.
3. Open Breakdown for a script. Tag props, wardrobe and locations in a few scenes. Accept and reject suggestions.
4. Make a shooting schedule with two days. Move scenes between days. Add a meal break.
5. Create a call sheet from a day. Change something in the schedule and see that the call sheet says it needs a refresh.
6. Make a backup. Restore it as a copy. Check it is complete.
7. Try the AI assistant: ask for a list of the scenes in one location, ask to rename a character, ask for a task. Read the
   proposed changes. Apply one. Undo it. Reject another.

### Day 3: stress and finish

1. Open the biggest script you have (more than 100 pages if possible). Type in the middle. Watch for slowness.
2. Leave the app open for an hour while working elsewhere. Put the computer to sleep and wake it.
3. Work with the network switched off for ten minutes, including the AI.
4. Try something unusual: very long names, special characters, pasting from Word, a project on an external drive.
5. Use Recently Deleted: delete a card, restore it.
6. Export the diagnostics file from Settings, General, Support and attach it to any bug report you write.
7. Fill in the end-of-pilot questions (section 9).

## 7. Clean-machine checklist (run by the owner before the pilot)

Windows: install; first start; create, type, close, reopen; AI with network off; export PDF; backup and restore on a second
account; upgrade over the older build; uninstall; reinstall; installer size and time; memory use while the AI runs; the
WebView2 runtime present; SmartScreen steps as written.

Mac: copy to Applications; first start with the Gatekeeper steps as written; the same functional steps; Command key
shortcuts; spell check; file dialogs; quit with Command+Q right after typing; AI with network off; memory use.

Both: the bundled AI files pass their check at first start; no network connection other than loopback is opened by the
app; logs contain no script text.

## 8. Reporting and fixing

1. A tester writes a report with the [template](bug-report-template.md), one problem per report, with the diagnostics file
   when possible.
2. The owner or delegate gives each report a severity:
   - **Blocker:** work is lost or damaged, the program cannot start or open a project, a security or privacy problem.
   - **Serious:** a main workflow cannot be finished, or a result is wrong.
   - **Annoying:** the workflow can be finished but is confusing, slow or ugly.
   - **Idea:** a wish, not a defect.
3. Blockers stop the pilot for that tester until a fix or a workaround exists. Serious reports are fixed before the next
   build.
4. Every fix gets a test that fails without it, then passes. The reporter re-checks it in the next build.
5. Installation warnings (SmartScreen, Gatekeeper) are logged separately as "installation restrictions", not defects.

## 9. End-of-pilot questions

1. Which tasks could you do without help? Which could you not?
2. Did you lose any work, even a little?
3. Where did the program feel slow?
4. Which feature did you use most? Which did you never find?
5. Did the AI help? Where was it wrong?
6. Would you use it for a real project next week? What stops you?

## 10. Finishing the pilot

The pilot ends when: every Blocker is fixed and re-checked, no Serious report is open without a decision, the end-of-pilot
answers are collected, and the owner signs off in [the progress log](../implementation/progress-log.md). A short summary of
findings is stored under `docs/release/evidence/`.
