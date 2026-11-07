<#
Local validation for Horizon Civilizations stacked source milestones.
No GitHub Actions, CI triggers, server deployment or gameplay execution.

Examples:
  .\scripts\validate-local.ps1
  .\scripts\validate-local.ps1 -Format
  .\scripts\validate-local.ps1 -NativeCheck
  .\scripts\validate-local.ps1 -BuildClient

-Format explicitly changes local source formatting. Default: read-only source
checks plus generated target/ artifacts. -NativeCheck needs network access to
download PINNED Horizon sources; -BuildClient additionally compiles a local
game executable without bundled Minecraft assets.
#>
param(
    [switch] $Format,
    [switch] $NativeCheck,
    [switch] $BuildClient
)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Require-Tool([string] $Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is missing from PATH."
    }
}

function Run-Checked([string] $Name, [string[]] $Arguments) {
    Write-Host ">>> $Name $($Arguments -join ' ')"
    & $Name @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

foreach ($tool in @('rustc', 'cargo', 'rustfmt', 'git')) {
    Require-Tool $tool
}
$rustVersion = (& rustc --version) -join ''
if ($LASTEXITCODE -ne 0 -or $rustVersion -notmatch '^rustc 1\.95\.') {
    throw "Expected Rust 1.95.x. Found: $rustVersion"
}

if (Get-Command py -ErrorAction SilentlyContinue) {
    $Python = 'py'
    $PyBase = @('-3')
} elseif (Get-Command python -ErrorAction SilentlyContinue) {
    $Python = 'python'
    $PyBase = @()
} else {
    throw 'Python 3.11+ is required.'
}
$pyVersion = (& $Python @PyBase -c 'import sys; print(".".join(map(str, sys.version_info[:2])))') -join ''
if ($LASTEXITCODE -ne 0) {
    throw 'Failed to inspect Python version.'
}
$parts = $pyVersion.Split('.')
if ([int]$parts[0] -lt 3 -or ([int]$parts[0] -eq 3 -and [int]$parts[1] -lt 11)) {
    throw "Python 3.11+ required; found $pyVersion"
}

Push-Location $Root
try {
    if ($Format) {
        Write-Warning 'Formatting changes source files in this local checkout.'
        Run-Checked 'cargo' @('fmt', '--all')
        foreach ($source in (Get-ChildItem 'mixins/civilizations' -Filter '*.rs' | Sort-Object FullName)) {
            Run-Checked 'rustfmt' @('--edition', '2024', $source.FullName)
        }
    }

    Run-Checked 'cargo' @('fmt', '--all', '--check')
    foreach ($source in (Get-ChildItem 'mixins/civilizations' -Filter '*.rs' | Sort-Object FullName)) {
        Run-Checked 'rustfmt' @('--edition', '2024', '--check', $source.FullName)
    }
    Run-Checked 'cargo' @('clippy', '--all-targets', '--', '-D', 'warnings')
    Run-Checked 'cargo' @('test')
    Run-Checked $Python ($PyBase + @('-m', 'unittest', 'discover', '-s', 'tests', '-p', 'test_*.py', '-v'))
    Run-Checked $Python ($PyBase + @('tools/package.py'))
    Run-Checked $Python ($PyBase + @('tools/verify_static.py'))
    if ($NativeCheck -or $BuildClient) {
        $nativeArgs = $PyBase + @('tools/native_build.py', '--test')
        if ($BuildClient) { $nativeArgs += '--build-client' }
        Run-Checked $Python $nativeArgs
    }
    Write-Host 'PASS: local source checks completed. Real Minecraft gameplay E2E remains REQUIRED.'
} finally {
    Pop-Location
}
