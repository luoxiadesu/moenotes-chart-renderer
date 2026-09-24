#!/usr/bin/env python3
"""Validate a caller-selected live metadata directory against its current result."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--masterdata", type=Path, required=True)
    parser.add_argument("--chart", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--key", required=True)
    parser.add_argument("--packs", type=Path)
    parser.add_argument("--output", type=Path, default=Path("output/metadata-check.png"))
    args = parser.parse_args()
    command = [str(ROOT / "target/release/moenotes-chart-renderer"), "render", str(args.chart),
               "-o", str(args.output), "--masterdata", str(args.masterdata), "--chart-key", args.key,
               "--assets", str(args.assets), "--language", "ja"]
    if args.packs:
        command += ["--packs", str(args.packs), "--skin", "skin001"]
    subprocess.run(command, check=True)
    report = json.loads(args.output.with_suffix(".render.json").read_text())
    metadata = report["metadata"]
    assert len(report["images"]) == 1
    assert metadata["title"] and metadata["author"] and metadata["cover"]
    print(json.dumps({"provenance": metadata["provenance"], "title": metadata["title"],
                      "author": metadata["author"], "cover_sha256": hashlib.sha256(Path(metadata["cover"]).read_bytes()).hexdigest(),
                      "master_full_combo": metadata["master_full_combo"], "reconstructed_full_combo": report["statistics"]["reconstructed_full_combo"],
                      "images": report["images"]}, ensure_ascii=False, indent=2))

if __name__ == "__main__":
    main()
