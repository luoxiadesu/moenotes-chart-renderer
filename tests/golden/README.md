# Reviewed complete-sheet baselines

## Print theme

`synthetic-print.png` is the default print theme (1032 x 670); the complete
single-column `synthetic-print-narrow.png` (360 x 1145) exercises minimum lane width,
cover, CJK title, FC, wrapped legend and stacked timestamps. Both were first
reviewed on 2026-09-25 from the responsive-header changes based on `a806ace`.
They were re-rendered and visually reviewed on Linux x86_64 / FreeType with
Rust 1.98.1 and Skia 0.153.3 on 2026-09-26 for the note-styling changes based
on `f429a46` (opaque note fills, vector Flick chevrons, gold critical diamond,
solid fever field), then again the same day for the sheet-layout changes
(top-aligned columns with a shared heading row and end times, pixel-snapped
grid rules, difficulty-colored badges only for standard names, one-line
footer). The synthetic sheets are narrower than 1500 px, so chart text scale
and header facts stay at their base form; dimensions are unchanged. Wide-sheet
text scaling and header facts were reviewed on real sheets alongside a
340-chart white/black geometry regression.

| File | SHA-256 |
| --- | --- |
| synthetic-print.png | 03b5e78bfc045fb7a9d40320dddd8453d45a5da5c55aab42c7bb5539894399b7 |
| synthetic-print-narrow.png | 5d86e7d792ffa42dd1e1bc4eb8dbdb0f19662fb0b6b21bf7a10fb290007cc740 |

Use `--theme print` and optionally `--narrow` with `check_golden.py` to check them.
These remain complete images; the narrow case is not a crop or page. Linux CI
checks both. Windows/macOS run print rendering/API checks but do not yet have
reviewed print pixel baselines; their native dark baselines below remain active.
Do not copy Linux print pixels into a purported native baseline.

## Deep black preset

`synthetic-black.png` is the neutral deep-black preset, first reviewed on
2026-09-25 from local changes based on `a806ace` and re-reviewed on Linux with
Rust 1.98.1 / Skia 0.153.3 on 2026-09-26 for the note-styling changes based on
`f429a46`, then for the sheet-layout changes. SHA-256:
`3b7265b43ce74bcd7d286fb256e655293a445f9e0227eb23e00c4931e01f3e71`.
It is the same complete 1032 x 670 synthetic sheet. Check it with `--theme black`.
Native Windows/macOS black pixel baselines have not yet been reviewed.

## Legacy dark theme

All images are the same original synthetic chart, cover and metadata from
`tests/fixtures`, rendered as one complete three-column PNG (1032 x 547).
They contain no game assets. Artwork remains under development.

Run `python scripts/check_golden.py --binary <binary>` to compare against the
current platform's dark baseline. `--update` explicitly replaces that baseline for
manual review. CI never updates baselines. All platforms keep the same mean
absolute channel error limit of 1.0 on the 0-255 scale; dimensions must match.

| File | Native environment | SHA-256 |
| --- | --- | --- |
| synthetic.png | Linux x86_64 / FreeType | ee5b45be1e8f1f74f10b031afdfabd041f5ab24e1e7f9667eee516659fb0056b |
| synthetic-macos.png | macos-14 ARM64 / CoreText | 424aab535555f9da60d9a9d0cde1e4a2efe175fc45bd7f7f865bf08d0d36f112 |
| synthetic-windows.png | windows-2022 x64 / DirectWrite | ca6b95585d19be8bc40563e11c4c979351ec7a26667e1a827275dafa53d9662e |

The Linux baseline was introduced in `c976d86` and matched pixel-for-pixel in
native CI at `840d4dc8f9e3f3e90ae9a7bab4eebf2b18cd2cbd`. The macOS and Windows
baselines came from that same source commit's
[native CI run](https://github.com/luoxiadesu/moenotes-chart-renderer/actions/runs/36057650861),
using Rust 1.98.1, Skia 0.153.3 and Pillow 10.2.0. They were reviewed as whole
images together with their full difference images against Linux.

Before platform baselines, the full-sheet differences from Linux were 1.241087
(macOS) and 1.014091 (Windows). Differences were concentrated in text edges
and advances despite identical embedded font files. The three note-track
interiors had maximum channel errors of 2/255 on macOS and 0 on Windows; the
cover was identical on both. Platform-specific baselines preserve checks of
the entire image, including text, without relaxing the threshold or masking
regions. They do not claim pixel-identical output across operating systems.
