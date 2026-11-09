# La mémoire d'une app et de ses descendants (ESC-365) : -Root <pid>.
# « interface » = le process racine et les msedgewebview2 de son arbre ; « arbre » = tout ce qui descend de lui
# (shells, agents, conhost compris). Ensemble de travail privé (Private Working Set, ce que le Gestionnaire des
# tâches affiche) et octets privés (commit), en Mo.
param([Parameter(Mandatory)][int]$Root)
$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
$kids = @{}
foreach ($p in $all) { if (-not $kids.ContainsKey([int]$p.ParentProcessId)) { $kids[[int]$p.ParentProcessId] = @() }; $kids[[int]$p.ParentProcessId] += $p }
$tree = @(); $stack = @($all | Where-Object { $_.ProcessId -eq $Root })
while ($stack.Count) { $p = $stack[0]; $stack = @($stack | Select-Object -Skip 1); $tree += $p; if ($kids.ContainsKey([int]$p.ProcessId)) { $stack += $kids[[int]$p.ProcessId] } }
$perf = @{}
Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | ForEach-Object { $perf[[int]$_.IDProcess] = $_ }
$mainWs = 0; $mainPriv = 0; $ui = 0; $uiPriv = 0; $tot = 0; $totPriv = 0; $n = 0; $rows = @()
foreach ($p in $tree) {
  $f = $perf[[int]$p.ProcessId]; if (-not $f) { continue }
  $ws = [double]$f.WorkingSetPrivate / 1MB; $pb = [double]$f.PrivateBytes / 1MB
  $isUi = ($p.ProcessId -eq $Root) -or ($p.Name -match 'msedgewebview2')
  if ($p.ProcessId -eq $Root) { $mainWs = $ws; $mainPriv = $pb }
  if ($isUi) { $ui += $ws; $uiPriv += $pb; $n++ }
  $tot += $ws; $totPriv += $pb
  $rows += '{0,-28} {1,7} {2,9:N1} {3,9:N1}' -f $p.Name, $p.ProcessId, $ws, $pb
}
$rows | ForEach-Object { Write-Host $_ }
[cultureinfo]::CurrentCulture = [cultureinfo]::InvariantCulture
'{{"root_private_ws_mb":{6:F1},"root_private_bytes_mb":{7:F1},"ui_private_ws_mb":{0:F1},"ui_private_bytes_mb":{1:F1},"ui_processes":{2},"tree_private_ws_mb":{3:F1},"tree_private_bytes_mb":{4:F1},"tree_processes":{5}}}' -f $ui, $uiPriv, $n, $tot, $totPriv, $tree.Count, $mainWs, $mainPriv
