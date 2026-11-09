# Dix terminaux (ESC-365) : au repos, puis tous en gros flux à la fois ($Lines lignes chacun).
param([int]$Terminals = 10, [int]$Lines = 20000, [int]$Runs = 3, [string]$OutName = "spike.jsonl")
[cultureinfo]::CurrentCulture = [cultureinfo]::InvariantCulture
$d = Split-Path -Parent $PSScriptRoot
$exe = "$d\target\release\gpui-spike.exe"; $res = "$d\results"; $out = "$res\$OutName"
Remove-Item Env:SPIKE_BENCH, Env:SPIKE_QUIT_AFTER, Env:SPIKE_FLOOD_LINES -ErrorAction SilentlyContinue
$env:SPIKE_TERMINALS = "$Terminals"; $env:SPIKE_OUT = "$res\ten.jsonl"
$p = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds 12
$m = (& "$PSScriptRoot\mem.ps1" -Root $p.Id | Select-Object -Last 1) | ConvertFrom-Json
& "$PSScriptRoot\shot.ps1" -ProcessId $p.Id -Out "$res\spike-10-terminaux.png" | Out-Null
Stop-Process -Id $p.Id -Force
$line = @{ kind = 'ten-terminals'; terminals = $Terminals; rest = $m } | ConvertTo-Json -Compress -Depth 4
Add-Content $out $line; $line
for ($i = 1; $i -le $Runs; $i++) {
  Start-Sleep -Seconds 2
  $env:SPIKE_BENCH = 'flood'; $env:SPIKE_FLOOD_LINES = "$Lines"; $env:SPIKE_OUT = "$res\ten-flood-$i.jsonl"
  Remove-Item $env:SPIKE_OUT -ErrorAction SilentlyContinue
  $p = Start-Process -FilePath $exe -PassThru
  $cpu0 = $null; $until = [DateTime]::Now.AddSeconds(180)
  while ([DateTime]::Now -lt $until) {
    Start-Sleep -Milliseconds 250; $p.Refresh()
    if ($null -eq $cpu0 -and [DateTime]::Now -gt $p.StartTime.AddSeconds(4)) { $cpu0 = $p.TotalProcessorTime.TotalMilliseconds }
    if ((Test-Path $env:SPIKE_OUT) -and ((Get-Content $env:SPIKE_OUT) -match '"flood"')) { break }
  }
  Start-Sleep -Milliseconds 500; $p.Refresh()
  $cpu = $p.TotalProcessorTime.TotalMilliseconds - $cpu0
  $m = (& "$PSScriptRoot\mem.ps1" -Root $p.Id | Select-Object -Last 1) | ConvertFrom-Json
  if ($i -eq 1) { & "$PSScriptRoot\shot.ps1" -ProcessId $p.Id -Out "$res\spike-10-terminaux-apres-flux.png" | Out-Null }
  $peak = $p.PeakWorkingSet64 / 1MB
  Stop-Process -Id $p.Id -Force
  $f = Get-Content $env:SPIKE_OUT | ConvertFrom-Json | Where-Object kind -eq 'flood'
  $line = @{ kind = 'ten-flood'; run = $i; terminals = $Terminals; lines_each = $Lines; result = $f; ui_cpu_ms = [Math]::Round($cpu); ui_peak_ws_mb = [Math]::Round($peak, 1); after = $m } | ConvertTo-Json -Compress -Depth 4
  Add-Content $out $line; $line
}
