# Print sheets

The default `white` theme (`print` alias) uses a white canvas, dark text, light grids and pale
slide fills. The original built-in notes have dark outlines and top edges;
Trace notes have a double line, critical notes a diamond, and guides dashed
edges. These cues remain visible when converted to grayscale. External packs
retain their original sprites with a body outline; use the built-in artwork
for the most consistent contrast on paper.

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
