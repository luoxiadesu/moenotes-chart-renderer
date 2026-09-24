# Validation status for the 0.2.0 candidate

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

## Deliberate boundaries

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
