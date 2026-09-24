#!/usr/bin/env python3
"""Portable runtime checks from a clean temporary working directory."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    with tempfile.TemporaryDirectory(prefix="moenotes-relocate-") as temp:
        root = Path(temp)
        exe = root / binary.name
        shutil.copy2(binary, exe)
        shutil.copytree(ROOT / "tests/fixtures", root / "fixtures")
        env = os.environ.copy()
        env.pop("MOENOTES_ASSETS_DIR", None)

        def run(*arguments, ok=True):
            result = subprocess.run([str(exe), *arguments], cwd=root, env=env, capture_output=True, text=True)
            assert (result.returncode == 0) == ok, result.stderr
            return result

        for bars in ["1", "2"]:
            run("render", "fixtures/synthetic.json", "-o", "chart.png", "--metadata", "fixtures/metadata.json",
                "--bars-per-column", bars, "--supersample", "1")
            report = json.loads((root / "chart.render.json").read_text())
            assert len(report["images"]) == 1 and report["columns"] > 1
        pointer = json.loads((root / "chart.render-set/current.json").read_text())
        generation = root / "chart.render-set" / pointer["generation"]
        for entry in pointer["files"]:
            assert hashlib.sha256((generation / entry["name"]).read_bytes()).hexdigest() == entry["sha256"]
        before = (root / "chart.render-set/current.json").read_bytes()
        run("render", "fixtures/synthetic.json", "-o", "chart.png", "--cover", "missing.png", ok=False)
        assert (root / "chart.render-set/current.json").read_bytes() == before
        run("inspect", "fixtures/synthetic.json", "-o", "fixtures/synthetic.json", ok=False)
        print(json.dumps({"relocated_runtime": "passed", "no_game_assets": True,
                          "complete_multi_column_single_image": True,
                          "failed_render_keeps_previous_generation": True, "images": 1}))


if __name__ == "__main__":
    main()
