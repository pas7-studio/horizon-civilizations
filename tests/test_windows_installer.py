"""Windows installer checks; requires no Windows environment or game assets."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]

class InstallerContractTests(unittest.TestCase):
    def test_source_is_formatted_before_validation(self):
        content = (ROOT / 'scripts/install-local.ps1').read_text()
        self.assertIn('& cargo fmt --all', content)
        self.assertLess(content.index('& cargo fmt --all\n'), content.index('& cargo fmt --all --check'))
        self.assertLess(content.index('& cargo fmt --all --check'), content.index('& cargo test'))
        self.assertIn('InstancePath', content)
        self.assertIn('horizon-native-mods', content)

if __name__ == '__main__':
    unittest.main()
