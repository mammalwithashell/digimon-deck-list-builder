# Capture a window's CLIENT area (no native title bar) to PNG via PrintWindow.
# PrintWindow works even when the window is not foreground/occluded.
param(
  [string]$ProcName = "digimon-tcg",
  [Parameter(Mandatory=$true)][string]$OutPath
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinCap {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@
[void][WinCap]::SetProcessDPIAware()

$p = Get-Process -Name $ProcName -ErrorAction SilentlyContinue |
     Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Write-Error "no '$ProcName' window"; exit 2 }
$h = $p.MainWindowHandle

$wr = New-Object WinCap+RECT; [void][WinCap]::GetWindowRect($h, [ref]$wr)
$cr = New-Object WinCap+RECT; [void][WinCap]::GetClientRect($h, [ref]$cr)
$origin = New-Object WinCap+POINT; $origin.X = 0; $origin.Y = 0
[void][WinCap]::ClientToScreen($h, [ref]$origin)
$ww = $wr.Right - $wr.Left; $wh = $wr.Bottom - $wr.Top
$cw = $cr.Right - $cr.Left; $ch = $cr.Bottom - $cr.Top
$offX = $origin.X - $wr.Left; $offY = $origin.Y - $wr.Top

# Full-window PrintWindow into a bitmap, then crop to the client rect.
$full = New-Object System.Drawing.Bitmap $ww, $wh
$g = [System.Drawing.Graphics]::FromImage($full); $hdc = $g.GetHdc()
[void][WinCap]::PrintWindow($h, $hdc, 0x2)   # PW_RENDERFULLCONTENT (WebView2)
$g.ReleaseHdc($hdc); $g.Dispose()

$client = New-Object System.Drawing.Bitmap $cw, $ch
$g2 = [System.Drawing.Graphics]::FromImage($client)
$g2.DrawImage($full, (New-Object System.Drawing.Rectangle 0,0,$cw,$ch),
              $offX, $offY, $cw, $ch, [System.Drawing.GraphicsUnit]::Pixel)
$g2.Dispose(); $full.Dispose()
$client.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png); $client.Dispose()
Write-Output "saved $OutPath (${cw}x${ch})"
