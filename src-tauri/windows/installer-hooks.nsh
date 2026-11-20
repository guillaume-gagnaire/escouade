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

; An update starts while the app that launched it is still ending: the app runs the installer, then
; exits, and Windows takes a while to tear a big process down. Tauri's check (`CheckIfAppIsRunning`,
; just after `NSIS_HOOK_PREINSTALL`) asks the Restart Manager to end what still holds the binary; a
; process the system is already tearing down cannot be ended (`RmShutdown` gives
; ERROR_FAIL_SHUTDOWN, event 10006 « Unknown App », status 17), and the silent installer then gives
; up without a word (ESC-533). So an update first waits for the binary to be free, a step at a time,
; `ESCOUADE_EXIT_WAIT_MS` at most: an app that really stayed open is then closed by Tauri's check as
; before. An install by hand does not wait.
!ifndef ESCOUADE_EXIT_STEP_MS
  !define ESCOUADE_EXIT_STEP_MS 250
!endif
!ifndef ESCOUADE_EXIT_WAIT_MS
  !define ESCOUADE_EXIT_WAIT_MS 30000
!endif
; The app an update restarts (`/R`), set once the install runs: `.onInstFailed`, above the
; template's defines, only knows it so.
Var RestartedApp
Var ExitSession
Var ExitStart

; Waits until no process holds the program at `path` (an update's app ending), `ESCOUADE_EXIT_WAIT_MS`
; at most, by the Restart Manager as Tauri's check after it. A new session each step: within one,
; `RmGetList` keeps giving what it first found. (Opening the program for writing is no test: under
; load, the antivirus that reads a program just run keeps it so for a long while.)
!macro EscouadeWaitFree path
  Push $0
  Push $1
  Push $2
  Push $3
  System::Call 'kernel32::GetTickCount() i .s'
  Pop $ExitStart
  ${Do}
    StrCpy $0 ""
    !insertmacro RestartManager_StartSession $ExitSession
    ${If} $ExitSession != ""
      !insertmacro RestartManager_RegisterFile $ExitSession "${path}"
      ${If} $0 = 0
        System::Call 'RSTRTMGR::RmGetList(i $ExitSession, *i .r1, *i .r2, p 0, *i .r3) i .r0'
      ${EndIf}
      !insertmacro RestartManager_EndSession $ExitSession
    ${EndIf}
    ; ERROR_MORE_DATA: some process holds it still. Anything else: Tauri's check sees to it.
    ${IfThen} $0 <> ${ERROR_MORE_DATA} ${|} ${Break} ${|}
    System::Call 'kernel32::GetTickCount() i .r2'
    IntOp $2 $2 - $ExitStart
    ${IfThen} $2 >= ${ESCOUADE_EXIT_WAIT_MS} ${|} ${Break} ${|}
    Sleep ${ESCOUADE_EXIT_STEP_MS}
  ${Loop}
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

; The other programs of the folder can run on their own when the installer starts (a hook of a
; Claude Code session started outside Escouade, a plugin command in a terminal): a file in use cannot
; be replaced, and the silent installer skips it without a word, the folder half up to date. Windows
; lets a running program be renamed: it is set aside (`<name>.<n>.old`, its process running on), and
; the new one written in its place; `aside` keeps where (empty: nothing set aside), so that an install
; that fails after puts it back. What was set aside before goes once nothing runs it any more.
Var HookAside
Var PluginAside

!macro EscouadeSetAside name aside
  Push $0
  Push $1
  StrCpy ${aside} ""
  Delete "$INSTDIR\${name}.*.old"
  ${If} ${FileExists} "$INSTDIR\${name}"
    ClearErrors
    FileOpen $0 "$INSTDIR\${name}" a
    ${If} ${Errors}
      StrCpy $1 0
      ${Do}
        IntOp $1 $1 + 1
        ClearErrors
        Rename "$INSTDIR\${name}" "$INSTDIR\${name}.$1.old"
        ${IfNot} ${Errors}
          StrCpy ${aside} "$INSTDIR\${name}.$1.old"
          ${Break}
        ${EndIf}
        ${IfThen} $1 >= 9 ${|} ${Break} ${|}
      ${Loop}
    ${Else}
      FileClose $0
    ${EndIf}
  ${EndIf}
  Pop $1
  Pop $0
!macroend

; Puts back what `EscouadeSetAside` set aside, when its place is still free.
!macro EscouadePutBack name aside
  ${If} ${aside} != ""
  ${AndIfNot} ${FileExists} "$INSTDIR\${name}"
    Rename "${aside}" "$INSTDIR\${name}"
  ${EndIf}
!macroend

; An install that did not go through (Tauri's check gave up) puts back what was set aside. An update
; that failed so left the user with no app at all: the app it was to restart starts again, on the
; version it had, and tells that the update did not install, with « Réessayer ». Only with `/R`, as
; `.onInstSuccess`.
Function .onInstFailed
  !insertmacro EscouadePutBack "escouade-hook.exe" $HookAside
  !insertmacro EscouadePutBack "escouade-plugin.exe" $PluginAside
  ${If} $RestartedApp != ""
    ClearErrors
    ${GetOptions} $CMDLINE "/R" $R0
    ${IfNot} ${Errors}
      ${GetOptions} $CMDLINE "/ARGS" $R0
      ; Tauri's plugin (RunAsUser) is only declared below these hooks; installed for the current
      ; user (`installMode`), the installer is never elevated, so the app runs as the user anyway.
      Exec '"$RestartedApp" $R0'
    ${EndIf}
  ${EndIf}
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  ${If} $UpdateMode = 1
    ${If} ${Silent}
    ${OrIf} $PassiveMode = 1
      StrCpy $RestartedApp "$INSTDIR\${MAINBINARYNAME}.exe"
    ${EndIf}
    !insertmacro EscouadeWaitFree "$INSTDIR\${MAINBINARYNAME}.exe"
  ${EndIf}
  !insertmacro EscouadeSetAside "escouade-hook.exe" $HookAside
  !insertmacro EscouadeSetAside "escouade-plugin.exe" $PluginAside
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

!macro NSIS_HOOK_POSTUNINSTALL
  ; What an install set aside (`EscouadeSetAside`) goes with the app, and so does its folder, once
  ; empty (the template removed it before, when nothing else was left in it).
  Delete "$INSTDIR\escouade-hook.exe.*.old"
  Delete "$INSTDIR\escouade-plugin.exe.*.old"
  RMDir "$INSTDIR"
!macroend
