; Uninstaller behaviour for the internal Windows installer (Tauri NSIS hooks).
;
; Uninstalling removes the program. It never touches projects, the Global Idea Vault or the library
; file (Documents\OpenFrame\library.json, ADR-0038), which keeps the person's identity, so a
; reinstall finds their work and their private notes again even when the app data was deleted.
; The "delete app data" checkbox (Tauri's $DeleteAppDataCheckboxState) removes only the items the
; app itself creates inside %LOCALAPPDATA%\OpenFrame, then the folder itself if it is empty:
;   folders: models, ai, logs, entitlement, tmp, reviews
;   files:   app.sqlite (with its -wal, -shm and -journal side files), engines.json
; Any other item in that folder is never removed, and then the folder stays. The root is never
; deleted recursively, so a project or anything else a person once placed there survives.
; The folder list is checked against the code by scripts/tests/governance.test.mjs.
;
; Projects are normally kept out of this folder: check_project_parent (openframe-application/src/util.rs)
; refuses it, because protected_roots() lists app_data_dir first. Older builds, received packages and
; hand-made folders were not covered by that check, hence the explicit list.

!macro NSIS_HOOK_PREUNINSTALL
  ; An update runs the old uninstaller silently; only a person who chose to uninstall sees this.
  ${If} $UpdateMode <> 1
  ${AndIfNot} ${Silent}
    MessageBox MB_OK|MB_ICONINFORMATION "Your projects, your Global Idea Vault and your OpenFrame library file are not removed. They stay where you keep them, usually in Documents\OpenFrame, and installing OpenFrame again finds them."
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    RMDir /r "$LOCALAPPDATA\OpenFrame\models"
    RMDir /r "$LOCALAPPDATA\OpenFrame\ai"
    RMDir /r "$LOCALAPPDATA\OpenFrame\logs"
    RMDir /r "$LOCALAPPDATA\OpenFrame\entitlement"
    RMDir /r "$LOCALAPPDATA\OpenFrame\tmp"
    RMDir /r "$LOCALAPPDATA\OpenFrame\reviews"
    Delete "$LOCALAPPDATA\OpenFrame\app.sqlite"
    Delete "$LOCALAPPDATA\OpenFrame\app.sqlite-wal"
    Delete "$LOCALAPPDATA\OpenFrame\app.sqlite-shm"
    Delete "$LOCALAPPDATA\OpenFrame\app.sqlite-journal"
    Delete "$LOCALAPPDATA\OpenFrame\engines.json"
    ; Not recursive: succeeds only when nothing else is left inside.
    RMDir "$LOCALAPPDATA\OpenFrame"
  ${EndIf}
!macroend
