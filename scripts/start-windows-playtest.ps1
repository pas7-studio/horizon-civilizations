# Launch a disposable Horizon Civilizations test world using the official Horizon Launcher.
# This script intentionally requires a human to trust an unsigned native executable.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ClientExe,
    [string]$LauncherExe = 'horizon-launcher',
    [string]$WorldName = 'Civilizations-Test',
    [switch]$TrustUnsigned,
    [switch]$SkipPrepare
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not $TrustUnsigned) {
    throw 'Explicit approval required: pass -TrustUnsigned only after you inspected and trust your own GitHub Actions build.'
}
if ($WorldName -notmatch '^[A-Za-z0-9_-]{1,40}$') {
    throw 'WorldName must be 1..40 ASCII letters, digits, underscore or dash (no paths).'
}
if (-not (Test-Path -LiteralPath $ClientExe -PathType Leaf)) {
    throw "Horizon client EXE not found: $ClientExe"
}
$client = (Resolve-Path -LiteralPath $ClientExe).Path
if ((Split-Path -Path $client -Leaf) -ine 'horizon-client.exe') {
    throw 'Expected horizon-client.exe from the native build artifact.'
}

$launcher = Get-Command -Name $LauncherExe -ErrorAction Stop
if ($launcher.CommandType -ne 'Application') {
    throw 'LauncherExe must resolve to an installed executable, not a script or alias.'
}
$launcherBinary = $launcher.Source

function Invoke-HorizonLauncher {
    param([string[]]$LaunchArgs)
    & $launcherBinary @LaunchArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Horizon Launcher failed (exit code $LASTEXITCODE): $($LaunchArgs -join ' ')"
    }
}

Write-Host 'Importing selected unsigned Horizon game build. This does not change your existing Minecraft account.'
Invoke-HorizonLauncher -LaunchArgs @('import-build', $client, '--trust-unsigned')
if (-not $SkipPrepare) {
    Write-Host 'Preparing resources from your own Minecraft Java Edition installation / entitled account.'
    Invoke-HorizonLauncher -LaunchArgs @('prepare')
}
Write-Host "Launching disposable test world: $WorldName"
Invoke-HorizonLauncher -LaunchArgs @('play', '--world', $WorldName, '--creative')
