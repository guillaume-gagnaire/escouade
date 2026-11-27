// A real wheel of Windows over a native webview of the app (ESC-679, ESC-694), shared by the e2e of the driving.
import { execFileSync } from 'node:child_process';
import path from 'node:path';

const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(process.cwd(), 'src-tauri', 'target', 'debug', 'escouade.exe');

const ps = (text: string) => `'${text.replace(/'/g, "''")}'`;

/**
 * A real wheel of Windows at the point (`x`, `y`) of the app's window, in CSS pixels of its client area: the user's own
 * gesture, `isTrusted`, over the native webview there (never the user's own Escouade: another program's path). The
 * window of the tests is off every screen, where no cursor goes: the wheel is the message Windows sends a window under
 * the cursor (`WM_MOUSEWHEEL`), posted to Chromium's render window of the webview that holds the point. The point is
 * the middle of what the window shows of the webview, its box cut by the window's client area: a page made wider than
 * its window (`setViewportSize` of 1440 × 900 in the CI runner's window of 1028 × 779) puts most of the webview past
 * the window's edge, where Chromium drops a wheel as no user could give it there (ESC-694). Says where it aimed.
 */
export function nativeWheel(box: { x: number; y: number; width: number; height: number }, dpr: number): string {
  const script = `
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Collections.Generic;
public static class Wheel {
  public delegate bool Each(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(Each e, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr p, Each e, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public static IntPtr Largest(uint[] pids) {
    IntPtr best = IntPtr.Zero; long area = 0;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (Array.IndexOf(pids, pid) < 0) return true;
      RECT r; GetWindowRect(h, out r);
      long a = (long)(r.Right - r.Left) * (r.Bottom - r.Top);
      if (a > area) { area = a; best = h; }
      return true;
    }, IntPtr.Zero);
    return best;
  }
  // The visible render window of Chromium whose box is the webview's (within 3 px): never the window's own.
  public static string Seen = "";
  public static IntPtr Boxed(IntPtr top, int x, int y, int w, int hh) {
    IntPtr best = IntPtr.Zero;
    EnumChildWindows(top, (h, l) => {
      var s = new System.Text.StringBuilder(256);
      GetClassName(h, s, 256);
      if (s.ToString() != "Chrome_RenderWidgetHostHWND" || !IsWindowVisible(h)) return true;
      RECT r; GetWindowRect(h, out r);
      Seen += r.Left + "," + r.Top + "," + (r.Right - r.Left) + "x" + (r.Bottom - r.Top) + " ";
      if (Math.Abs(r.Left - x) <= 3 && Math.Abs(r.Top - y) <= 3 && Math.Abs(r.Right - r.Left - w) <= 3 && Math.Abs(r.Bottom - r.Top - hh) <= 3) best = h;
      return true;
    }, IntPtr.Zero);
    return best;
  }
}
"@
[void][Wheel]::SetProcessDPIAware()
$exe = ${ps(path.resolve(EXE))}
$pids = @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -ieq $exe } | ForEach-Object { [uint32]$_.ProcessId })
$top = [Wheel]::Largest($pids)
if ($top -eq [IntPtr]::Zero) { throw 'no window' }
$o = New-Object Wheel+POINT
$o.X = ${Math.round(box.x * dpr)}; $o.Y = ${Math.round(box.y * dpr)}
[void][Wheel]::ClientToScreen($top, [ref]$o)
$h = [Wheel]::Boxed($top, $o.X, $o.Y, ${Math.round(box.width * dpr)}, ${Math.round(box.height * dpr)})
if ($h -eq [IntPtr]::Zero) { throw ('no webview at ' + $o.X + ',' + $o.Y + ' among ' + [Wheel]::Seen) }
$c = New-Object Wheel+RECT; [void][Wheel]::GetClientRect($top, [ref]$c)
$co = New-Object Wheel+POINT; [void][Wheel]::ClientToScreen($top, [ref]$co)
$left = [Math]::Max($o.X, $co.X); $right = [Math]::Min($o.X + ${Math.round(box.width * dpr)}, $co.X + $c.Right)
$up = [Math]::Max($o.Y, $co.Y); $down = [Math]::Min($o.Y + ${Math.round(box.height * dpr)}, $co.Y + $c.Bottom)
$shown = 'webview ' + $o.X + ',' + $o.Y + ' ${Math.round(box.width * dpr)}x${Math.round(box.height * dpr)} in a window ' + $co.X + ',' + $co.Y + ' ' + $c.Right + 'x' + $c.Bottom
if ($right - $left -lt 8 -or $down - $up -lt 8) { throw ('the window shows none of the ' + $shown) }
$p = New-Object Wheel+POINT
$p.X = [int](($left + $right) / 2); $p.Y = [int](($up + $down) / 2)
$l = [IntPtr](([int64]($p.Y -band 0xFFFF) -shl 16) -bor ([int64]($p.X -band 0xFFFF)))
$w = [IntPtr]([int64](0xFF880000))
foreach ($i in 1..3) { [void][Wheel]::PostMessage($h, 0x020A, $w, $l); Start-Sleep -Milliseconds 120 }
Write-Output ($shown + ', at ' + $p.X + ',' + $p.Y)
`;
  return execFileSync('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { encoding: 'utf8' }).trim();
}
