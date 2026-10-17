; Uninstaller behaviour for the internal Windows installer (Tauri NSIS hooks).
;
; Uninstalling removes the program. It never touches projects or the Global Idea Vault.
; The "delete app data" checkbox (Tauri's $DeleteAppDataCheckboxState) removes only the
; app data folder %LOCALAPPDATA%\OpenFrame: settings, recent projects, downloaded AI models, logs.
;
; Why that is safe: a project folder or the vault can never be placed inside the app data
; folder. check_project_parent (openframe-application/src/util.rs) and the other output
; checks call openframe_security::validate_output_dir / validate_output_file, and
; protected_roots() lists app_data_dir first, so a path inside it is refused.

!macro NSIS_HOOK_PREUNINSTALL
  ; An update runs the old uninstaller silently; only a person who chose to uninstall sees this.
  ${If} $UpdateMode <> 1
  ${AndIfNot} ${Silent}
    MessageBox MB_OK|MB_ICONINFORMATION "Your projects and your Global Idea Vault are not removed. They stay where you keep them, usually in Documents\OpenFrame."
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    RMDir /r "$LOCALAPPDATA\OpenFrame"
  ${EndIf}
!macroend
