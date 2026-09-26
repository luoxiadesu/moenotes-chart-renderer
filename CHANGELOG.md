# Changelog

## Unreleased — sheet layout

- White/black columns align their last measure to one top line; shorter columns
  leave space below. Headings share one row with each column's end time. Time
  direction, tick scale and column cuts are unchanged; legacy dark stays
  bottom-aligned and pixel-identical.
- Bar numbers, times, side labels and leader lines scale with wide sheets
  (`chart_text_scale` 1–1.8), widening their gutters; images get wider.
- Lane, beat and measure rules snap to whole output pixels; major measures
  are two pixels.
- Wide headers show length and visible TAP/SLIDE/FLICK/TRACE/CRITICAL counts
  beside the title when they fit. EASY/NORMAL/HARD/EXPERT badges are
  blue/green/amber/red with the name printed; other names stay neutral.
- The footer is one muted `moenotes · bdon.moe` line with the reading direction;
  the header keeps the wordmark.
- Report schema v4 adds `chart_text_scale` and `columns_top_aligned`; white/black
  `flick_callouts[].y` follow top alignment.
- Documentation describes critical styling for the built-in artwork; external
  packs keep their own critical mark in white.

## Unreleased — note styling

- White notes use opaque hue fills under their dark outlines (Tap cyan, Slide
  periwinkle), one pale highlight instead of a second dark top edge.
- Built-in Flick arrows are round-capped vector chevrons scaled to the arrow
  height in white, black and side rails; narrow left/right Flick draws one chevron.
  Arrow boxes, spacing and callout positions are unchanged.
- White/black critical notes use a body-sized gold diamond with an outline and a
  warm underlay/glow. `--native-critical` and legacy dark keep their marks.
- Fever ranges are one opaque warm field over the lane bands, with a wider rail;
  the legend matches.
- A constant integral BPM is no longer repeated beside the first measure; short
  Call rhythms (e.g. `50%/100%`) are two-line side labels instead of a CHART NOTES
  row. Sheets without appendix entries omit that section, so image heights may
  shrink; `chart_offset_y`, columns, ticks and report annotations are unchanged.
- Decimal levels such as `27.5` shrink to fit the difficulty badge instead of
  being truncated.
- Legacy dark output is pixel-identical. Linux white, narrow and black
  baselines were re-reviewed.

## Unreleased — sheet presentation

- Enlarge title, cover, difficulty and total Combo on white/black sheets; use
  a responsive moenotes/bdon.moe masthead and a separate branded footer.
- Wrap titles/credits, use actual note shapes in the seven-item legend, position
  column headings above each track and emphasize major measure labels.
- Present overflow events as chart notes with bar/quarter-note-beat positions;
  wrap long entries and expose the chart's vertical presentation offset.
- Reserve brand clearance even without cover/artist metadata, and honor critical
  mark/strict-asset options when drawing the legend.
- Isolate parallel CLI test workspaces even when Windows timestamps coincide.
- Preserve tick/lane geometry, column cuts, full-image export and legacy dark
  rendering. Review new Linux complete and narrow visual baselines.

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
