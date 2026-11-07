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
        self.assertIn("if ($LASTEXITCODE -ne 0)", SCRIPT)
        self.assertLess(SCRIPT.index("@('import-build', $client, '--trust-unsigned')"),
                        SCRIPT.index("@('prepare')"))
        self.assertLess(SCRIPT.index("@('prepare')"),
                        SCRIPT.index("@('play', '--world', $WorldName, '--creative')"))

    def test_fails_closed_on_unexpected_launcher_type(self):
        self.assertIn("$launcher.CommandType -ne 'Application'", SCRIPT)
        self.assertNotIn("Invoke-Expression", SCRIPT)
        self.assertNotIn("irm ", SCRIPT)
