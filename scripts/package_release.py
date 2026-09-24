#!/usr/bin/env python3
"""Prepare a local binary package; this script never publishes a release."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    name = f"moenotes-chart-renderer-{version}-{args.target}"
    with tempfile.TemporaryDirectory() as temp:
        folder = Path(temp) / name
        folder.mkdir()
        shutil.copy2(args.binary, folder / args.binary.name)
        for filename in ["README.md", "LICENSE", "THIRD_PARTY.md"]:
            shutil.copyfile(ROOT / filename, folder / filename)
        shutil.copytree(ROOT / "assets/licenses", folder / "licenses/components")
        shutil.copytree(ROOT / "assets/fonts", folder / "licenses/fonts",
                        ignore=shutil.ignore_patterns("*.ttf", "*.otf"))
        shutil.copyfile(ROOT / "vendor/moenotes-chart-parser/LICENSE", folder / "licenses/parser-MIT.txt")
        shutil.copyfile(ROOT / "vendor/moenotes-chart-parser/third_party/LICENSE.yyjson", folder / "licenses/yyjson-MIT.txt")
        shutil.copytree(ROOT / "tests/fixtures", folder / "examples")
        hashes = {str(path.relative_to(folder)): hashlib.sha256(path.read_bytes()).hexdigest()
                  for path in folder.rglob("*") if path.is_file()}
        (folder / "SHA256.json").write_text(json.dumps(hashes, indent=2) + "\n")
        if "windows" in args.target:
            destination = args.output / (name + ".zip")
            with zipfile.ZipFile(destination, "w", zipfile.ZIP_DEFLATED) as archive:
                for path in folder.rglob("*"):
                    if path.is_file():
                        archive.write(path, str(path.relative_to(folder.parent)))
        else:
            destination = args.output / (name + ".tar.gz")
            with tarfile.open(destination, "w:gz") as archive:
                archive.add(folder, arcname=name)
        print(destination)


if __name__ == "__main__":
    main()
