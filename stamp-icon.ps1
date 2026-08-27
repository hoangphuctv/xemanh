# Embeds assets\xemanh.png as the Windows PE icon of an exe.
param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath,
    [string]$PngPath = ""
)

$ErrorActionPreference = "Stop"
if (-not $PngPath) {
    $PngPath = Join-Path $PSScriptRoot "assets\xemanh.png"
}

$exe = (Resolve-Path $ExePath).Path
$png = (Resolve-Path $PngPath).Path
$pngBytes = [System.IO.File]::ReadAllBytes($png)
if ($pngBytes.Length -lt 24) { throw "PNG is too small: $png" }

$width = [BitConverter]::ToUInt32([byte[]]($pngBytes[19], $pngBytes[18], $pngBytes[17], $pngBytes[16]), 0)
$height = [BitConverter]::ToUInt32([byte[]]($pngBytes[23], $pngBytes[22], $pngBytes[21], $pngBytes[20]), 0)
$bWidth = if ($width -ge 256) { [byte]0 } else { [byte]$width }
$bHeight = if ($height -ge 256) { [byte]0 } else { [byte]$height }

# GRPICONDIR + GRPICONDIRENTRY (14 bytes, uses nID instead of offset)
$group = New-Object byte[] 20
$group[2] = 1 # type = icon
$group[4] = 1 # count
$group[6] = $bWidth
$group[7] = $bHeight
$group[10] = 1 # planes
$group[12] = 32 # bit count
[BitConverter]::GetBytes([uint32]$pngBytes.Length).CopyTo($group, 14)
$group[18] = 1 # RT_ICON id
$group[19] = 0

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class NativeRes {
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern IntPtr BeginUpdateResource(string fileName, bool deleteExisting);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool UpdateResource(IntPtr hUpdate, IntPtr type, IntPtr name, ushort language, byte[] data, uint size);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool EndUpdateResource(IntPtr hUpdate, bool discard);
}
"@ -ErrorAction SilentlyContinue

$RT_ICON = [IntPtr]3
$RT_GROUP_ICON = [IntPtr]14
$NAME = [IntPtr]1
$LANG_NEUTRAL = [uint16]0

$h = [NativeRes]::BeginUpdateResource($exe, $false)
if ($h -eq [IntPtr]::Zero) {
    throw "BeginUpdateResource failed (Win32 $( [Runtime.InteropServices.Marshal]::GetLastWin32Error() ))"
}
if (-not [NativeRes]::UpdateResource($h, $RT_ICON, $NAME, $LANG_NEUTRAL, $pngBytes, [uint32]$pngBytes.Length)) {
    throw "UpdateResource ICON failed (Win32 $( [Runtime.InteropServices.Marshal]::GetLastWin32Error() ))"
}
if (-not [NativeRes]::UpdateResource($h, $RT_GROUP_ICON, $NAME, $LANG_NEUTRAL, $group, [uint32]$group.Length)) {
    throw "UpdateResource GROUP_ICON failed (Win32 $( [Runtime.InteropServices.Marshal]::GetLastWin32Error() ))"
}
if (-not [NativeRes]::EndUpdateResource($h, $false)) {
    throw "EndUpdateResource failed (Win32 $( [Runtime.InteropServices.Marshal]::GetLastWin32Error() ))"
}

Write-Host "[OK] Embedded icon into $exe"
