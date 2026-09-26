# Print sheets

The default `white` theme (`print` alias) uses a white canvas, dark text, light grids and pale
slide fills. The built-in notes have dark outlines over opaque fills
that follow the screen hues, so Tap (cyan) and Slide (periwinkle) differ in hue
and lightness; Trace notes have a double line, critical notes a gold diamond
with a warm underlay, and guides dashed edges. Outlines and shapes keep these
cues visible in grayscale. The note styling below describes the built-in
artwork. External packs retain their original sprites with a body outline (in
white their critical mark is the pack's own); use the built-in artwork
for the most consistent contrast on paper.

## Notes, Flick, critical and fever

- Built-in Flick arrows use round-capped vector chevrons whose stroke scales
  with `--arrow-height`; left/right Flick narrower than 14 px draws one chevron.
  The arrow box, its spacing reservation and callout rails are unchanged.
- With the built-in artwork, a critical diamond is sized to the note body (about 60% of
  `--note-height` each side of center, at least 3 px) with a dark outline and a
  warm glow/underlay. `--native-critical` keeps the source mark; legacy `dark`
  keeps its original small mark.
- Black keeps skin sprites for bodies and draws built-in Flick arrows with the
  same vector chevrons as rails.
- A fever range is one opaque warm field across the track, replacing the lane
  bands beneath it, with a 2.5 px rail on the left.

## Columns, chart text and grid

- Every column's last measure sits on one shared top line; shorter columns end
  above the bottom, so all columns start reading at the same height. Time still
  runs upward at one scale. Column headings share a single row, with each
  column's end time right-aligned beside its heading when both fit the track.
- Bar numbers, times, side labels and their leader lines scale with the sheet
  (`chart_text_scale`, 1–1.8 following the header). The bar-number and label
  gutters widen by the same factor; lane width and time scale are unchanged.
- Lane, beat and measure rules are placed on whole output pixels. Measure
  lines are one pixel (major every four measures: two pixels); beat/lane
  hierarchy uses opacity, not sub-pixel widths, so rules stay crisp after
  supersampling and export scaling.

## Header and footer

- Wide sheets show chart facts between the title and the wordmark: length and
  visible TAP/SLIDE/FLICK/TRACE/CRITICAL counts. SLIDE counts starts, relay
  points and ends, so the four kinds sum to the visible note count; CRITICAL is
  a flag on other kinds. Facts are omitted when they would meet a long title.
- EASY/NORMAL/HARD/EXPERT badges use ordered blue/green/amber/red fills with the
  name and level always printed; other difficulty names keep the neutral badge.
  The fills were checked with the dataviz palette validator on each surface.
- The header keeps the moenotes wordmark and bdon.moe. The footer is a single
  muted line, `moenotes · bdon.moe`, with the reading direction when it fits.

## Side labels

- A constant integral BPM, already exact in the header, is not repeated beside
  the first measure. Varying or non-integral tempos stay beside the chart and
  every source value remains in `annotations` in the report.
- A Call with a short rhythm list (up to 8 characters, e.g. `50%/100%`) is a
  two-line side label. Longer lists keep `CALL ↳` and appear under CHART NOTES.
- A label either fits beside its column in full or moves to CHART NOTES.
  `annotation_overflow` counts rows in that appendix; the section is omitted
  when there is nothing to list.
- Decimal master levels such as `27.5` shrink to fit the difficulty badge
  instead of being truncated. Integral levels keep their size.

White and deep black now share a responsive information hierarchy. Large sheets
use a 75 px title, a 190 px cover, a difficulty badge and grouped BPM/total Combo;
the moenotes wordmark and bdon.moe site occupy their own masthead area. Small sheets
use compact sizes and stacked rows. Titles and credits wrap to two lines, with
ellipsis for any remaining text; the full strings remain in the report.
The seven-item legend uses note/guide/fever shapes. Major measure labels have
greater weight. The chart-notes section uses bar and quarter-note-beat
coordinates rather than raw tick numbers.

```sh
moenotes-chart-renderer render chart.json -o print.png
moenotes-chart-renderer render chart.json -o screen.png --theme black
moenotes-chart-renderer render chart.json -o narrow.png --long --pixels-per-lane 2
```

Every command outputs one complete chart. PNGs are not assigned an A4/Letter
physical page size, split for printing, or automatically downscaled. Choose a
paper size and print scale appropriate to the complete sheet. Physical printer
output has not been validated.

Narrow sheets use at least 360 px of canvas width, center their tracks, and put
the start and end timestamps on separate lines when the track is under 140 px.
The cover, title, FC and wrapped legend remain within the canvas.

## Dense charts

Auto-spacing considers body height and centered Flick arrows. It stays capped
at 160 px/beat, and `--fixed-spacing` retains the caller's requested scale.
Two connection anchors only 1–2 ticks apart do not force the entire sheet to its
maximum spacing. Their exact ticks, glyphs and overlaps remain in the scene and
report. `structural_connection_overlaps` is a subset of `dense_body_overlaps`;
neither is a count of omitted notes. Ordinary body intersections and arrow/body
intersections produce separate warnings. Geometry metrics are bounding boxes,
not alpha-mask collision tests; external arrow widths are conservative.

The built-in artwork now defaults to `--flick-layout callout`: an arrow that
intersects a later body is placed in a side rail at its source note's exact y,
with a dashed leader. Arrows retain their size and direction; bodies and ticks
are unchanged. Separate rail slots prevent arrows overlapping one another.
`--flick-layout inline` restores original placement. External skins retain their
source arrows. Rail width, moved note IDs, inline/remaining intersections and
unresolved IDs are reported. Pathological charts exceeding 16 slots retain
unresolved warnings rather than silently omitting arrows.

## Local regression review, 2026-09-25

- All 340 distinct retained charts rendered as complete print PNGs using the
  built-in artwork, requested 64 px/beat, 10 px/lane, 8 px body, 10 px arrow,
  automatic spacing and 2x supersampling. PNG decode, dimensions, image SHA,
  visible glyph count, branch count and reconstructed FC were checked.
- Zero render failures, missing visible glyphs or FC changes relative to the
  retained reference run. No nonstructural body-box overlaps were reported.
- Three charts retain 8 near-coincident connection-pair overlaps. Their spacing
  changed from 160 px/beat to 64, 66 and 132 px/beat respectively; original
  tick coordinates were preserved.
- Eight charts previously had 140 colliding Flick arrows at the spacing cap.
  Built-in callout mode places all 140 into side rails: zero remaining
  arrow/body box overlaps and zero unresolved callouts in the 340-chart run.
  This is presentation placement, not a change to the source chart.
- Twelve additional complete mirrored sheets cover three representative charts
  with the built-in artwork and skin001/002/003, including source-arrow fallback.
- The corpus uses retained hk-tw-mo metadata from commit
  `49ea87dc076ba0c5445bfa8e76643a5d9bb266cc`. It does not claim a new live-master
  sync, a fresh CDN download, or another 1,020-row regional benchmark. The normal
  synchronization tool continues to follow upstream main.
- Linux complete and narrow print baselines were reviewed in color and the
  complete synthetic sheet in grayscale. Native Windows/macOS print pixel
  baselines remain unreviewed; the existing dark baselines remain active.

Raw game charts, rendered game sheets and per-chart diagnostics stay outside
version control. Public visual fixtures are synthetic. The initial print-only review used
binary `86542e7ad10be146baa8f330e22bc1725ae986f79def8bdb9b2be658c53c1437`; the
later callout regression is recorded separately in local validation evidence.
