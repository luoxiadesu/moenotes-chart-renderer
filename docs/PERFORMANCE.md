# Full-masterdata single-image benchmark

Measured 2026-09-25 (Asia/Tokyo). The acceptance unit is **one complete multi-column
PNG per score row**, never paginated. Artwork is still under development.
This historical run used the original dark theme and spacing algorithm before
the print refinement. It is not a new benchmark of the current default print
theme. See [PRINT.md](PRINT.md) for the later retained-corpus regression scope.

## Inputs and coverage

- Live source: https://github.com/StarMoe-org/moenotes-masterdata, current main at
  test start and end. Observed commit `49ea87dc076ba0c5445bfa8e76643a5d9bb266cc` was unchanged during
  this run. It is evidence for this measurement, not a fixed update version.
- Regions: hk-tw-mo, en, kr; 340 MasterLiveMusicScore rows each, 1,020 renders total.
- 340 unique chart keys and 340 retained local chart contents; the same chart
  content is reused across regional metadata. These inputs are not claimed to
  be freshly downloaded from the current game CDN.
- All 1,020 outputs passed one-image, dimensions, PNG decode and SHA-256 checks.
- Four `0047` score rows per region have no MasterLiveMusic owner (12 total).
  They are included with the chart key/score level/FC and explicit unknown title,
  author and cover association. No stale table supplies missing song metadata.
- No input chart was missing; no quality reduction or pagination fallback occurred.

## Configuration and environment

Linux WSL2, Intel Core i7-11800H, about 23.5 GiB visible RAM, Rust 1.98.1,
skia-safe 0.153.3, parser 0.3.0. Sequential fresh CLI processes (one at a time).
Skin001, 64 px/beat requested with auto-spacing enabled, 10 px/lane, 8 px body,
10 px arrow height, 2x supersampling, balanced 24-beat target columns. Regional
metadata uses Japanese text, cover and role-aware author credits where available.
Largest output width: 8,512 px; largest output height:
4,675 px (not necessarily the same image).

| Stage / resource | Minimum | Median | Mean | P95 | Maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| CLI wall seconds | 0.379 | 0.736 | 0.807 | 1.334 | 2.696 |
| Draw + PNG encode seconds | 0.305 | 0.654 | 0.726 | 1.252 | 2.619 |
| Peak RSS MiB | 150.5 | 226.8 | 246.3 | 371.9 | 693.8 |

Summed command wall time: 823.0 s; summed draw/encode:
740.4 s. Harness loop including PNG verification and
repository end-check: 912.0 s; total including initial metadata
sync: 951.0 s. Drawing excludes metadata loading and
publication; complete command wall time includes them. RSS is measured per process
by `/usr/bin/time`; per-process peaks are not added into a memory requirement.

## Failure handling and reproducibility

An initial run produced 996 images and 24 errors: two song titles across four
difficulties and three regions used U+1F701/U+1F703 absent from the original font
fallback. The unmodified licensed Unifont Upper font was added. A complete fresh
run of all 1,020 rows with one uniform rebuilt binary then passed. The failed run
was retained locally; it was not combined with successes to claim a clean run.

The script follows repository main and advances regional `current.json` pointers.
Per-run commits/hashes are preserved so updates do not mix tables mid-render.
`per-chart.jsonl`, CSV, manifests, images and detailed process logs stay local;
only this aggregate report and reusable benchmark code are public.

```sh
python3 scripts/benchmark_masterdata.py --binary ./moenotes-chart-renderer \
  --assets /path/to/by-key --packs /path/to/packs --skin skin001 --theme dark \
  --region all --output output/benchmark
```

Results establish successful complete-image generation and the measured cost on
this host. They are not exhaustive visual correctness, current-CDN equality,
parallel throughput, Windows/macOS performance or game-pixel equivalence.
