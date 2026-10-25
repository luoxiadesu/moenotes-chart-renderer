# Reviewed complete-sheet baselines

## Print theme

`synthetic-print.png` is the default print theme (1152 x 670); the complete
single-column `synthetic-print-narrow.png` (360 x 1145) exercises minimum lane width,
cover, CJK title, FC, wrapped legend and stacked timestamps. Both were first
reviewed on 2026-09-25 from the responsive-header changes based on `a806ace`.
They were re-rendered and visually reviewed on Linux x86_64 / FreeType with
Rust 1.98.1 and Skia 0.153.3 on 2026-09-26 for the note-styling changes based
on `f429a46` (opaque note fills, vector Flick chevrons, gold critical diamond,
solid fever field), then again the same day for the sheet-layout changes
(top-aligned columns with a shared heading row and end times, pixel-snapped
grid rules, difficulty-colored badges only for standard names, one-line
footer). The note-value rail (68 px left gutter with note values beside the bar
numbers) was added on 2026-10-24; every baseline was regenerated from the
`build-and-test` workflow on each native runner and reviewed there, with the
Linux sheets re-checked locally.
The synthetic sheets are narrower than 1500 px, so chart text scale
and header facts stay at their base form; dimensions are unchanged. Wide-sheet
text scaling and header facts were reviewed on real sheets alongside a
340-chart white/black geometry regression.

| File | SHA-256 |
| --- | --- |
| synthetic-print.png | 908ccc607c01cee13cbce60a9be87b9cdd681dd14ebae7d54ac655c84a244d56 |
| synthetic-print-narrow.png | 6b968b0d0eba1196b5550bd9236c24e9cc8fd579d7ed4614207129c0dd0eae30 |

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
`167312b540cbcfe5ea0b4daa521db0bf4a1f544a4119306d9ecd0b5cd211b50d`.
It is the same complete 1152 x 670 synthetic sheet. Check it with `--theme black`.
Native Windows/macOS black pixel baselines have not yet been reviewed.

## Legacy dark theme

All images are the same original synthetic chart, cover and metadata from
`tests/fixtures`, rendered as one complete three-column PNG (1152 x 547).
They contain no game assets. Artwork remains under development.

Run `python scripts/check_golden.py --binary <binary>` to compare against the
current platform's dark baseline. `--update` explicitly replaces that baseline for
manual review. CI never updates baselines. All platforms keep the same mean
absolute channel error limit of 1.0 on the 0-255 scale; dimensions must match.
Text rasterization differs between CoreText, DirectWrite and FreeType, so the
baselines are platform-specific: each platform's own runner must match its own
PNG exactly (0.0 here), and the ~0.85-1.2 cross-platform drift between them is
expected. Never copy one platform's PNG over another's.

| File | Native environment | SHA-256 |
| --- | --- | --- |
| synthetic.png | Linux x86_64 / FreeType | f6ae7cb46f13bb9699c5d3185a36940479d6765b0a67bd55f3ea3085d50d6230 |
| synthetic-macos.png | macos-14 ARM64 / CoreText | 0a49b10c209239a71babc323ba126fd53a39f94e4680f241fe3b22bef10cf964 |
| synthetic-windows.png | windows-2022 x64 / DirectWrite | 11c20e7c614dd4eb5a7c2fa7b011e78da7bc483c90a04dc6fc53527aa134539c |

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
