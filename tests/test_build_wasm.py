"""Cargo artifact selection must never silently package a stale default build."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("build_wasm", Path(__file__).resolve().parents[1] / "scripts/build_wasm.py")
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


class BuildArtifacts(unittest.TestCase):
    def test_cargo_report_overrides_stale_default_directory(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            current = root / "custom target/wasm32-unknown-emscripten/release"
            stale = root / "target/wasm32-unknown-emscripten/release"
            for folder, marker in [(current, b"current"), (stale, b"stale")]:
                folder.mkdir(parents=True)
                (folder / "moenotes-wasm.js").write_bytes(marker)
                (folder / "moenotes_wasm.wasm").write_bytes(marker)
            message = {"reason": "compiler-artifact", "target": {"name": "moenotes-wasm", "kind": ["bin"]},
                       "executable": str(current / "moenotes-wasm.js")}
            # Both environment and .cargo/config overrides produce Cargo's paths.
            for env in [{"CARGO_TARGET_DIR": str(root / "custom target")}, {}]:
                with self.subTest(env=env), patch.object(build, "ROOT", root), patch.object(build.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, json.dumps(message))) as run:
                    glue, wasm = build.build_artifacts("cargo", env)
                    self.assertEqual(glue.read_bytes(), b"current")
                    self.assertEqual(wasm.read_bytes(), b"current")
                    self.assertEqual(run.call_args.kwargs["env"], env)

    def test_missing_artifact_does_not_fall_back(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            old = root / "target/wasm32-unknown-emscripten/release"
            old.mkdir(parents=True)
            (old / "moenotes-wasm.js").write_bytes(b"stale")
            (old / "moenotes_wasm.wasm").write_bytes(b"stale")
            message = {"reason": "compiler-artifact", "target": {"name": "moenotes-wasm", "kind": ["bin"]},
                       "executable": str(root / "missing/moenotes-wasm.js")}
            with patch.object(build, "ROOT", root), patch.object(build.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, json.dumps(message))):
                with self.assertRaises(FileNotFoundError):
                    build.build_artifacts("cargo", {})
            with patch.object(build.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, '{"reason":"build-finished","success":true}\n')):
                with self.assertRaisesRegex(ValueError, "exactly one"):
                    build.build_artifacts("cargo", {})


if __name__ == "__main__":
    unittest.main()
