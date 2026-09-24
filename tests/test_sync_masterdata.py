"""Synthetic repository updates; no network or game data required."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "scripts/sync_masterdata.py"
spec = importlib.util.spec_from_file_location("sync_masterdata", SCRIPT)
sync = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sync)


class Updates(unittest.TestCase):
    def response(self, url):
        if url.endswith("current_version.json"):
            return json.dumps({"regions": {"en": {"data_path": "en", "version": "test", "resource_version": "test", "verified_at": "test"}}}).encode()
        return b'{"_allData": []}'

    def test_pointer_advances_and_corrupt_cache_does_not_replace(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(sync, "get", self.response):
            root = Path(temp)
            a = sync.sync("en", root, "a" * 40)
            self.assertEqual(json.loads((root / "en/current.json").read_text(encoding="utf-8"))["commit"], "a" * 40)
            b = sync.sync("en", root, "b" * 40)
            self.assertTrue(a.is_dir())
            self.assertEqual(json.loads((root / "en/current.json").read_text(encoding="utf-8"))["commit"], "b" * 40)
            (a / "MasterText.json").write_bytes(b"corrupt")
            with self.assertRaises(ValueError):
                sync.sync("en", root, "a" * 40)
            self.assertEqual(json.loads((root / "en/current.json").read_text(encoding="utf-8"))["commit"], "b" * 40)
            for name, digest in json.loads((b / "provenance.json").read_text(encoding="utf-8"))["files"].items():
                self.assertEqual(hashlib.sha256((b / name).read_bytes()).hexdigest(), digest)

    def test_default_sync_resolves_new_repository_head_each_time(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(sync, "get", self.response), patch.object(sync, "head_commit", side_effect=["c" * 40, "d" * 40]):
            root = Path(temp)
            sync.sync("en", root)
            sync.sync("en", root)
            self.assertEqual(json.loads((root / "en/current.json").read_text(encoding="utf-8"))["commit"], "d" * 40)

    def test_partial_download_preserves_previous_pointer(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(sync, "get", self.response):
            root = Path(temp)
            sync.sync("en", root, "a" * 40)
            def broken(url):
                if url.endswith("MasterText.json"):
                    raise OSError("synthetic network failure")
                return self.response(url)
            with patch.object(sync, "get", broken), self.assertRaises(OSError):
                sync.sync("en", root, "b" * 40)
            self.assertEqual(json.loads((root / "en/current.json").read_text(encoding="utf-8"))["commit"], "a" * 40)
            self.assertFalse((root / "en" / ("b" * 40)).exists())

if __name__ == "__main__":
    unittest.main()
