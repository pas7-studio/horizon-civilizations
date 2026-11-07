"""PowerShell launcher safety and command ordering without requiring Windows."""
from pathlib import Path
import unittest

SCRIPT = (Path(__file__).resolve().parents[1] / "scripts/start-windows-playtest.ps1").read_text()

class TestWindowsPlaytestLauncher(unittest.TestCase):
    def test_requires_unsigned_native_binary_approval(self):
        self.assertIn("if (-not $TrustUnsigned)", SCRIPT)
        self.assertIn("pass -TrustUnsigned", SCRIPT)
        self.assertIn("horizon-client.exe", SCRIPT)
        self.assertIn("WorldName -notmatch", SCRIPT)
        self.assertIn("Test-Path -LiteralPath $ClientExe -PathType Leaf", SCRIPT)

    def test_uses_official_launcher_with_checked_failures(self):
        self.assertIn("Get-Command -Name $LauncherExe -ErrorAction Stop", SCRIPT)
        self.assertIn("if ($exitCode -ne 0)", SCRIPT)
        self.assertLess(SCRIPT.index("@('import-build', $client, '--trust-unsigned')"),
                        SCRIPT.index("@('prepare')"))
        self.assertLess(SCRIPT.index("@('prepare')"),
                        SCRIPT.index("@('play', '--world', $WorldName, '--creative')"))

    def test_windows_gui_launcher_is_awaited_before_next_stage(self):
        # Horizon Launcher is a Windows GUI executable; an unpiped call
        # can return to PowerShell before import-build/prepare has exited.
        self.assertIn("& $launcherBinary @LaunchArgs | Out-Host", SCRIPT)
        self.assertIn("$global:LASTEXITCODE = $null", SCRIPT)
        self.assertIn("if ($null -eq $exitCode)", SCRIPT)
        self.assertIn("if ($exitCode -ne 0)", SCRIPT)
        self.assertNotIn("    & $launcherBinary @LaunchArgs\n", SCRIPT)

    def test_fails_closed_on_unexpected_launcher_type(self):
        self.assertIn("$launcher.CommandType -ne 'Application'", SCRIPT)
        self.assertNotIn("Invoke-Expression", SCRIPT)
        self.assertNotIn("irm ", SCRIPT)
