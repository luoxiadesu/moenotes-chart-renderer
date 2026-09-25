# Development and release rules

Keep production source, synthetic fixtures, English documentation, Cargo.lock and
third-party notices in this repository. Do not commit game charts, game skins or
covers, APKs, extracted binaries, private catalogs, credentials, machine-specific
configuration, raw benchmark output or generated build artifacts. Private
research tools and evidence stay outside the repository.

Before a source commit intended for push:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo doc --no-deps --locked
cargo build --release --locked
python3 -m unittest discover -s tests -p 'test_*.py'
python3 scripts/verify_release.py --binary target/release/moenotes-chart-renderer
python3 scripts/check_golden.py --binary target/release/moenotes-chart-renderer
# On Linux, also check the reviewed print baselines.
python3 scripts/check_golden.py --binary target/release/moenotes-chart-renderer --theme print
python3 scripts/check_golden.py --binary target/release/moenotes-chart-renderer --theme print --narrow
python3 scripts/check_golden.py --binary target/release/moenotes-chart-renderer --theme black
git diff --check
```

Review code and the staged diff after performance tests. Scan staged text and
file names for private paths, credentials and unintended binaries. Preserve
third-party notices unchanged. Update CHANGELOG, API contracts and usage docs.
Record validation scope honestly; no test implies pixel-identical game behavior.

Rendering tests use a complete multi-column single PNG for each chart. Never
split a chart into pages to make a benchmark pass or silently lower quality.
Record errors, missing chart assets and missing music metadata explicitly.
Performance records include configuration, repository state observed for the
run, per-chart process/render time, RSS and one-image validation. Live metadata
follows the authoritative masterdata repository; a commit is run provenance,
not the project's configured update version.

Artwork is still under development. Golden images detect regressions in the
current implementation; they do not freeze the final visual design. Regenerate
a golden explicitly and review the change before committing it.
`check_golden.py --update` updates only the current platform's complete sheet.
Record its source commit/runner and hash in `tests/golden/README.md` after review;
never update a failing baseline automatically in CI.
Print pixel baselines currently exist only for Linux. On another native platform,
review its complete print outputs before establishing platform-specific baselines;
keep running the existing native dark baseline and default-print API/CLI tests.

When changing rendering or the WASM boundary, build with `scripts/build_wasm.py`,
run `node tests/wasm-smoke.mjs`, and run `scripts/check_wasm_browser.py` with
Playwright/Chromium. These tests exercise the SDK without adding a frontend app.

CI uploads only synthetic artifacts and code packages. Public source push does
not authorize a tag, GitHub Release, container deployment or game-asset upload.
