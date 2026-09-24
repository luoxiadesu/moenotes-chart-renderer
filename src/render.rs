//! Page-oriented Skia presentation. Geometry is already resolved in Scene.
use crate::{
    Skin,
    layout::{self, Column},
    scene::{Ribbon, Row, Scene},
    sha256,
    skin::{NotePart, PreviewNote, gradient_color, rgba},
    typography::Fonts,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use skia_safe::{self as sk, Canvas, Color, Paint, Rect};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
const BG: Color = Color::from_rgb(13, 20, 29);
const PANEL: Color = Color::from_rgb(20, 29, 40);
const FG: Color = Color::from_rgb(228, 237, 246);
const MUTED: Color = Color::from_rgb(137, 157, 177);
const GOLD: Color = Color::from_rgb(240, 204, 119);
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub master_full_combo: Option<u64>,
    #[serde(default)]
    pub provenance: Option<serde_json::Value>,
}
#[derive(Debug, Serialize)]
pub struct ImageReport {
    pub file: String,
    pub width: i32,
    pub height: i32,
    pub sha256: String,
    pub columns: [usize; 2],
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub parser_version: String,
    pub skin: String,
    pub mirror: bool,
    pub metadata: Metadata,
    pub source_notes: usize,
    pub glyphs: usize,
    pub branches: usize,
    pub curve_points: usize,
    pub instantaneous_segments: usize,
    pub hidden_nodes: usize,
    pub zero_width_nodes: usize,
    pub outside_viewport_glyphs: usize,
    pub bars: usize,
    pub columns: usize,
    pub images: Vec<ImageReport>,
    pub warnings: Vec<String>,
    pub missing_arrow_note_ids: Vec<i32>,
    pub fallback_arrow_note_ids: Vec<i32>,
    pub annotations: Vec<crate::scene::Annotation>,
    pub layout: crate::layout::Options,
    pub unused_vertical_fraction: f64,
    pub track_width_fraction: f64,
    pub dense_body_overlaps: usize,
    pub arrow_body_box_overlaps: usize,
    pub mark_body_box_overlaps: usize,
    pub annotation_overflow: usize,
    pub statistics: crate::parser::Statistics,
}
fn paint(color: Color) -> Paint {
    let mut p = Paint::default();
    p.set_anti_alias(true).set_color(color);
    p
}
fn stroke(c: &Canvas, a: (f64, f64), b: (f64, f64), color: Color, width: f32) {
    let mut p = paint(color);
    p.set_stroke_width(width);
    c.draw_line((a.0 as f32, a.1 as f32), (b.0 as f32, b.1 as f32), &p);
}
fn panel(c: &Canvas, r: Rect, color: Color) {
    c.draw_round_rect(r, 6., 6., &paint(color));
}
fn band_geometry(
    rows: &[Row],
    col: &Column,
    scene: &Scene,
    x: f64,
    inset: f64,
) -> (sk::Path, sk::Path, sk::Path) {
    let rows: Vec<_> = rows
        .iter()
        .filter(|v| col.start <= v.tick && v.tick <= col.end)
        .collect();
    let mut fill = sk::PathBuilder::new();
    let mut left = sk::PathBuilder::new();
    let mut right = sk::PathBuilder::new();
    for (i, row) in rows.iter().enumerate() {
        let mid = (row.left + row.right) / 2.;
        let l = (row.left + inset).min(mid);
        let r = (row.right - inset).max(mid);
        let y = scene.layout.y(col, row.tick as f64) as f32;
        let lp = ((x + l * scene.layout.options.pixels_per_lane) as f32, y);
        let rp = ((x + r * scene.layout.options.pixels_per_lane) as f32, y);
        if i == 0 {
            fill.move_to(lp);
            left.move_to(lp);
            right.move_to(rp);
        } else {
            fill.line_to(lp);
            left.line_to(lp);
            right.line_to(rp);
        }
    }
    for row in rows.iter().rev() {
        let r = (row.right - inset).max((row.left + row.right) / 2.);
        fill.line_to((
            (x + r * scene.layout.options.pixels_per_lane) as f32,
            scene.layout.y(col, row.tick as f64) as f32,
        ));
    }
    fill.close();
    (fill.detach(), left.detach(), right.detach())
}
fn draw_ribbon(c: &Canvas, r: &Ribbon, col: &Column, scene: &Scene, skin: &Skin, x: f64) {
    let g = &skin.manifest.line.normal;
    let base = if r.guide {
        let g = &skin.manifest.line.guide_color;
        rgba([g.r, g.g, g.b], g.a)
    } else {
        gradient_color(g, 0.5)
    };
    for segment in &r.segments {
        let Some(a) = segment.first() else {
            continue;
        };
        let b = segment.last().unwrap();
        if b.tick < col.start || a.tick > col.end {
            continue;
        }
        if a.tick == b.tick {
            if scene.layout.owner(a.tick) == scene.layout.owner(col.start) {
                let y = scene.layout.y(col, a.tick as f64);
                stroke(
                    c,
                    (x + a.left * scene.layout.options.pixels_per_lane, y),
                    (x + b.left * scene.layout.options.pixels_per_lane, y),
                    base,
                    1.,
                );
                stroke(
                    c,
                    (x + a.right * scene.layout.options.pixels_per_lane, y),
                    (x + b.right * scene.layout.options.pixels_per_lane, y),
                    base,
                    1.,
                );
            }
            continue;
        }
        let min_width = segment
            .iter()
            .map(|v| (v.right - v.left).max(0.))
            .fold(f64::INFINITY, f64::min);
        let inset = if r.guide {
            0.
        } else {
            ((1. - skin.manifest.line.width_scale as f64) * 6.).min(min_width / 2.) / 2.
        };
        let (path, left, right) = band_geometry(segment, col, scene, x, inset);
        let mut p = paint(base);
        if r.guide {
            p.set_alpha_f(0.20);
        } else {
            let positions = [0., 0.5, 1.];
            let colors: Vec<sk::Color4f> = positions
                .iter()
                .map(|&t| {
                    let color = gradient_color(g, t);
                    let mut f = sk::Color4f::from(color);
                    f.a = 0.29;
                    f
                })
                .collect();
            let colors =
                sk::gradient::Colors::new(&colors, Some(&positions), sk::TileMode::Clamp, None);
            let gradient =
                sk::gradient::Gradient::new(colors, sk::gradient::Interpolation::default());
            let begin = r.rows.first().unwrap().tick;
            let end = r.rows.last().unwrap().tick;
            if begin != end {
                p.set_shader(sk::shaders::linear_gradient(
                    (
                        (x as f32, scene.layout.y(col, begin as f64) as f32),
                        (x as f32, scene.layout.y(col, end as f64) as f32),
                    ),
                    &gradient,
                    None,
                ));
            }
        }
        c.draw_path(&path, &p);
        // Thin edge light plus restrained outer glow. No noisy per-polygon borders
        // at segment joins. This is a documented static style, not a Unity shader.
        if !r.guide {
            let mut glow = paint(base);
            glow.set_style(sk::paint::Style::Stroke)
                .set_stroke_width(2.2)
                .set_alpha_f(0.12);
            glow.set_mask_filter(sk::MaskFilter::blur(sk::BlurStyle::Normal, 1.1, None));
            c.draw_path(&left, &glow);
            c.draw_path(&right, &glow);
        }
        let mut edge = paint(base);
        edge.set_style(sk::paint::Style::Stroke)
            .set_stroke_width(if r.guide { 0.7 } else { 1.0 })
            .set_alpha_f(if r.guide { 0.36 } else { 0.70 });
        c.draw_path(&left, &edge);
        c.draw_path(&right, &edge);
    }
}
fn event_color(kind: &str) -> Color {
    match kind {
        "bpm" => Color::from_rgb(246, 181, 106),
        "meter" => Color::from_rgb(169, 174, 232),
        "skill" => Color::from_rgb(238, 143, 184),
        _ => Color::from_rgb(108, 203, 182),
    }
}
/// Dense tempo ramps get an explicit first→last label plus a complete appendix.
/// Other event types remain distinct, including simultaneous Skill and meter.
fn display_events(scene: &Scene, ci: usize) -> (Vec<crate::scene::Annotation>, Vec<String>) {
    let events: Vec<_> = scene
        .annotations
        .iter()
        .filter(|a| scene.layout.owner(a.tick) == Some(ci))
        .cloned()
        .collect();
    let mut bpm: Vec<_> = events.iter().filter(|a| a.kind == "bpm").cloned().collect();
    bpm.sort_by_key(|a| a.tick);
    let mut shown: Vec<_> = events.into_iter().filter(|a| a.kind != "bpm").collect();
    let mut details = vec![];
    let mut i = 0;
    while i < bpm.len() {
        let mut end = i + 1;
        while end < bpm.len()
            && (bpm[end].tick - bpm[end - 1].tick) as f64 / 480.
                * scene.layout.options.pixels_per_beat
                < 18.
        {
            end += 1;
        }
        if end - i >= 4 {
            let short = |s: &str| {
                s.trim_end_matches(" BPM")
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            };
            shown.push(crate::scene::Annotation {
                tick: bpm[i].tick,
                label: format!("{}→{}", short(&bpm[i].label), short(&bpm[end - 1].label)),
                kind: "bpm".into(),
            });
            for a in &bpm[i..end] {
                details.push(format!("@{}  {}", a.tick, a.label));
            }
        } else {
            shown.extend_from_slice(&bpm[i..end]);
        }
        i = end;
    }
    for event in &mut shown {
        if event.kind == "call" && event.label.len() > 12 {
            details.push(format!("@{}  {}", event.tick, event.label));
            event.label = "CALL ↳".into();
        }
    }
    shown.sort_by_key(|a| std::cmp::Reverse(a.tick));
    (shown, details)
}
/// Stable monotone label packing with leader lines; equal-time events remain
/// separate labels. Overflow is listed in an appendix panel, never summarized away.
pub fn pack_labels(desired: &[f64], top: f64, bottom: f64, gap: f64) -> (Vec<f64>, usize) {
    let capacity = ((bottom - top) / gap).floor().max(0.) as usize;
    let count = desired.len().min(capacity);
    let mut ys = Vec::new();
    for &y in &desired[..count] {
        ys.push(y.max(ys.last().map(|v| v + gap).unwrap_or(top)));
    }
    if let Some(&last) = ys.last()
        && last > bottom
    {
        let mut next = bottom + gap;
        for y in ys.iter_mut().rev() {
            *y = (*y).min(next - gap);
            next = *y;
        }
    }
    (ys, desired.len() - count)
}
fn metric_bpm(scene: &Scene) -> String {
    let mut vals = scene
        .annotations
        .iter()
        .filter(|a| a.kind == "bpm")
        .filter_map(|a| {
            a.label
                .strip_suffix(" BPM")
                .and_then(|s| s.parse::<f64>().ok())
        });
    let first = vals.next().unwrap_or(120.);
    let (min, max) = vals.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v)));
    if (max - min).abs() < 0.01 {
        format!("{min:.0} BPM")
    } else {
        format!("{min:.0}–{max:.0} BPM")
    }
}
fn note_height(role: &str, height: f64) -> f64 {
    match role {
        "trace" => height * 0.72,
        "connection" => height * 0.53,
        _ => height,
    }
}
fn overlap_count(scene: &Scene) -> usize {
    let mut n = 0;
    let glyphs = &scene.glyphs;
    for (i, a) in glyphs.iter().enumerate() {
        for b in glyphs.iter().skip(i + 1) {
            let dt =
                (b.note.tick - a.note.tick) as f64 / 480. * scene.layout.options.pixels_per_beat;
            if dt > scene.layout.options.note_height {
                break;
            }
            if dt > 0.
                && a.column == b.column
                && a.note.left < b.note.right
                && b.note.left < a.note.right
                && dt
                    < (note_height(a.note.role().unwrap(), scene.layout.options.note_height)
                        + note_height(b.note.role().unwrap(), scene.layout.options.note_height))
                        / 2.
            {
                n += 1;
            }
        }
    }
    n
}
fn decoration_overlap_count(scene: &Scene) -> (usize, usize) {
    let mut arrows = 0;
    let mut marks = 0;
    let unit = scene.layout.options.pixels_per_beat / 480.;
    for (i, a) in scene.glyphs.iter().enumerate() {
        for b in scene.glyphs.iter().skip(i + 1) {
            let delta = (b.note.tick - a.note.tick) as f64 * unit;
            if delta > scene.layout.options.arrow_height + scene.layout.options.note_height + 3. {
                break;
            }
            if delta <= 0.
                || a.column != b.column
                || a.note.left >= b.note.right
                || b.note.left >= a.note.right
            {
                continue;
            }
            if a.note.role().is_some_and(|r| r.starts_with("flick"))
                && delta
                    < scene.layout.options.arrow_height + scene.layout.options.note_height + 1.5
            {
                arrows += 1;
            }
            if a.note.critical && delta < (scene.layout.options.note_height + 4.6) / 2. {
                marks += 1;
            }
        }
    }
    (arrows, marks)
}
pub struct Page {
    pub name: String,
    pub png: Vec<u8>,
}
pub struct Rendered {
    pub pages: Vec<Page>,
    pub report: Report,
}
pub fn render_memory(
    scene: &Scene,
    skin: &Skin,
    fonts: &Fonts,
    metadata: &Metadata,
    mirror: bool,
    basename: &str,
    cover_bytes: Option<&[u8]>,
) -> Result<Rendered> {
    let output = Path::new(basename);
    ensure!(
        [
            &metadata.title,
            &metadata.difficulty,
            &metadata.level,
            &metadata.artist,
            &metadata.author
        ]
        .iter()
        .all(|s| s.len() <= 4096),
        "Metadata field exceeds 4096 bytes"
    );
    ensure!(
        basename.len() <= 180
            && !basename.contains(['/', '\\'])
            && basename != "."
            && basename != "..",
        "Unsafe output basename"
    );
    ensure!(
        output.file_name().is_some_and(|n| n == output.as_os_str())
            && output.extension().is_some_and(|e| e == "png"),
        "Output basename must be a simple .png filename"
    );
    let cover = if let Some(bytes) = cover_bytes {
        ensure!(bytes.len() <= 16 * 1024 * 1024, "Cover exceeds 16 MiB");
        let image =
            sk::Image::from_encoded(sk::Data::new_copy(bytes)).context("Cover image decode")?;
        ensure!(
            image.width() > 0
                && image.height() > 0
                && image.width() as i64 * image.height() as i64 <= 16_000_000,
            "Cover exceeds pixel budget"
        );
        Some(image)
    } else {
        None
    };
    let mut output_pages = vec![];
    let mut encoded_bytes = 0usize;
    ensure!(
        !metadata.title.is_empty() && metadata.title.len() <= 512,
        "Title must contain 1..512 UTF-8 bytes"
    );
    ensure!(
        [
            &metadata.title,
            &metadata.difficulty,
            &metadata.level,
            &metadata.artist,
            &metadata.author
        ]
        .iter()
        .all(|s| fonts.supports(s)),
        "Metadata contains glyphs absent from the bundled fonts"
    );
    ensure!(
        [
            &metadata.title,
            &metadata.difficulty,
            &metadata.level,
            &metadata.artist,
            &metadata.author
        ]
        .iter()
        .all(|s| s.chars().all(|c| !c.is_control())),
        "Metadata must be single-line printable text"
    );
    let pages = scene.layout.pages();
    let mut images = vec![];
    let mut missing = BTreeSet::new();
    let mut fallbacks = BTreeSet::new();
    let mut overflow_count = 0;
    let ids: BTreeMap<_, _> = scene.glyphs.iter().map(|g| (g.note.id, &g.note)).collect();
    let outside = scene
        .glyphs
        .iter()
        .filter(|g| g.note.right <= 0. || g.note.left >= 24.)
        .count();
    for (page, range) in pages.iter().enumerate() {
        let width = scene.layout.page_width(range.len())?;
        let mut overflows: Vec<String> = vec![];
        // Reserve a legible appendix if an exceptionally dense column has more event
        // labels than can be placed beside it (common in tempo-ramp test charts).
        for ci in range.clone() {
            let col = &scene.layout.columns[ci];
            let top = scene.layout.y(col, col.end as f64);
            let bottom = scene.layout.y(col, col.start as f64);
            let (events, details) = display_events(scene, ci);
            overflows.extend(details);
            let cap = ((bottom - top + 30.) / 12.).floor() as usize;
            overflows.extend(
                events
                    .into_iter()
                    .skip(cap)
                    .map(|a| format!("@{}  {}", a.tick, a.label)),
            );
        }
        let appendix_columns = ((width as usize).saturating_sub(40) / 220).clamp(1, 3);
        let appendix_rows = overflows.len().div_ceil(appendix_columns);
        let height = scene.layout.height
            + (appendix_rows as i32 * 17)
            + if overflows.is_empty() { 0 } else { 36 };
        ensure!(
            height <= 32768 && width as i64 * height as i64 <= 64_000_000,
            "Full sheet with annotation appendix exceeds image limits"
        );
        overflow_count += overflows.len();
        let ss = scene.layout.options.supersample as i32;
        ensure!(
            width as i64 * height as i64 * (ss * ss) as i64 <= 256_000_000,
            "Supersampled full sheet exceeds memory limit"
        );
        let mut surface = sk::surfaces::raster_n32_premul((width * ss, height * ss))
            .context("Cannot allocate page")?;
        let c = surface.canvas();
        c.scale((ss as f32, ss as f32));
        c.clear(BG);
        let compact = width < 600;
        let cover_size = if compact { 52. } else { 80. };
        let text_x = if cover.is_some() {
            cover_size + 34.
        } else {
            20.
        };
        let content_width = width as f32 - text_x as f32 - 20.;
        if let Some(image) = &cover {
            c.save();
            c.clip_rect(
                Rect::from_xywh(20., 20., cover_size as f32, cover_size as f32),
                None,
                true,
            );
            let side = image.width().min(image.height()) as f32;
            let src = Rect::from_xywh(
                (image.width() as f32 - side) / 2.,
                (image.height() as f32 - side) / 2.,
                side,
                side,
            );
            c.draw_image_rect_with_sampling_options(
                image,
                Some((&src, sk::canvas::SrcRectConstraint::Strict)),
                Rect::from_xywh(20., 20., cover_size as f32, cover_size as f32),
                sk::FilterMode::Linear,
                &Paint::default(),
            );
            c.restore();
        }
        fonts.draw(
            c,
            &metadata.title,
            text_x,
            42.,
            if compact { 16. } else { 25. },
            FG,
            true,
            content_width,
        );
        let difficulty = if metadata.difficulty.is_empty() {
            "CHART"
        } else {
            &metadata.difficulty
        };
        let subtitle = format!(
            "{difficulty} {}   /   {}   /   {} notes{}",
            metadata.level,
            metric_bpm(scene),
            scene.glyphs.len(),
            if mirror { "   /   MIRROR" } else { "" }
        );
        fonts.draw(
            c,
            &subtitle,
            if compact { 20. } else { text_x },
            if compact { 89. } else { 65. },
            if compact { 9.5 } else { 11.5 },
            MUTED,
            false,
            if compact {
                width as f32 - 40.
            } else {
                content_width
            },
        );
        let fc = metadata
            .master_full_combo
            .map(|v| format!("FC {v} (master)"))
            .unwrap_or_else(|| {
                format!(
                    "FC {} (reconstructed)",
                    scene.statistics.reconstructed_full_combo
                )
            });
        fonts.draw(c, &fc, width as f64 - 205., 101., 10., MUTED, false, 185.);
        if !metadata.artist.is_empty() {
            fonts.draw(
                c,
                &metadata.artist,
                text_x,
                if compact { 64. } else { 84. },
                10.5,
                MUTED,
                false,
                content_width,
            );
        }
        if !metadata.author.is_empty() {
            fonts.draw(
                c,
                &metadata.author,
                20.,
                118.,
                10.,
                MUTED,
                false,
                width as f32 - 40.,
            );
        }
        // Compact visual legend instead of debug implementation text.
        let ly = 141.;
        let labels = [
            ("TAP", Color::from_rgb(117, 199, 226)),
            ("SLIDE", Color::from_rgb(124, 148, 243)),
            ("FLICK", Color::from_rgb(238, 189, 94)),
            ("TRACE", Color::from_rgb(181, 156, 230)),
            ("CRITICAL", GOLD),
            ("FEVER", Color::from_rgb(191, 163, 95)),
        ];
        let mut lx = 20.;
        for (label, color) in labels {
            if lx + 80. > width as f64 {
                break;
            }
            c.draw_circle((lx as f32 + 3., ly as f32 - 3.), 2.4, &paint(color));
            fonts.draw(c, label, lx + 11., ly, 9., MUTED, false, 80.);
            lx += if label == "CRITICAL" { 83. } else { 66. };
        }
        stroke(
            c,
            (20., layout::HEADER - 31.),
            (width as f64 - 20., layout::HEADER - 31.),
            Color::from_rgb(42, 55, 69),
            0.8,
        );
        fonts.draw(
            c,
            &format!("{:02} / {:02}", page + 1, pages.len()),
            width as f64 - 83.,
            layout::HEADER - 42.,
            10.,
            MUTED,
            false,
            64.,
        );
        for (local, ci) in range.clone().enumerate() {
            let col = &scene.layout.columns[ci];
            let x = layout::GAP + local as f64 * scene.layout.column_width + layout::LEFT;
            let track = 24. * scene.layout.options.pixels_per_lane;
            let bottom = scene.layout.y(col, col.start as f64);
            let top = scene.layout.y(col, col.end as f64);
            let rect = Rect::new(x as f32, top as f32, (x + track) as f32, bottom as f32);
            panel(c, rect, PANEL);
            fonts.draw(
                c,
                &format!("{:03}—{:03}", col.first_bar, col.last_bar),
                x,
                layout::HEADER - 5.,
                11.,
                FG,
                true,
                track as f32,
            );
            // Subtle alternating broad-lane bands; fine 24-lane divisions stay quiet.
            for i in [0, 2] {
                c.draw_rect(
                    Rect::from_xywh(
                        (x + i as f64 * 6. * scene.layout.options.pixels_per_lane) as f32,
                        top as f32,
                        (6. * scene.layout.options.pixels_per_lane) as f32,
                        (bottom - top) as f32,
                    ),
                    &paint(Color::from_argb(9, 152, 180, 213)),
                );
            }
            for &(a, b) in &scene.fever {
                let lo = a.max(col.start);
                let hi = b.min(col.end);
                if lo >= hi {
                    continue;
                }
                let y0 = scene.layout.y(col, hi as f64);
                let y1 = scene.layout.y(col, lo as f64);
                c.draw_rect(
                    Rect::new(x as f32, y0 as f32, (x + track) as f32, y1 as f32),
                    &paint(Color::from_argb(9, 255, 205, 118)),
                );
                stroke(
                    c,
                    (x - 3., y0),
                    (x - 3., y1),
                    Color::from_rgb(168, 143, 90),
                    2.,
                );
            }
            for lane in 0..=24 {
                let bold = lane % 6 == 0;
                stroke(
                    c,
                    (x + lane as f64 * scene.layout.options.pixels_per_lane, top),
                    (
                        x + lane as f64 * scene.layout.options.pixels_per_lane,
                        bottom,
                    ),
                    if bold {
                        Color::from_rgb(46, 60, 76)
                    } else {
                        Color::from_rgb(27, 38, 51)
                    },
                    if bold { 0.7 } else { 0.35 },
                );
            }
            for bar in scene
                .layout
                .bars
                .iter()
                .filter(|b| col.start <= b.tick && b.tick <= col.end)
            {
                let y = scene.layout.y(col, bar.tick as f64);
                stroke(
                    c,
                    (x, y),
                    (x + track, y),
                    Color::from_rgb(85, 103, 125),
                    0.9,
                );
                fonts.draw(
                    c,
                    &format!("{:03}", bar.number),
                    x - 26.,
                    y + 3.,
                    9.5,
                    MUTED,
                    false,
                    25.,
                );
            }
            for segment in scene.layout.bars.windows(2) {
                let mut tick = segment[0].tick + 240;
                while tick < segment[1].tick {
                    if tick >= col.start && tick < col.end {
                        let y = scene.layout.y(col, tick as f64);
                        stroke(
                            c,
                            (x, y),
                            (x + track, y),
                            if (tick - segment[0].tick) % 480 == 0 {
                                Color::from_rgb(40, 54, 70)
                            } else {
                                Color::from_rgb(28, 40, 53)
                            },
                            0.5,
                        );
                    }
                    tick += 240;
                }
            }
            c.save();
            c.clip_rect(rect, None, true);
            for guide in [true, false] {
                for r in scene.ribbons.iter().filter(|r| r.guide == guide) {
                    draw_ribbon(c, r, col, scene, skin, x);
                }
            }
            for &(a, b) in &scene.pairs {
                let a = ids[&a];
                let b = ids[&b];
                if scene.layout.owner(a.tick) == Some(ci) {
                    stroke(
                        c,
                        (
                            x + (a.left + a.right) / 2. * scene.layout.options.pixels_per_lane,
                            scene.layout.y(col, a.tick as f64),
                        ),
                        (
                            x + (b.left + b.right) / 2. * scene.layout.options.pixels_per_lane,
                            scene.layout.y(col, b.tick as f64),
                        ),
                        Color::from_argb(108, 182, 201, 222),
                        0.8,
                    );
                }
            }
            c.restore();
            c.save();
            c.clip_rect(
                Rect::new(
                    x as f32,
                    (top - layout::PAD) as f32,
                    (x + track) as f32,
                    (bottom + layout::PAD) as f32,
                ),
                None,
                true,
            );
            // Every body is drawn before any arrow, and marks are the final pass.
            for part in [NotePart::Body, NotePart::Arrow, NotePart::Mark] {
                for glyph in scene.glyphs.iter().filter(|g| g.column == ci) {
                    let n = &glyph.note;
                    if n.right <= 0. || n.left >= 24. {
                        continue;
                    }
                    let request = PreviewNote {
                        role: n.role().unwrap(),
                        width_lanes: n.width as f32,
                        center: (
                            (x + (n.left + n.right) / 2. * scene.layout.options.pixels_per_lane)
                                as f32,
                            scene.layout.y(col, n.tick as f64) as f32,
                        ),
                        lane_px: scene.layout.options.pixels_per_lane as f32,
                        body_height: scene.layout.options.note_height as f32,
                        arrow_height: scene.layout.options.arrow_height as f32,
                        critical: n.critical,
                        native_critical: scene.layout.options.native_critical,
                        strict_assets: scene.layout.options.strict_assets,
                    };
                    if skin.draw_preview_part(c, &request, part)? {
                        if scene.layout.options.strict_assets {
                            missing.insert(n.id);
                        } else {
                            fallbacks.insert(n.id);
                        }
                    }
                }
            }
            c.restore();
            let (annotations, details) = display_events(scene, ci);
            let desired: Vec<f64> = annotations
                .iter()
                .map(|a| scene.layout.y(col, a.tick as f64) + 3.)
                .collect();
            let (ys, overflow) = pack_labels(&desired, top - 12., bottom + 18., 12.);
            for (a, &y) in annotations.iter().zip(&ys) {
                let color = event_color(&a.kind);
                let anchor = scene.layout.y(col, a.tick as f64);
                stroke(c, (x + track, anchor), (x + track + 4., y - 3.), color, 0.6);
                let label = if let Some(bpm) = a.label.strip_suffix(" BPM") {
                    bpm.trim_end_matches('0').trim_end_matches('.').to_owned()
                } else {
                    a.label.clone()
                };
                let size = (9.5
                    * ((layout::RIGHT - 8.) as f32 / fonts.width(&label, 9.5, false).max(1.))
                        .min(1.))
                .max(7.);
                fonts.draw(
                    c,
                    &label,
                    x + track + 6.,
                    y,
                    size,
                    color,
                    false,
                    (layout::RIGHT - 8.) as f32,
                );
            }
            if overflow > 0 || !details.is_empty() {
                fonts.draw(
                    c,
                    "↳ appendix",
                    x + track + 4.,
                    bottom + 31.,
                    8.,
                    MUTED,
                    false,
                    55.,
                );
            }
            let time = scene
                .layout
                .bars
                .iter()
                .find(|b| b.tick == col.start)
                .map(|b| b.time_ms / 1000)
                .unwrap_or(0);
            fonts.draw(
                c,
                &format!("{}:{:02}  ↑", time / 60, time % 60),
                x,
                bottom + 20.,
                9.,
                MUTED,
                false,
                80.,
            );
            let end_time = scene
                .layout
                .bars
                .iter()
                .find(|b| b.tick == col.end)
                .map(|b| b.time_ms / 1000)
                .unwrap_or(0);
            fonts.draw(
                c,
                &format!("→  {}:{:02}", end_time / 60, end_time % 60),
                x + track - 70.,
                bottom + 20.,
                9.,
                MUTED,
                false,
                70.,
            );
        }
        for (i, label) in overflows.iter().enumerate() {
            let x = 20.
                + (i % appendix_columns) as f64 * (width as f64 - 40.) / appendix_columns as f64;
            let y = scene.layout.height as f64 + 15. + (i / appendix_columns) as f64 * 17.;
            fonts.draw(
                c,
                label,
                x,
                y,
                10.,
                MUTED,
                false,
                (width as f32 - 40.) / appendix_columns as f32 - 12.,
            );
        }
        fonts.draw(
            c,
            &format!(
                "moenotes bdon.moe  ·  {}  ·  upward / left to right",
                skin.manifest.skin
            ),
            20.,
            height as f64 - 15.,
            9.,
            MUTED,
            false,
            width as f32 - 40.,
        );
        let path = if pages.len() == 1 {
            output.to_path_buf()
        } else {
            output.with_file_name(format!(
                "{}-{:03}.png",
                output
                    .file_stem()
                    .context("Output filename")?
                    .to_string_lossy(),
                page + 1
            ))
        };
        let hi = surface.image_snapshot();
        let image = if ss == 1 {
            hi
        } else {
            let mut reduced =
                sk::surfaces::raster_n32_premul((width, height)).context("Resize allocation")?;
            reduced.canvas().draw_image_rect_with_sampling_options(
                &hi,
                None,
                Rect::from_wh(width as f32, height as f32),
                sk::SamplingOptions::from(sk::CubicResampler::mitchell()),
                &Paint::default(),
            );
            reduced.image_snapshot()
        };
        let bytes = image
            .encode(None, sk::EncodedImageFormat::PNG, 100)
            .context("PNG encoding")?;
        encoded_bytes = encoded_bytes
            .checked_add(bytes.len())
            .context("Encoded byte overflow")?;
        ensure!(
            encoded_bytes <= 256 * 1024 * 1024,
            "Encoded page set exceeds 256 MiB; render a smaller selection"
        );
        output_pages.push(Page {
            name: path.file_name().unwrap().to_string_lossy().into(),
            png: bytes.as_bytes().to_vec(),
        });
        images.push(ImageReport {
            file: path.file_name().unwrap().to_string_lossy().into(),
            width,
            height,
            sha256: sha256(bytes.as_bytes()),
            columns: [range.start, range.end],
        });
    }
    let mut warnings = scene.warnings.clone();
    if metadata
        .master_full_combo
        .is_some_and(|v| v != scene.statistics.reconstructed_full_combo as u64)
    {
        warnings.push(
            "Master FC differs from reconstructed parser count; both are preserved in report"
                .into(),
        );
    }
    if overlap_count(scene) > 0 {
        warnings.push("Some body boxes still overlap at the selected density limit; increase spacing or lower note height".into());
    }
    if !missing.is_empty() {
        warnings.push(format!(
            "{} source arrow slots empty (strict-assets)",
            missing.len()
        ));
    }
    if !fallbacks.is_empty() {
        warnings.push(format!(
            "{} missing source arrows rendered with explicit vector fallback",
            fallbacks.len()
        ));
    }
    Ok(Rendered {
        pages: output_pages,
        report: Report {
            schema_version: 3,
            parser_version: "0.3.0".into(),
            skin: skin.manifest.skin.clone(),
            mirror,
            metadata: metadata.clone(),
            source_notes: scene.source_notes,
            glyphs: scene.glyphs.len(),
            branches: scene.ribbons.len(),
            curve_points: scene.points,
            instantaneous_segments: scene.instantaneous_segments,
            hidden_nodes: scene.hidden_notes,
            zero_width_nodes: scene.zero_width,
            outside_viewport_glyphs: outside,
            bars: scene.layout.bars.len() - 1,
            columns: scene.layout.columns.len(),
            images,
            warnings,
            missing_arrow_note_ids: missing.into_iter().collect(),
            fallback_arrow_note_ids: fallbacks.into_iter().collect(),
            annotations: scene.annotations.clone(),
            layout: scene.layout.options.clone(),
            unused_vertical_fraction: scene.layout.unused_vertical_fraction,
            track_width_fraction: 24. * scene.layout.options.pixels_per_lane
                / scene.layout.column_width,
            dense_body_overlaps: overlap_count(scene),
            arrow_body_box_overlaps: decoration_overlap_count(scene).0,
            mark_body_box_overlaps: decoration_overlap_count(scene).1,
            annotation_overflow: overflow_count,
            statistics: scene.statistics.clone(),
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_dont_overlap() {
        let (y, n) = pack_labels(&[5., 5., 6., 19.], 0., 50., 12.);
        assert_eq!(n, 0);
        assert!(y.windows(2).all(|p| p[1] - p[0] >= 12.));
        assert!(y[0] >= 0. && y[3] <= 50.);
    }
    #[test]
    fn dense_bpm_keeps_range_and_every_original_label() -> Result<()> {
        let chart=br#"{"events":{"bpm":[{"t":0,"bpm":120},{"t":1,"bpm":121},{"t":2,"bpm":122},{"t":3,"bpm":123}]},"notes":[]}"#;
        let score = crate::parser::Score::parse(chart, false)?;
        let scene = Scene::build(
            &score,
            crate::layout::Layout::build(&score, crate::layout::Options::default())?,
        )?;
        let (shown, appendix) = display_events(&scene, 0);
        assert!(shown.iter().any(|a| a.label == "120→123"));
        assert_eq!(appendix.len(), 4);
        assert!(appendix[0].contains("120.00 BPM"));
        assert!(appendix[3].contains("123.00 BPM"));
        Ok(())
    }
}
