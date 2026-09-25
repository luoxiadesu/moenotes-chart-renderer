# Changelog

## 0.3.0 — source preview

- WASM32 Emscripten SDK with typed byte input/PNG output, CJK fonts, resource
  adapters and viewport math for other frontends; no application UI.
- White/deep-black presets, independent export scale and dense Flick side rails.
- Validate final and supersampled pixel budgets after export scaling, including
  rails/appendices; large logical sheets can be exported at a smaller resolution.
- Package the exact Cargo-reported WASM artifacts, including custom target
  directories from environment/configuration, without stale default-path fallback.
- Watch-mode recovery after invalid JSON/table/cache errors.
- Default white print theme with dark outlines, low-ink grids/ribbons, distinct
  Trace/critical/guide shapes and retained `--theme dark` screen rendering.
- Fix ambiguous music/difficulty ownership, cover precedence before master
  jacket lookup, and stale external-pack settings breaking the built-in skin.
- Keep narrow-sheet metadata on the canvas; center tracks and stack timestamps.
- Reserve Flick arrow clearance; do not stretch entire charts for structural
  connection pairs 1–2 ticks apart. Preserve exact ticks and report intersections.
- Match collision body bounds to the selected artwork; make text truncation
  linear in label length and enforce the 64-million-pixel final PNG limit.
- Add CLI regressions and reviewed Linux complete/narrow print baselines.
  Existing native dark baselines remain in use on all three platforms.

## 0.2.0 — initial public source preview

- Cross-platform build review: isolate the parser-private `sig_t` typedef from
  macOS headers and preserve LF in hashed metadata fixtures on Windows.
- Read and write tooling JSON as UTF-8 on every platform; retain full synthetic
  image diagnostics for cross-platform visual checks.
- Review native Linux, macOS and Windows full-sheet baselines separately to
  account for native font rasterization, retaining the original error threshold.

Artwork remains work in progress; this is not the final visual design.

- Complete multi-column single-image output. No chart pagination; full-master
  performance tests use the same full-sheet output contract.
- Unifont Upper fallback covers supplementary-plane symbols in live song titles.

- Vendored pinned parser and checked-in ABI, bundled zlib, original default skin
  and licensed embedded Latin/CJK fonts; no research workspace runtime dependency.
- Supported memory-rendering facade, categorized errors and owned page bytes.
- Live StarMoe-org masterdata synchronization with current pointers and optional
  watch mode; per-run commit/hash provenance, cover/title/role-aware author
  credits, artist/difficulty/level/FC and `moenotes bdon.moe` footer.
- Immutable output generations, atomic current pointer, locking, rollback and
  stale alias removal. Schema v3 adds statistics and decoration collision metrics.
- Auto-spacing, explicit Call rhythm/fade annotations, synthetic visual baseline,
  relocation/metadata/API tests, benchmarks and native multi-OS CI configuration.

## 0.1.0 — local research preview

- Game skin conversion, CPU Skia component samples and complete beat-axis PNGs.
- Balanced columns, CJK fallback, independent sprite sizing, explicit segments,
  equal-tick transitions, mirrored curve modes and visual acceptance material.
