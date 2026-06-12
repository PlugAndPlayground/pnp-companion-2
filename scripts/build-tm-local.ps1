param(
    [string]$TmDir,
    [string]$OutputRoot,
    [switch]$Debug
)

$ErrorActionPreference = 'Stop'
$companionRoot = Split-Path -Parent $PSScriptRoot

if (-not $TmDir) {
    $TmDir = Join-Path $companionRoot 'tm'
}
if (-not $OutputRoot) {
    $OutputRoot = Join-Path $companionRoot 'artifacts'
}

$TmDir = (Resolve-Path $TmDir).Path
if (-not (Test-Path (Join-Path $TmDir 'index.html'))) {
    throw "TM directory must contain index.html: $TmDir"
}

Push-Location $companionRoot
try {
    $cargoArguments = @('build')
    if (-not $Debug) {
        $cargoArguments += '--release'
    }
    & cargo @cargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Companion build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$profile = if ($Debug) { 'debug' } else { 'release' }
$extension = if ($IsWindows -or $env:OS -eq 'Windows_NT') { '.exe' } else { '' }
$binaryName = "tm-companion$extension"
$binary = Join-Path $companionRoot "target/$profile/$binaryName"
$companionOnly = Join-Path $OutputRoot 'companion-only'
$tmLocal = Join-Path $OutputRoot 'tm-local'

function Reset-OutputDirectory([string]$Path) {
    if (Test-Path $Path) {
        for ($attempt = 1; $attempt -le 5; $attempt++) {
            try {
                Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction Stop
                break
            } catch {
                if ($attempt -eq 5) {
                    throw
                }
                Start-Sleep -Milliseconds 500
            }
        }
    }
    New-Item -ItemType Directory -Path $Path | Out-Null
}

foreach ($path in @($companionOnly, $tmLocal)) {
    Reset-OutputDirectory $path
}

Copy-Item -LiteralPath $binary -Destination (Join-Path $companionOnly $binaryName)
Copy-Item -LiteralPath $binary -Destination (Join-Path $tmLocal $binaryName)
Copy-Item -LiteralPath $TmDir -Destination (Join-Path $tmLocal 'tm') -Recurse

Write-Output "Companion-only binary: $(Join-Path $companionOnly $binaryName)"
Write-Output "TM Local distribution: $tmLocal"
