<#
Install the source-only native mod into an explicitly selected Horizon test instance.
Does not modify the Horizon executable or create a fork. Run from Windows PowerShell:
  .\scripts\install-local.ps1 -InstancePath 'C:\path\to\your\Horizon\instance'
Requires Git/Rust 1.95, Python 3.11+, and a disposable test instance.
#>
param(
    [Parameter(Mandatory=$true)]
    [string] $InstancePath
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not (Test-Path -Path $InstancePath -PathType Container)) {
    throw "InstancePath does not exist: $InstancePath"
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo is not installed. Install Rust 1.95 before proceeding.'
}
$Python = Get-Command py -ErrorAction SilentlyContinue
if ($Python) { $PythonExe = 'py'; $PythonArgs = @('-3') }
elseif (Get-Command python -ErrorAction SilentlyContinue) { $PythonExe = 'python'; $PythonArgs = @() }
else { throw 'Python 3.11+ is required.' }
Push-Location $Root
try {
    & cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed' }
    & cargo test
    if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
    & $PythonExe @PythonArgs tools/package.py
    if ($LASTEXITCODE -ne 0) { throw 'package.py failed' }
    & $PythonExe @PythonArgs tools/verify_static.py
    if ($LASTEXITCODE -ne 0) { throw 'verify_static.py failed' }
    $Package = Join-Path $Root 'target\civilizations-0.1.3.hnmod'
    $Destination = Join-Path $InstancePath 'horizon-native-mods'
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    Copy-Item -Path $Package -Destination $Destination -Force
    Write-Host "Installed source package to $Destination"
    Write-Host 'NEXT: Open the Horizon launcher and explicitly TRUST/BUILD native mods.'
    Write-Host 'Use a disposable world. Rust unit tests are not proof of Horizon game compatibility.'
} finally {
    Pop-Location
}
