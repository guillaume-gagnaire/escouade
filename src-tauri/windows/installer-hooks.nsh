; Escouade was called « Claude Code Manager » up to 0.1.2, and the installer names its folder,
; its uninstall entry and its shortcuts after the product: installed over it (the update from
; 0.1.2), Escouade removes that install first, keeping the user's data, then creates its own
; shortcuts, which an update does not create.
;
; The installer speaks the system's language (English or French, `languages` in tauri.conf.json).
; These hooks show no text of their own: what `CheckIfAppIsRunning` asks comes from Tauri's
; strings, in both languages. A message added here needs a `LangString` for each.

!ifndef OLD_PRODUCTNAME
  !define OLD_PRODUCTNAME "Claude Code Manager"
!endif
!define OLD_UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${OLD_PRODUCTNAME}"

Var OldInstallDir
Var OldUninstaller
Var OldBinary
Var OldQuote
Var OldResult
Var OldHadDesktopShortcut
Var CameFromOldName

!macro NSIS_HOOK_PREINSTALL
  StrCpy $CameFromOldName 0
  StrCpy $OldHadDesktopShortcut 0
  ReadRegStr $OldUninstaller HKCU "${OLD_UNINSTKEY}" "UninstallString"
  ReadRegStr $OldInstallDir HKCU "${OLD_UNINSTKEY}" "InstallLocation"
  ReadRegStr $OldBinary HKCU "${OLD_UNINSTKEY}" "MainBinaryName"
  ${If} $OldBinary == ""
    StrCpy $OldBinary "claude-code-manager.exe"
  ${EndIf}
  ${If} $OldUninstaller != ""
  ${AndIf} $OldInstallDir != ""
    StrCpy $CameFromOldName 1
    ; The location is stored between quotes.
    StrCpy $OldQuote $OldInstallDir 1
    ${If} $OldQuote == '"'
      StrCpy $OldInstallDir $OldInstallDir -1 1
    ${EndIf}
    ${If} ${FileExists} "$DESKTOP\${OLD_PRODUCTNAME}.lnk"
      StrCpy $OldHadDesktopShortcut 1
    ${EndIf}
    ${If} ${FileExists} "$OldInstallDir\uninstall.exe"
      ; Closes the old app if it runs (asking first when installing by hand).
      !insertmacro CheckIfAppIsRunning "$OldInstallDir\$OldBinary" "${OLD_PRODUCTNAME}"
      ; Silent, so the app's data stays (it is only deleted when asked, in the dialog). `_?=` runs
      ; the uninstaller in place, so that ExecWait waits for it; it cannot delete itself there.
      ClearErrors
      ExecWait '$OldUninstaller /S _?=$OldInstallDir' $OldResult
      ; Cleaned up only once the old app is really gone: otherwise it stays whole, uninstallable.
      ${IfNot} ${Errors}
      ${AndIf} $OldResult = 0
      ${AndIfNot} ${FileExists} "$OldInstallDir\$OldBinary"
        Delete "$OldInstallDir\uninstall.exe"
        RMDir "$OldInstallDir"
        DeleteRegKey HKCU "Software\${MANUFACTURER}\${OLD_PRODUCTNAME}"
      ${EndIf}
    ${ElseIfNot} ${FileExists} "$OldInstallDir\$OldBinary"
      ; Its folder is already gone: only its entry and shortcuts are left.
      DeleteRegKey HKCU "${OLD_UNINSTKEY}"
      DeleteRegKey HKCU "Software\${MANUFACTURER}\${OLD_PRODUCTNAME}"
      Delete "$SMPROGRAMS\${OLD_PRODUCTNAME}.lnk"
      Delete "$DESKTOP\${OLD_PRODUCTNAME}.lnk"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 0.1.4 changed the app identifier, which Windows goes by to show notifications: the shortcuts
  ; an update keeps get the current one. (No start menu folder is configured: shortcuts sit at
  ; the root of the start menu.)
  ${If} $UpdateMode = 1
    ${If} ${FileExists} "$SMPROGRAMS\${PRODUCTNAME}.lnk"
      !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}
    ${If} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
      !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
  ${EndIf}
  ; Coming from Claude Code Manager, the app gets its own shortcuts (an update creates none), even
  ; when the old app could not be removed.
  ${If} $CameFromOldName = 1
  ${AndIf} $UpdateMode = 1
    ${IfNot} ${FileExists} "$SMPROGRAMS\${PRODUCTNAME}.lnk"
      CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
      !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}
    ${If} $OldHadDesktopShortcut = 1
    ${AndIfNot} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
      CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
      !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
  ${EndIf}
!macroend
