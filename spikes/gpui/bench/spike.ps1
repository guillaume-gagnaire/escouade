# Le banc du prototype (ESC-365) : démarrage, gros flux, dix terminaux. Résultats dans results/.
param([int]$Launches = 5, [int]$Floods = 3, [int]$Lines = 100000)
$ErrorActionPreference = 'Stop'
[cultureinfo]::CurrentCulture = [cultureinfo]::InvariantCulture
$d = Split-Path -Parent $PSScriptRoot
$exe = "$d\target\release\gpui-spike.exe"
$res = "$d\results"; New-Item -ItemType Directory -Force $res | Out-Null
$out = "$res\spike.jsonl"; Remove-Item $out -ErrorAction SilentlyContinue
function Now { [DateTimeOffset]::Now.ToUnixTimeMilliseconds() }
function Clear-Env { Remove-Item Env:SPIKE_BENCH, Env:SPIKE_QUIT_AFTER, Env:SPIKE_TERMINALS, Env:SPIKE_FLOOD_LINES -ErrorAction SilentlyContinue }

# 1. Démarrage : du lancement du process à la première frame peinte, et la mémoire au repos 10 s après.
Clear-Env
for ($i = 1; $i -le $Launches; $i++) {
  $env:SPIKE_OUT = "$res\startup-$i.jsonl"; Remove-Item $env:SPIKE_OUT -ErrorAction SilentlyContinue
  $t0 = Now; $p = Start-Process -FilePath $exe -PassThru
  Start-Sleep -Seconds 10
  $s = Get-Content $env:SPIKE_OUT | ConvertFrom-Json | Where-Object kind -eq 'startup'
  $m = (& "$PSScriptRoot\mem.ps1" -Root $p.Id | Select-Object -Last 1) | ConvertFrom-Json
  if ($i -eq 1) { & "$PSScriptRoot\shot.ps1" -ProcessId $p.Id -Out "$res\spike-repos.png" | Out-Null }
  Stop-Process -Id $p.Id -Force
  $line = @{ kind = 'startup'; launch = $i; window_ms = $s.first_frame_epoch_ms - $t0; since_main_ms = $s.since_main_ms; rest = $m } | ConvertTo-Json -Compress -Depth 4
  Add-Content $out $line; $line
  Start-Sleep -Seconds 2
}

# 2. Le gros flux dans le terminal : $Lines lignes colorées, par PowerShell, comme dans l'app.
for ($i = 1; $i -le $Floods; $i++) {
  Clear-Env
  $env:SPIKE_BENCH = 'flood'; $env:SPIKE_FLOOD_LINES = "$Lines"
  $env:SPIKE_OUT = "$res\flood-$i.jsonl"; Remove-Item $env:SPIKE_OUT -ErrorAction SilentlyContinue
  $p = Start-Process -FilePath $exe -PassThru
  $cpu0 = $null; $peak = 0; $until = (Now) + 120000; $shot = $false
  while ((Now) -lt $until) {
    Start-Sleep -Milliseconds 250
    $p.Refresh(); if ($p.HasExited) { break }
    if ($null -eq $cpu0) { $cpu0 = $p.TotalProcessorTime.TotalMilliseconds }
    $peak = [Math]::Max($peak, $p.WorkingSet64)
    if (-not $shot -and (Test-Path $env:SPIKE_OUT) -and ((Get-Content $env:SPIKE_OUT) -match 'flood')) { break }
    if (-not $shot -and ((Now) - $p.StartTime.ToUniversalTime().Subtract([datetime]'1970-01-01').TotalMilliseconds) -gt 5500) {
      & "$PSScriptRoot\shot.ps1" -ProcessId $p.Id -Out "$res\spike-flux-$i.png" | Out-Null; $shot = $true
    }
  }
  Start-Sleep -Milliseconds 500
  $p.Refresh()
  $cpu = $p.TotalProcessorTime.TotalMilliseconds - $cpu0
  $m = (& "$PSScriptRoot\mem.ps1" -Root $p.Id | Select-Object -Last 1) | ConvertFrom-Json
  & "$PSScriptRoot\shot.ps1" -ProcessId $p.Id -Out "$res\spike-apres-flux-$i.png" | Out-Null
  $peakWs = $p.PeakWorkingSet64 / 1MB
  Stop-Process -Id $p.Id -Force
  $f = Get-Content $env:SPIKE_OUT | ConvertFrom-Json | Where-Object kind -eq 'flood'
  $line = @{ kind = 'flood'; run = $i; lines = $Lines; result = $f; ui_cpu_ms = [Math]::Round($cpu); ui_peak_ws_mb = [Math]::Round($peakWs, 1); after = $m } | ConvertTo-Json -Compress -Depth 4
  Add-Content $out $line; $line
  Start-Sleep -Seconds 2
}

# 3. Dix terminaux ouverts, au repos.
Clear-Env
$env:SPIKE_TERMINALS = '10'; $env:SPIKE_OUT = "$res\ten.jsonl"
$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 12
$m = (& "$PSScriptRoot\mem.ps1" -Root $p.Id | Select-Object -Last 1) | ConvertFrom-Json
& "$PSScriptRoot\shot.ps1" -ProcessId $p.Id -Out "$res\spike-10-terminaux.png" | Out-Null
Stop-Process -Id $p.Id -Force
$line = @{ kind = 'ten-terminals'; rest = $m } | ConvertTo-Json -Compress -Depth 4
Add-Content $out $line; $line
Get-Process powershell -ErrorAction SilentlyContinue | Where-Object { $_.Parent -eq $null } | Out-Null
