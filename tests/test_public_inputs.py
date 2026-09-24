"""Public fixture and vendored-source contract checks, with no game resources."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class SourceContracts(unittest.TestCase):
    def test_vendor_hashes_match_recorded_upstream(self):
        root = ROOT / "vendor/moenotes-chart-parser"
        manifest = json.loads((root / "UPSTREAM.json").read_text(encoding="utf-8"))
        for name, digest in manifest["sha256"].items():
            self.assertEqual(hashlib.sha256((root / name).read_bytes()).hexdigest(), digest, name)

    def test_embedded_fonts_match_recorded_hashes(self):
        root = ROOT / "assets/fonts"
        for name, digest in json.loads((root / "font-provenance.json").read_text(encoding="utf-8")).items():
            self.assertEqual(hashlib.sha256((root / name).read_bytes()).hexdigest(), digest, name)

    def test_metadata_fixture_is_explicitly_synthetic(self):
        root = ROOT / "tests/fixtures/masterdata"
        manifest = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
        self.assertTrue(manifest["fixture"])
        for name, digest in manifest["files"].items():
            self.assertEqual(hashlib.sha256((root / name).read_bytes()).hexdigest(), digest, name)

if __name__ == "__main__":
    unittest.main()
