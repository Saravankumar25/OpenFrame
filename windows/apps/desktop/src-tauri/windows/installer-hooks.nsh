; Uninstaller behaviour for the internal Windows installer (Tauri NSIS hooks).
;
; Uninstalling removes the program. It never touches projects, the Global Idea Vault or the library
; file (Documents\Straft Studio\library.json, ADR-0038), which keeps the person's identity, so a
; reinstall finds their work and their private notes again even when the app data was deleted.
; The "delete app data" checkbox (Tauri's $DeleteAppDataCheckboxState) removes only the items the
; app itself creates inside %LOCALAPPDATA%\Straft, then the folder itself if it is empty:
;   folders: models, ai, logs, entitlement, tmp, reviews
;   files:   app.sqlite (with its -wal, -shm and -journal side files), engines.json
; Any other item in that folder is never removed, and then the folder stays. The root is never
; deleted recursively, so a project or anything else a person once placed there survives.
; The data folder of the earlier product name is never touched: that app may still be in use, and
; its folder may be the only copy after a failed first start.
; The folder list is checked against the code by scripts/tests/governance.test.mjs.
;
; Projects are normally kept out of this folder: check_project_parent (straft-application/src/util.rs)
; refuses it, because protected_roots() lists app_data_dir first. Older builds, received packages and
; hand-made folders were not covered by that check, hence the explicit list.

!macro NSIS_HOOK_PREUNINSTALL
  ; An update runs the old uninstaller silently; only a person who chose to uninstall sees this.
  ${If} $UpdateMode <> 1
  ${AndIfNot} ${Silent}
    MessageBox MB_OK|MB_ICONINFORMATION "Your projects, your Global Idea Vault and your Straft library file are not removed. They stay where you keep them, usually in Documents\Straft Studio, and installing Straft again finds them."
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    RMDir /r "$LOCALAPPDATA\Straft\models"
    RMDir /r "$LOCALAPPDATA\Straft\ai"
    RMDir /r "$LOCALAPPDATA\Straft\logs"
    RMDir /r "$LOCALAPPDATA\Straft\entitlement"
    RMDir /r "$LOCALAPPDATA\Straft\tmp"
    RMDir /r "$LOCALAPPDATA\Straft\reviews"
    Delete "$LOCALAPPDATA\Straft\app.sqlite"
    Delete "$LOCALAPPDATA\Straft\app.sqlite-wal"
    Delete "$LOCALAPPDATA\Straft\app.sqlite-shm"
    Delete "$LOCALAPPDATA\Straft\app.sqlite-journal"
    Delete "$LOCALAPPDATA\Straft\engines.json"
    ; Not recursive: succeeds only when nothing else is left inside.
    RMDir "$LOCALAPPDATA\Straft"
  ${EndIf}
!macroend
