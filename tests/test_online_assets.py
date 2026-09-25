import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch
from email.message import Message

spec = importlib.util.spec_from_file_location("online_assets", Path(__file__).resolve().parents[1] / "scripts/online_assets.py")
assets = importlib.util.module_from_spec(spec)
spec.loader.exec_module(assets)


class Response(io.BytesIO):
    def __init__(self, data, content_type="application/octet-stream"):
        super().__init__(data)
        self.headers = Message()
        self.headers["Content-Type"] = content_type


class OnlineAssets(unittest.TestCase):
    def test_chart_key_and_url_are_bounded(self):
        self.assertEqual(assets.chart_object("0069/0069_03"), "Live/MusicScore/0069/0069_03/0069_03.json")
        for key in ["../x", "a/b/c", "a/%2e", "a/..", "a/漢字"]:
            with self.assertRaises(ValueError):
                assets.chart_object(key)
        for key in ["Live/MusicScore/../a.json", "Live/MusicScore/%2e/a.json", "other/file.json", "Image/Jacket/a.html"]:
            with self.assertRaises(ValueError):
                assets.object_url(assets.DEFAULT_BASE, key)

    def test_fetch_retains_exact_bytes_and_provenance(self):
        data = b'{"events":{},"notes":[]}'
        with patch.object(assets.urllib.request, "urlopen", return_value=Response(data)):
            result, evidence = assets.fetch_object(assets.chart_object("test/test_03"))
        self.assertEqual(result, data)
        self.assertEqual(evidence["bytes"], len(data))
        self.assertEqual(len(evidence["sha256"]), 64)

    def test_directory_html_and_invalid_images_are_errors(self):
        with patch.object(assets.urllib.request, "urlopen", return_value=Response(b"<html>", "text/html")):
            with self.assertRaisesRegex(ValueError, "directory page"):
                assets.fetch_object(assets.chart_object("test/test_03"))
        with patch.object(assets.urllib.request, "urlopen", return_value=Response(b"invalid")):
            with self.assertRaisesRegex(ValueError, "not a PNG"):
                assets.fetch_object("Image/Jacket/test/test__00000.png")


if __name__ == "__main__":
    unittest.main()
