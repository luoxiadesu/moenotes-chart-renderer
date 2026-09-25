# Validation status for the 0.3.0 candidate

## Test coverage

The normal suite uses original synthetic charts/artwork, never private game
fixtures. It covers C parser ABI and error handling, geometry, equal-tick
segments, full-sheet layout, typography, metadata integrity/current-pointer
updates, publication rollback and the memory API. Python tests check dynamic
masterdata updates and vendored/font hashes. The visual baseline is a complete
multi-column PNG; artwork is still under development.

Visual checks compare each complete sheet against its reviewed platform
baseline at the same mean absolute channel error limit of 1.0/255. Bundled
font files are identical, but native Skia font backends produce different
glyph edges and advances. Linux, Windows and macOS baselines and their review
provenance are in [tests/golden](../tests/golden/README.md). CI retains the full
actual/reference/difference images and metrics as synthetic diagnostics.

The full-masterdata performance test resolves the authoritative repository's
current main and includes every MasterLiveMusicScore row from hk-tw-mo, en and
kr. It records missing input, orphan metadata and failure rows, and requires
exactly one complete multi-column PNG for every successful row. See [PERFORMANCE.md](PERFORMANCE.md)
for the measured run and its limitations. Raw charts/images/results remain local.

CI runs native Linux x86_64, Windows x64 and macOS ARM64 build, tests, Clippy,
documentation, relocation and synthetic visual checks. Check the repository's
Actions run for current platform results; configuration alone is not proof of
cross-platform correctness. Public-source checks follow DEVELOPMENT.md.

## WASM and dense Flick follow-up

The built-in callout layout rendered all 340 retained charts, preserving glyph,
branch and FC counts. All 140 previously colliding arrows across 8 charts were
placed into rails with zero remaining arrow/body intersections or unresolved
callouts. Near-coincident structural connections remain explicitly counted.
Native regression tests cover themes, export scaling, callout layout and metadata.

The wasm32-unknown-emscripten SDK passes Node runtime tests and headless Chromium
module-Worker rendering/PNG decode, including a chart/cover downloaded from the
configured online endpoint. See [WASM.md](WASM.md) for target, memory, CORS and
browser support limits. There is no frontend application in this repository.
Native and WASM CI jobs validate source pushes. Consult the current Actions run
for remote results; local validation alone does not imply those jobs passed.

Export-budget regressions cover logical sheets above 64M pixels, logical dimensions
above 32768, rounding at the final-pixel limit, and supersampled allocations.
Native and WASM rendering verify downscaled complete sheets and reject the same
input at an excessive export scale. Build-tool tests cover custom Cargo artifact
paths and refusal to substitute stale or missing output.

## Deliberate boundaries

- The print theme has reviewed Linux complete and narrow visual baselines. Its
  Windows/macOS pixel baselines and physical printer output are not yet verified.
  Native dark baselines are retained; the new default print path has portable
  API/CLI tests. See the golden README for exact scope and provenance.

- Actual source alpha animation and absolute Call scheduling remain unverified;
  static overview shows fade flags and relative Call fractions explicitly.
- Nonmonotonic lines fail. Complex shared graphs retain the parser warning.
- Native-parameters mode is a flat parameter study; no Unity screenshot/GPU/
  IFix/pixel equivalence is claimed.
- Overlap metrics are bounding-box approximations; auto spacing is capped.
- Complex script shaping (Arabic/Indic), GPU, SVG/PDF and playback are not part
  of this PNG release. CJK/Latin strings are covered by bundled fallback tests.
- Output snapshot atomicity applies to `current.json` + immutable generation.
  Loose sibling images are convenience aliases; see OUTPUT.md for crash limits.
