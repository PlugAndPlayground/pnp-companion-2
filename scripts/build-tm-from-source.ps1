param(
    [string]$Repository = 'git@github.com:tailrmade/tm.git',
    [string]$Ref = 'main',
    [string]$OutputRoot,
    [switch]$Debug
)

$ErrorActionPreference = 'Stop'
$companionRoot = Split-Path -Parent $PSScriptRoot
$checkoutDir = Join-Path ([System.IO.Path]::GetTempPath()) "tm-build-$([guid]::NewGuid())"

try {
    $cloneArguments = @('clone', '--depth', '1', '--branch', $Ref)
    $cloneArguments += @($Repository, $checkoutDir)

    & git @cloneArguments
    if ($LASTEXITCODE -ne 0) {
        throw "TM clone failed with exit code $LASTEXITCODE"
    }

    Push-Location $checkoutDir
    try {
        & yarn install --immutable
        if ($LASTEXITCODE -ne 0) {
            throw "TM dependency installation failed with exit code $LASTEXITCODE"
        }

        & yarn build:self-hosted
        if ($LASTEXITCODE -ne 0) {
            throw "TM build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $packageArguments = @{
        TmDir = Join-Path $checkoutDir 'dist'
    }
    if ($OutputRoot) {
        $packageArguments.OutputRoot = $OutputRoot
    }
    if ($Debug) {
        $packageArguments.Debug = $true
    }

    & (Join-Path $PSScriptRoot 'build-tm-local.ps1') @packageArguments
} finally {
    if (Test-Path $checkoutDir) {
        Remove-Item -LiteralPath $checkoutDir -Recurse -Force
    }
}
