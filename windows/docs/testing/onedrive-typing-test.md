# Manual test: ten minutes of typing in a OneDrive folder

> Windows phase W2 ([plan](../roadmap/chapter-1a-windows-phases.md), "Cloud-synced folders"). The owner runs this by hand
> on a laptop whose Documents folder is backed up by OneDrive. It checks what automated tests cannot: the real OneDrive
> client locking or replacing files while OpenFrame writes. Written 2026-10-16. Result goes in the progress log with the
> date, the laptop and the OneDrive version.

## Before you start

- [ ] A Windows 11 laptop signed in to OneDrive, with "Documents" turned on under OneDrive, Settings, Sync and back up,
      Manage back up. The OneDrive icon in the taskbar shows "Up to date".
- [ ] OpenFrame Studio installed or started from the development build. It is the only copy running.
- [ ] Note the OneDrive version (OneDrive icon, Settings, About) and the time.

## Steps

1. [ ] On Application Home choose **New Project**. Look at the location line.
   - Expected: a note says the folder is synced by OneDrive and that syncing while you write can lock or damage project
     files. A button offers a folder on this computer instead.
2. [ ] Do **not** use the button. Create the project in the OneDrive folder anyway, named "OneDrive Test".
3. [ ] Open **Screenplay**, create a screenplay, and type for 10 minutes without stopping: scene headings, action,
   character names and dialogue. Every minute or so press Ctrl+S once.
4. [ ] During the 10 minutes, twice: right-click the OneDrive icon, choose **Pause syncing, 2 hours**, keep typing for a
   minute, then **Resume syncing**.
5. [ ] At the end, type one last sentence and close the window at once with the close button.
6. [ ] Wait until OneDrive shows "Up to date". Start OpenFrame Studio again and open "OneDrive Test".
7. [ ] Open **New Project** again, press the button that offers a folder on this computer, create a second project, and
   check that its location line shows no OneDrive note.

## Pass when all of these are true

- [ ] No message about a locked file, a file in use or a save error appeared at any time. (If one did, write down its
      exact words and the time.)
- [ ] After reopening, the last sentence from step 5 is there, and the screenplay holds everything typed.
- [ ] No recovery question appeared on reopening.
- [ ] The OneDrive folder holds no files named like `project-COMPUTERNAME.sqlite` or `project (1).sqlite` (OneDrive
      conflict copies) inside `OneDrive Test.openframe`.
- [ ] The existing projects were not moved: the first project is still where it was created.

## If it fails

Keep the project folder as it is (do not open it again), take a screenshot of any message, and write a bug report with
[bug-report-template.md](bug-report-template.md). Attach the logs from Help, Show Logs.
