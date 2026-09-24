# External research validation

Normal build/tests and CI use synthetic, redistributable fixtures only. Optional
`research-tests` use explicit environment paths, never a hard-coded workstation:

- `MOENOTES_TEST_CHARTS`: directory containing external JSON charts.
- `MOENOTES_TEST_PACKS`: directory containing prepared skin001/002/003 packs.
- `MOENOTES_TEST_ARROW_ORACLE`: independent native arrow-selector result JSON.

These assets/oracles are not distributed. Private extraction, emulator and
benchmark evidence tools stay outside this repository. See DEVELOPMENT.md for
public-source boundaries. Whole-chart image tests must produce one complete
multi-column image per chart; historical paginated samples are not the current
acceptance criterion.
