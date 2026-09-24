#!/usr/bin/env python3
"""Build and test a copy without the parent workspace or external resources."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cargo", default="cargo")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="moenotes-source-") as temp:
        root = Path(temp)
        for name in ["src", "vendor", "assets", "tests", "examples"]:
            shutil.copytree(ROOT / name, root / name, ignore=shutil.ignore_patterns("__pycache__"))
        for name in ["Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml", "LICENSE", "README.md", "THIRD_PARTY.md"]:
            shutil.copy2(ROOT / name, root / name)
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(ROOT / "target/standalone-source")
        for command in [["test", "--release", "--locked"], ["build", "--release", "--locked"],
                        ["run", "--release", "--locked", "--example", "memory"]]:
            subprocess.run([args.cargo, *command], cwd=root, env=env, check=True)
        print(json.dumps({"source_copy": "passed", "outside_workspace": True,
                          "game_assets": False, "sibling_parser": False, "normal_tests": "passed"}))


if __name__ == "__main__":
    main()
