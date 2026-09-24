#!/usr/bin/env python3
"""Compare a synthetic, redistributable sheet to a reviewed visual baseline."""
import argparse
import json
from pathlib import Path
import platform
import subprocess
import tempfile

from PIL import Image, ImageChops, ImageStat

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--update", action="store_true")
    parser.add_argument("--diagnostics", type=Path, default=ROOT / "output/golden-check")
    args = parser.parse_args()
    golden = ROOT / "tests/golden/synthetic.png"
    with tempfile.TemporaryDirectory() as temp:
        output = Path(temp) / "synthetic.png"
        result = subprocess.run([
            str(args.binary.resolve()), "render", str(ROOT / "tests/fixtures/synthetic.json"),
            "-o", str(output), "--metadata", str(ROOT / "tests/fixtures/metadata.json"),
            "--supersample", "1", "--pixels-per-beat", "24", "--fixed-spacing",
            "--bars-per-column", "2",
        ], capture_output=True, text=True, encoding="utf-8")
        assert result.returncode == 0, result.stderr
        with Image.open(output) as source:
            image = source.convert("RGB")
        if args.update:
            golden.parent.mkdir(exist_ok=True)
            image.save(golden)
            print("Golden updated explicitly")
            return
        with Image.open(golden) as source:
            reference = source.convert("RGB")
        args.diagnostics.mkdir(parents=True, exist_ok=True)
        image.save(args.diagnostics / "actual.png")
        reference.save(args.diagnostics / "reference.png")
        assert image.size == reference.size, (image.size, reference.size)
        diff = ImageChops.difference(image, reference)
        diff.save(args.diagnostics / "difference.png")
        mean = sum(ImageStat.Stat(diff).mean) / 3
        metrics = {"mean_absolute_channel_error": mean, "dimensions": image.size,
                   "threshold": 1.0, "platform": platform.system(),
                   "difference_bounds": diff.getbbox()}
        (args.diagnostics / "metrics.json").write_text(
            json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(metrics), flush=True)
        # Font rasterizers may vary glyph edge pixels. Keep the reviewed limit
        # and retain full-sheet diagnostics to investigate platform differences.
        assert mean <= 1.0, f"Visual baseline error {mean:.4f} > 1.0"


if __name__ == "__main__":
    main()
