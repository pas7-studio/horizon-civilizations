"""Offline contract for host-authoritative settlement time."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
from civil_sources import core_source, glue_source
CORE = core_source()
GLUE = glue_source()

class HostClockContracts(unittest.TestCase):
    def test_server_uses_horizon_clock_not_mod_uptime(self):
        self.assertIn("game.world.day_time.div_euclid(24_000)", GLUE)
        self.assertIn(".observe_host_day(host_day)", GLUE)
        self.assertNotIn("server_ticks % TICKS_PER_DAY", GLUE)

    def test_legacy_anchor_and_bounded_delta(self):
        self.assertIn("pub last_host_day: Option<i64>", CORE)
        self.assertIn("pub fn observe_host_day(", CORE)
        self.assertIn("MAX_HOST_CATCHUP_DAYS", CORE)
        self.assertIn("i128::from(host_day) - i128::from(previous)", CORE)

    def test_artifact_version_matches_manifest(self):
        import tomllib
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        manifest = tomllib.loads((ROOT / "mixins/civilizations/horizon-mixin.toml").read_text())
        ver = cargo["package"]["version"]
        self.assertEqual(ver, manifest["mod"]["version"])
        self.assertIn(f"civilizations-{ver}.hnmod",
                      (ROOT / "scripts/install-local.ps1").read_text())
