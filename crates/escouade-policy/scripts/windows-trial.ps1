# The real trial of TS-12 on Windows, to run in a PowerShell **as an administrator** (UAC), from the repository's root:
#   powershell -ExecutionPolicy Bypass -File crates/escouade-policy/scripts/windows-trial.ps1
# 1. Copies the generated ADMX and its ADML into C:\Windows\PolicyDefinitions (backing up what was there).
# 2. Writes a few policies in HKLM and HKCU\SOFTWARE\Policies\Escouade, the way Group Policy writes them from the ADMX,
#    and prints what escouade-policy reads, then removes them.
# 3. Says what to check by hand in gpedit.msc (the GPO itself), then `-Cleanup` removes the ADMX.
param([switch]$Cleanup)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\..\..\.."
$defs = 'C:\Windows\PolicyDefinitions'
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw 'Run it as an administrator: PolicyDefinitions and the Policies keys are theirs alone.'
}
if ($Cleanup) {
  Remove-Item "$defs\escouade.admx", "$defs\fr-FR\escouade.adml", "$defs\en-US\escouade.adml" -ErrorAction SilentlyContinue
  Remove-Item 'HKLM:\SOFTWARE\Policies\Escouade', 'HKCU:\SOFTWARE\Policies\Escouade' -Recurse -ErrorAction SilentlyContinue
  'Removed.'
  return
}
Copy-Item "$root\policies\windows\escouade.admx" $defs -Force
New-Item -ItemType Directory -Force "$defs\fr-FR", "$defs\en-US" | Out-Null
Copy-Item "$root\policies\windows\fr-FR\escouade.adml" "$defs\fr-FR" -Force
Copy-Item "$root\policies\windows\en-US\escouade.adml" "$defs\en-US" -Force
'ADMX and ADML copied.'

# What Group Policy writes for: « Éteindre les statistiques d'usage » (machine), « Modes de permission permis » (a list,
# machine), « Couper la mise à jour automatique » and « Canal stable seulement » (user), « Modèles permis » (JSON, user).
reg add 'HKLM\SOFTWARE\Policies\Escouade' /v telemetry.enabled /t REG_DWORD /d 0 /f | Out-Null
reg add 'HKLM\SOFTWARE\Policies\Escouade\permissions.modes' /v 1 /t REG_SZ /d plan /f | Out-Null
reg add 'HKLM\SOFTWARE\Policies\Escouade\permissions.modes' /v 2 /t REG_SZ /d default /f | Out-Null
reg add 'HKCU\SOFTWARE\Policies\Escouade' /v updates.automatic /t REG_DWORD /d 0 /f | Out-Null
reg add 'HKCU\SOFTWARE\Policies\Escouade' /v updates.channel /t REG_SZ /d stable /f | Out-Null
reg add 'HKCU\SOFTWARE\Policies\Escouade' /v telemetry.enabled /t REG_DWORD /d 1 /f | Out-Null
reg add 'HKCU\SOFTWARE\Policies\Escouade' /v harnesses.allowedModels /t REG_SZ /d '{\"claude\": [\"claude-sonnet-5-5\"]}' /f | Out-Null
'--- the ACL of the keys, as written:'
(Get-Acl 'HKLM:\SOFTWARE\Policies\Escouade').Access | Format-Table IdentityReference, RegistryRights -AutoSize
(Get-Acl 'HKCU:\SOFTWARE\Policies\Escouade').Access | Format-Table IdentityReference, RegistryRights -AutoSize
'--- what escouade-policy reads (expected: telemetry.enabled false from the machine, permissions.modes [plan, default],'
'    updates.automatic false and updates.channel stable from the user, the models as an object):'
Push-Location "$root\src-tauri"
cargo run -q -p escouade-policy --example read
Pop-Location
Remove-Item 'HKLM:\SOFTWARE\Policies\Escouade', 'HKCU:\SOFTWARE\Policies\Escouade' -Recurse
'Keys removed. Now by hand, in gpedit.msc (Modèles d''administration › Escouade, computer then user side):'
'  - enable « Interdire le mode Bypass », « Modes de permission permis » (plan, default), « Canal stable seulement »;'
'  - run `cargo run -p escouade-policy --example read -- --watch` in another console (not elevated) and change one:'
'    it prints the new policies within a few seconds (RegNotifyChangeKeyValue);'
'  - check the values gpedit wrote: reg query HKLM\SOFTWARE\Policies\Escouade /s'
'  - start Escouade (not elevated): Réglages show the settings greyed, « Imposé par la stratégie de ce poste ».'
'Then: powershell -ExecutionPolicy Bypass -File crates/escouade-policy/scripts/windows-trial.ps1 -Cleanup'
