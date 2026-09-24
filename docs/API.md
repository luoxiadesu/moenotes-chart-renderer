# Embedding API v0.2

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

`Renderer::from_skin_pack(path_to_skin_json)` loads an external game skin.
Reuse a renderer to retain textures/fonts across calls in one worker. No promise
is made that Skia handles are `Send`/`Sync`; allocate one renderer per worker.

`Error.kind` is one of Input, Resources, Layout, Render (non-exhaustive).
Diagnostic wording is descriptive, not a stable machine protocol. Values and
options are pinned for the 0.2 API; incompatible facade changes increment minor
version during 0.x. Render report schema is v3. No ID is a persistent chart ID.

`Metadata` includes title/difficulty/level/artist/author, optional cover path for
CLI I/O, optional master FC and provenance. The memory API uses the explicit
cover byte slice; it never opens `Metadata.cover` itself. Lyrics/composition/
arrangement roles are preserved by the masterdata adapter. Skin/source charts
are caller resources, not embedded game assets.

Counts are distinct: visible glyphs; source judgement notes; generated slide
combos; skipped combos; reconstructed full combo. A separate parse with combo
unit 8 computes statistics. Online master FC is retained separately and a
mismatch warns. Reconstructed counts remain bounded by parser compatibility.

Auto-spacing increases pixels/beat to separate overlapping note bodies, up to
160 px/beat. `auto_spacing=false` keeps the requested scale. Reports also give
arrow/body and mark/body bounding-box intersections; they are conservative
metrics, not alpha-mask collision tests. Global ordering prevents later bodies
from covering arrows. Call relative rhythms and source fade modes are visible
annotations; absolute Call scheduling and animated fades are not invented.

Nonmonotonic lines return an Input error. Complex shared endpoint shapes retain
parser warnings and are only as authoritative as the tested corpus. Native-
parameters mode still excludes perspective, frame clipping and native shaders.
