# Changelog

## 0.2.0 — initial public source preview

- Cross-platform build review: isolate the parser-private `sig_t` typedef from
  macOS headers and preserve LF in hashed metadata fixtures on Windows.

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
