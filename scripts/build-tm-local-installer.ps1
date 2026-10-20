# Builds "Tailrmade Local" installers: the companion with a TM build bundled.
# Usage: .\scripts\build-tm-local-installer.ps1 [-TmDir path\to\tm\dist]
param(
    [string]$TmDir
)

$ErrorActionPreference = 'Stop'
$companionRoot = Split-Path -Parent $PSScriptRoot
$staged = Join-Path $companionRoot 'tm'

if (-not $TmDir) {
    $TmDir = $staged
}
$TmDir = (Resolve-Path $TmDir).Path
if (-not (Test-Path (Join-Path $TmDir 'index.html'))) {
    throw "TM directory must contain index.html: $TmDir"
}

$stagedResolved = if (Test-Path $staged) { (Resolve-Path -LiteralPath $staged).Path } else { $null }
if ($TmDir -ne $stagedResolved) {
    if (Test-Path $staged) { Remove-Item -LiteralPath $staged -Recurse -Force }
    Copy-Item -LiteralPath $TmDir -Destination $staged -Recurse
}

Push-Location $companionRoot
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw "npm ci failed" }
    npx tauri build --config tauri.tmlocal.conf.json
    if ($LASTEXITCODE -ne 0) { throw "tauri build failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}

Write-Output "Installers: $(Join-Path $companionRoot 'target/release/bundle')"
