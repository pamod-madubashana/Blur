$ErrorActionPreference = "Stop"

$here = $PSScriptRoot
$vcvars = $null

foreach ($root in @(
    "C:\Program Files (x86)\Microsoft Visual Studio\2022",
    "C:\Program Files\Microsoft Visual Studio\2022"
)) {
    if (Test-Path -LiteralPath $root) {
        $candidate = Get-ChildItem -LiteralPath $root -Filter "vcvarsall.bat" -Recurse -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($candidate) {
            $vcvars = $candidate.FullName
            break
        }
    }
}

if (-not $vcvars) {
    throw "vcvarsall.bat not found - install Visual Studio 2022 with the x86 C++ toolchain"
}

$source = Join-Path $here "blur_lan_bridge.cpp"
$def = Join-Path $here "blur_lan_bridge.def"
$obj = Join-Path $here "blur_lan_bridge.obj"
$output = Join-Path $here "..\files\amax\dlls\blur_lan_bridge.asi"

$outputDir = Split-Path -Parent $output
if (-not (Test-Path -LiteralPath $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$command = "`"$vcvars`" x86 && cl /nologo /LD /MT /O2 /Fo:`"$obj`" /Fe:`"$output`" /DEF:`"$def`" `"$source`" /link ws2_32.lib"
cmd /c $command

if ($LASTEXITCODE -ne 0) {
    throw "blur_lan_bridge build failed"
}

Remove-Item -LiteralPath (Join-Path $outputDir "blur_lan_bridge.exp") -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $outputDir "blur_lan_bridge.lib") -ErrorAction SilentlyContinue

Write-Host "Built: $output"
