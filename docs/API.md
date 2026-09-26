# Embedding API v0.3

The supported embedding facade is `moenotes_chart_renderer::api`.
Other modules remain accessible for research, but their structs/functions are
implementation detail and may change during 0.x. The facade returns owned bytes;
it does not write files or make network requests.

```rust
use moenotes_chart_renderer::api::{Renderer, RenderOptions, Metadata};
let renderer = Renderer::builtin()?;
let result = renderer.render(
    chart_bytes, RenderOptions::default(),
    Metadata { title: "Song".into(), author: "Writer".into(), ..Default::default() },
    false, "chart.png", cover_png_bytes,
)?;
// result.pages[0].png: the complete multi-column PNG.
// result.report: serializable diagnostics.
```

One chart returns exactly one image. `RenderOptions.columns_per_page` is a
reserved compatibility field and must be zero; nonzero values are rejected.
Image tests and performance benchmarks must not split a chart into pages.

`RenderOptions.theme` defaults to `Theme::Print` (white canvas and print artwork).
Set `Theme::Black` for deep black or `Theme::Dark` for the legacy screen palette.
JSON uses `white` (accepts `print`), `black`, and `dark`.
`output_scale` (0.25..4, default 1) multiplies final PNG pixels without changing
logical layout. Image reports include `logical_width` and `logical_height`.
Pixel and dimension budgets apply after scaling and include rails/appendices;
logical dimensions alone do not reject a safely downscaled export.
White/black presentation reserves responsive header and footer space, so image
height can vary with title/credit wrapping. Score geometry and column cuts do not
change. `Report.chart_offset_y` is the logical-pixel vertical offset from Scene
coordinates to the final sheet. Reported `flick_callouts[].y` already includes it;
apply `output_scale` only when converting those coordinates to exported pixels.
In white/black (`columns_top_aligned`), each column's last measure sits on one
shared top line and unused height is left below shorter columns; legacy dark
keeps bottom-aligned column starts. Time still runs upward at the same scale.
`chart_text_scale` (1..1.8, 1 for dark) enlarges bar numbers, times and side
labels together with their gutters, so column width grows on wide sheets.
`flick_layout` defaults to `FlickLayout::Callout`; `Inline` restores original placement.
Built-in colliding arrows use side rails and leader lines; source ticks and bodies
remain unchanged. Reports preserve `inline_arrow_body_box_overlaps`, remaining
`arrow_body_box_overlaps`, `flick_callouts`, `flick_rail_width` and
`unresolved_flick_note_ids`. External skins retain their original arrow artwork. Narrow images have a minimum
canvas width of 360 px without changing the requested lane width or time scale.

`Renderer::from_skin_pack(path_to_skin_json)` loads an external game skin.
Reuse a renderer to retain textures/fonts across calls in one worker. No promise
is made that Skia handles are `Send`/`Sync`; allocate one renderer per worker.

`Error.kind` is one of Input, Resources, Layout, Render (non-exhaustive).
Diagnostic wording is descriptive, not a stable machine protocol. New required Rust option fields and the default appearance changed in 0.3; use
`..RenderOptions::default()` and select a theme explicitly when migrating. Values and
options are pinned for the 0.3 API; incompatible facade changes increment minor
version during 0.x. Render report schema is v4 (v4 adds `chart_text_scale` and
`columns_top_aligned`; white/black callout y values follow top alignment). No ID is a persistent chart ID.

`Metadata` includes title/difficulty/level/artist/author, optional cover path for
CLI I/O, optional master FC and provenance. The memory API uses the explicit
cover byte slice; it never opens `Metadata.cover` itself. Lyrics/composition/
arrangement roles are preserved by the masterdata adapter. Skin/source charts
are caller resources, not embedded game assets.

Counts are distinct: visible glyphs; source judgement notes; generated slide
combos; skipped combos; reconstructed full combo. A separate parse with combo
unit 8 computes statistics. Online master FC is retained separately and a
mismatch warns. Reconstructed counts remain bounded by parser compatibility.

Auto-spacing increases pixels/beat to separate note bodies and centered Flick
arrows, up to 160 px/beat. Pairs of structural connection anchors 1–2 ticks apart
retain their exact coordinates and do not inflate spacing. The report's additive
`structural_connection_overlaps` field is a subset of `dense_body_overlaps`, not
an extra count; these intersections remain visible in diagnostics. Body bounds
use print geometry or the loaded skin's actual height ratio.
`auto_spacing=false` keeps the requested scale. Reports also give
arrow/body and mark/body bounding-box intersections; they are conservative
metrics, not alpha-mask collision tests; external-skin arrow widths remain
conservative. Global ordering prevents later bodies
from covering arrows. Call relative rhythms and source fade modes are visible
annotations; absolute Call scheduling and animated fades are not invented.

Nonmonotonic lines return an Input error. Complex shared endpoint shapes retain
parser warnings and are only as authoritative as the tested corpus. Native-
parameters mode still excludes perspective, frame clipping and native shaders.
