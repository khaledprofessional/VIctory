# Installs the D3D12 Agility SDK runtime the executable opts into.
#
# victory.exe exports D3D12SDKVersion/D3D12SDKPath (see
# apps/victory/src/agility.rs), which makes the D3D12 loader look for
# D3D12Core.dll in a `D3D12` folder beside the executable. Without that file
# the loader refuses to create a device:
#
#   The D3D12 SDK version configuration of the host exe is invalid. (0x887E0003)
#
# and the title then dies in D3D12SerializeVersionedRootSignature. This script
# downloads the Microsoft.Direct3D.D3D12 package whose version matches
# `D3D12SDKVersion` (620 = 1.620.x) and copies the x64 runtime next to the exe.
#
#   powershell -ExecutionPolicy Bypass -File tools/install-agility.ps1
#   powershell -ExecutionPolicy Bypass -File tools/install-agility.ps1 -ExeDir target\diag
#
param(
    [string]$ExeDir = 'target\release',
    [string]$Version = '1.620.2'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$dir = if ([System.IO.Path]::IsPathRooted($ExeDir)) { $ExeDir } else { Join-Path $root $ExeDir }
if (-not (Test-Path (Join-Path $dir 'victory.exe'))) {
    throw "no victory.exe in $dir (build it first: cargo build --release -p victory)"
}
$url = "https://api.nuget.org/v3-flatcontainer/microsoft.direct3d.d3d12/$Version/microsoft.direct3d.d3d12.$Version.nupkg"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "d3d12-$Version-$PID"
New-Item -ItemType Directory -Force $tmp | Out-Null
$nupkg = Join-Path $tmp 'd3d12.nupkg'
$zip = Join-Path $tmp 'd3d12.zip'
Write-Host "downloading $url"
Invoke-WebRequest -Uri $url -OutFile $nupkg -UseBasicParsing
Copy-Item $nupkg $zip
Expand-Archive -Path $zip -DestinationPath (Join-Path $tmp 'pkg') -Force
$out = Join-Path $dir 'D3D12'
New-Item -ItemType Directory -Force $out | Out-Null
foreach ($dll in 'D3D12Core.dll', 'd3d12SDKLayers.dll') {
    $from = Join-Path $tmp "pkg\build\native\bin\x64\$dll"
    if (Test-Path $from) {
        Copy-Item $from (Join-Path $out $dll) -Force
        Write-Host "installed $dll -> $out"
    }
}
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
Write-Host "done: $out"
