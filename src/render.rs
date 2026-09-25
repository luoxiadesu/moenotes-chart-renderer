//! Page-oriented Skia presentation. Geometry is already resolved in Scene.
use crate::{
    Skin,
    layout::{self, Column, FlickLayout, Theme},
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
mod sheet;

const BG: Color = Color::from_rgb(13, 20, 29);
const PANEL: Color = Color::from_rgb(20, 29, 40);
const FG: Color = Color::from_rgb(228, 237, 246);
const MUTED: Color = Color::from_rgb(137, 157, 177);
const GOLD: Color = Color::from_rgb(240, 204, 119);
struct Palette {
    background: Color,
    panel: Color,
    text: Color,
    muted: Color,
    rule: Color,
    lane: Color,
    fine_lane: Color,
    bar: Color,
    beat: Color,
    subdivision: Color,
}
impl Palette {
    fn new(theme: Theme) -> Self {
        match theme {
            Theme::Print => Self {
                background: Color::WHITE,
                panel: Color::WHITE,
                text: Color::from_rgb(30, 41, 55),
                muted: Color::from_rgb(80, 92, 106),
                rule: Color::from_rgb(192, 200, 210),
                lane: Color::from_rgb(207, 214, 223),
                fine_lane: Color::from_rgb(238, 240, 244),
                bar: Color::from_rgb(133, 148, 166),
                beat: Color::from_rgb(217, 223, 231),
                subdivision: Color::from_rgb(237, 240, 244),
            },
            Theme::Dark => Self {
                background: BG,
                panel: PANEL,
                text: FG,
                muted: MUTED,
                rule: Color::from_rgb(42, 55, 69),
                lane: Color::from_rgb(46, 60, 76),
                fine_lane: Color::from_rgb(27, 38, 51),
                bar: Color::from_rgb(85, 103, 125),
                beat: Color::from_rgb(40, 54, 70),
                subdivision: Color::from_rgb(28, 40, 53),
            },
            Theme::Black => Self {
                background: Color::from_rgb(8, 9, 12),
                panel: Color::from_rgb(16, 18, 23),
                text: Color::from_rgb(243, 244, 247),
                muted: Color::from_rgb(176, 182, 194),
                rule: Color::from_rgb(61, 65, 76),
                lane: Color::from_rgb(53, 57, 68),
                fine_lane: Color::from_rgb(27, 30, 38),
                bar: Color::from_rgb(110, 119, 139),
                beat: Color::from_rgb(48, 53, 64),
                subdivision: Color::from_rgb(33, 37, 45),
            },
        }
    }
}
fn print_note_color(role: &str) -> Color {
    match role {
        "tap" => Color::from_rgb(22, 100, 129),
        "trace" => Color::from_rgb(111, 67, 158),
        "flick" => Color::from_rgb(155, 94, 8),
        "flick_left" => Color::from_rgb(21, 118, 80),
        "flick_right" => Color::from_rgb(172, 52, 91),
        _ => Color::from_rgb(62, 83, 168),
    }
}
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
    pub logical_width: i32,
    pub logical_height: i32,
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
    /// Subset of body overlaps between structural anchors 1–2 ticks apart.
    pub structural_connection_overlaps: usize,
    pub arrow_body_box_overlaps: usize,
    /// Collisions before dense arrows were moved to their explicit side rail.
    pub inline_arrow_body_box_overlaps: usize,
    pub flick_callouts: Vec<FlickCallout>,
    pub unresolved_flick_note_ids: Vec<i32>,
    pub flick_rail_width: f64,
    /// Vertical presentation offset added to logical Scene coordinates.
    pub chart_offset_y: f64,
    pub mark_body_box_overlaps: usize,
    pub annotation_overflow: usize,
    pub statistics: crate::parser::Statistics,
}
#[derive(Debug, Clone, Serialize)]
pub struct FlickCallout {
    pub note_id: i32,
    pub column: usize,
    /// Center relative to the track's right edge; y is the actual note y.
    pub offset_x: f64,
    pub y: f64,
}
fn arrow_body_collision(
    scene: &Scene,
    skin: &Skin,
    a: &crate::scene::Glyph,
    b: &crate::scene::Glyph,
) -> bool {
    if a.column != b.column || !a.note.role().is_some_and(|r| r.starts_with("flick")) {
        return false;
    }
    let delta = (b.note.tick - a.note.tick) as f64 * scene.layout.options.pixels_per_beat / 480.;
    let own = rendered_body_height(scene, skin, &a.note);
    let other = rendered_body_height(scene, skin, &b.note);
    let half = if scene.layout.options.theme == Theme::Print && skin.manifest.skin == "builtin" {
        (a.note.width * scene.layout.options.pixels_per_lane - 2.).clamp(2., 22.)
            / scene.layout.options.pixels_per_lane
            / 2.
    } else {
        a.note.width / 2.
    };
    let center = (a.note.left + a.note.right) / 2.;
    delta > own / 2. + 1.5 - other / 2.
        && delta < own / 2. + 1.5 + scene.layout.options.arrow_height + other / 2.
        && b.note.left < center + half
        && b.note.right > center - half
}
/// Interval coloring keeps full-size arrows off note bodies without changing
/// ticks or lane geometry. Extra rail width is explicit in the output report.
fn flick_callouts(scene: &Scene, skin: &Skin) -> (Vec<FlickCallout>, Vec<i32>, f64) {
    if scene.layout.options.flick_layout == FlickLayout::Inline || skin.manifest.skin != "builtin" {
        return (vec![], vec![], 0.);
    }
    let mut callouts = vec![];
    let mut unresolved = vec![];
    let mut maximum = 0;
    for ci in 0..scene.layout.columns.len() {
        let mut ends: Vec<f64> = vec![];
        let glyphs: Vec<_> = scene.glyphs.iter().filter(|g| g.column == ci).collect();
        for (i, a) in glyphs.iter().enumerate() {
            if !a.note.role().is_some_and(|role| role.starts_with("flick")) {
                continue;
            }
            let collision = glyphs
                .iter()
                .skip(i + 1)
                .take_while(|b| {
                    (b.note.tick - a.note.tick) as f64 * scene.layout.options.pixels_per_beat / 480.
                        <= scene.layout.options.note_height + scene.layout.options.arrow_height + 4.
                })
                .any(|b| arrow_body_collision(scene, skin, a, b));
            if !collision {
                continue;
            }
            let position = a.note.tick as f64 * scene.layout.options.pixels_per_beat / 480.;
            let height = scene.layout.options.arrow_height + 4.;
            let slot = ends
                .iter()
                .position(|end| *end <= position - height / 2.)
                .unwrap_or(ends.len());
            if slot >= 16 {
                unresolved.push(a.note.id);
                continue;
            }
            if slot == ends.len() {
                ends.push(0.);
            }
            ends[slot] = position + height / 2.;
            maximum = maximum.max(slot + 1);
            callouts.push(FlickCallout {
                note_id: a.note.id,
                column: ci,
                offset_x: 20. + slot as f64 * 32.,
                y: scene
                    .layout
                    .y(&scene.layout.columns[ci], a.note.tick as f64),
            });
        }
    }
    (
        callouts,
        unresolved,
        maximum as f64 * 32. + if maximum > 0 { 8. } else { 0. },
    )
}
fn draw_callout(c: &Canvas, note: &crate::parser::Note, x: f64, y: f64, height: f64, theme: Theme) {
    let color = if theme == Theme::Print {
        print_note_color(note.role().unwrap())
    } else {
        match note.direction {
            1 => Color::from_rgb(108, 225, 172),
            2 => Color::from_rgb(255, 159, 192),
            _ => GOLD,
        }
    };
    let mut path = sk::PathBuilder::new();
    let (x, y) = (x as f32, y as f32);
    let h = height as f32 / 2. - 0.8;
    if note.direction == 0 {
        path.move_to((x - 10., y + h));
        path.line_to((x, y - h));
        path.line_to((x + 10., y + h));
    } else {
        let sign = if note.direction == 1 { -1. } else { 1. };
        for offset in [-5., 5.] {
            path.move_to((x + offset - sign * 3., y - h));
            path.line_to((x + offset + sign * 3., y));
            path.line_to((x + offset - sign * 3., y + h));
        }
    }
    let mut p = paint(color);
    p.set_style(sk::paint::Style::Stroke).set_stroke_width(1.6);
    c.draw_path(&path.detach(), &p);
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
/// Original print artwork uses outlines and shape as well as color. External
/// packs retain their sprites, with an underlay to separate pale bodies from paper.
fn draw_print_note(
    c: &Canvas,
    skin: &Skin,
    request: &PreviewNote<'_>,
    part: NotePart,
) -> Result<bool> {
    let n = request;
    let color = print_note_color(n.role);
    let (x, y) = n.center;
    let width = n.width_lanes * n.lane_px;
    let body = layout::body_height(n.role, n.body_height as f64) as f32;
    if skin.manifest.skin != "builtin" {
        if matches!(part, NotePart::Body) {
            let mut outline = paint(color);
            outline
                .set_style(sk::paint::Style::Stroke)
                .set_stroke_width(1.1);
            c.draw_round_rect(
                Rect::from_xywh(x - width / 2., y - body / 2., width, body),
                1.,
                1.,
                &outline,
            );
        }
        return skin.draw_preview_part(c, n, part);
    }
    match part {
        NotePart::Body => {
            let rect = Rect::from_xywh(
                x - width / 2. + 0.5,
                y - body / 2.,
                (width - 1.).max(0.5),
                body,
            );
            let mut fill = paint(color);
            fill.set_alpha_f(if n.role == "connection" { 0.12 } else { 0.22 });
            c.draw_round_rect(rect, 1., 1., &paint(Color::WHITE));
            c.draw_round_rect(rect, 1., 1., &fill);
            let mut edge = paint(color);
            edge.set_style(sk::paint::Style::Stroke)
                .set_stroke_width(1.1);
            c.draw_round_rect(rect, 1., 1., &edge);
            if n.role == "trace" {
                stroke(
                    c,
                    ((x - width / 2. + 2.) as f64, y as f64),
                    ((x + width / 2. - 2.) as f64, y as f64),
                    color,
                    0.8,
                );
            } else if n.role != "connection" {
                stroke(
                    c,
                    ((x - width / 2. + 1.) as f64, (y - body / 2. + 1.) as f64),
                    ((x + width / 2. - 1.) as f64, (y - body / 2. + 1.) as f64),
                    color,
                    1.4,
                );
            }
        }
        NotePart::Arrow if n.role.starts_with("flick") => {
            let cy = y - body / 2. - 1.5 - n.arrow_height / 2.;
            let h = n.arrow_height / 2. - 0.8;
            let w = (width - 2.).clamp(2., 22.);
            let mut path = sk::PathBuilder::new();
            if n.role == "flick" {
                path.move_to((x - w / 2., cy + h));
                path.line_to((x, cy - h));
                path.line_to((x + w / 2., cy + h));
            } else {
                let sign = if n.role == "flick_left" { -1. } else { 1. };
                for offset in [-w * 0.24, w * 0.24] {
                    path.move_to((x + offset - sign * w * 0.16, cy - h));
                    path.line_to((x + offset + sign * w * 0.16, cy));
                    path.line_to((x + offset - sign * w * 0.16, cy + h));
                }
            }
            let mut edge = paint(color);
            edge.set_style(sk::paint::Style::Stroke)
                .set_stroke_width(1.6);
            c.draw_path(&path.detach(), &edge);
        }
        NotePart::Mark if n.critical => {
            if n.native_critical {
                return skin.draw_preview_part(c, n, part);
            }
            let mut path = sk::PathBuilder::new();
            path.move_to((x, y - 2.6));
            path.line_to((x + 2.6, y));
            path.line_to((x, y + 2.6));
            path.line_to((x - 2.6, y));
            path.close();
            let path = path.detach();
            let mut halo = paint(Color::WHITE);
            halo.set_style(sk::paint::Style::Stroke)
                .set_stroke_width(1.4);
            c.draw_path(&path, &halo);
            c.draw_path(&path, &paint(Color::from_rgb(129, 78, 9)));
        }
        _ => {}
    }
    Ok(false)
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
    let print = scene.layout.options.theme == Theme::Print;
    let g = &skin.manifest.line.normal;
    let base = if print {
        if r.guide {
            Color::from_rgb(112, 83, 153)
        } else {
            print_note_color("slide")
        }
    } else if r.guide {
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
        if print {
            p.set_alpha_f(if r.guide { 0.07 } else { 0.14 });
        } else if r.guide {
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
        if !r.guide && !print {
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
            .set_alpha_f(if r.guide { 0.50 } else { 0.80 });
        if !print {
            edge.set_alpha_f(if r.guide { 0.36 } else { 0.70 });
        } else if r.guide {
            edge.set_path_effect(sk::PathEffect::dash(&[3., 3.], 0.));
        }
        c.draw_path(&left, &edge);
        c.draw_path(&right, &edge);
    }
}
fn event_color(kind: &str, theme: Theme) -> Color {
    if theme == Theme::Print {
        return match kind {
            "bpm" => Color::from_rgb(142, 81, 15),
            "meter" => Color::from_rgb(87, 72, 144),
            "skill" => Color::from_rgb(155, 47, 102),
            _ => Color::from_rgb(15, 110, 91),
        };
    }
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
                details.push(if scene.layout.options.theme == Theme::Dark {
                    format!("@{}  {}", a.tick, a.label)
                } else {
                    format!("{}   {}", sheet::location(scene, a.tick), a.label)
                });
            }
        } else {
            shown.extend_from_slice(&bpm[i..end]);
        }
        i = end;
    }
    for event in &mut shown {
        if event.kind == "call" && event.label.len() > 12 {
            details.push(if scene.layout.options.theme == Theme::Dark {
                format!("@{}  {}", event.tick, event.label)
            } else {
                format!("{}   {}", sheet::location(scene, event.tick), event.label)
            });
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
fn rendered_body_height(scene: &Scene, skin: &Skin, note: &crate::parser::Note) -> f64 {
    let role = note.role().unwrap();
    if scene.layout.options.theme == Theme::Print && skin.manifest.skin == "builtin" {
        return layout::body_height(role, scene.layout.options.note_height);
    }
    let main = &skin.manifest.sprites[&skin.manifest.notes[role].main];
    let tap = &skin.manifest.sprites[&skin.manifest.notes["tap"].main];
    (main.rect.height / main.pixels_per_unit / (tap.rect.height / tap.pixels_per_unit)) as f64
        * scene.layout.options.note_height
}
fn overlap_count(scene: &Scene, skin: &Skin) -> (usize, usize) {
    let mut n = 0;
    let mut structural = 0;
    let glyphs = &scene.glyphs;
    let heights: Vec<_> = glyphs
        .iter()
        .map(|g| rendered_body_height(scene, skin, &g.note))
        .collect();
    let max_height = heights.iter().copied().fold(0., f64::max);
    for (i, a) in glyphs.iter().enumerate() {
        for (j, b) in glyphs.iter().enumerate().skip(i + 1) {
            let dt =
                (b.note.tick - a.note.tick) as f64 / 480. * scene.layout.options.pixels_per_beat;
            if dt > max_height {
                break;
            }
            if dt > 0.
                && a.column == b.column
                && a.note.left < b.note.right
                && b.note.left < a.note.right
                && dt < (heights[i] + heights[j]) / 2.
            {
                n += 1;
                if a.note.role() == Some("connection")
                    && b.note.role() == Some("connection")
                    && b.note.tick - a.note.tick <= 2
                {
                    structural += 1;
                }
            }
        }
    }
    (n, structural)
}
fn decoration_overlap_count(scene: &Scene, skin: &Skin, moved: &BTreeSet<i32>) -> (usize, usize) {
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
            if !moved.contains(&a.note.id) && arrow_body_collision(scene, skin, a, b) {
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
    let theme = scene.layout.options.theme;
    let palette = Palette::new(theme);
    let print = theme == Theme::Print;
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
    let (callouts, unresolved_callouts, rail_width) = flick_callouts(scene, skin);
    let callout_ids: BTreeSet<_> = callouts.iter().map(|v| v.note_id).collect();
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
    let mut header_shift = 0.;
    for (page, range) in pages.iter().enumerate() {
        let base_width = scene.layout.page_width(range.len())?;
        let width = (base_width as f64 + rail_width * range.len() as f64).ceil();
        ensure!(
            width <= i32::MAX as f64,
            "Flick rails exceed logical coordinate range"
        );
        let width = width as i32;
        let modern = theme != Theme::Dark;
        let header = sheet::Header::plan(width, fonts, metadata, cover.is_some());
        header_shift = if modern {
            header.height - layout::HEADER
        } else {
            0.
        };
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
            overflows.extend(events.into_iter().skip(cap).map(|a| {
                if modern {
                    format!("{}   {}", sheet::location(scene, a.tick), a.label)
                } else {
                    format!("@{}  {}", a.tick, a.label)
                }
            }));
        }
        let appendix_columns = if modern {
            ((width as usize).saturating_sub(48) / 520).clamp(1, 4)
        } else {
            ((width as usize).saturating_sub(40) / 220).clamp(1, 3)
        };
        let overflow_count_before_wrap = overflows.len();
        if modern {
            let max = (width as f32 - 48. * header.scale as f32) / appendix_columns as f32 - 16.;
            overflows = overflows
                .iter()
                .flat_map(|line| fonts.lines(line, 12. * header.scale as f32, false, max, 256))
                .collect();
        }
        let appendix_rows = overflows.len().div_ceil(appendix_columns);
        let appendix_step = if modern { 22. * header.scale } else { 17. };
        let appendix_height = (appendix_rows as f64 * appendix_step
            + if overflows.is_empty() {
                0.
            } else if modern {
                58. * header.scale
            } else {
                36.
            })
        .ceil();
        let footer_height = if modern {
            84. * header.scale
        } else {
            layout::FOOTER
        };
        let height = scene.layout.height as f64 + header_shift - layout::FOOTER
            + footer_height
            + appendix_height;
        ensure!(height <= i32::MAX as f64, "Logical image height overflow");
        let height = height.ceil() as i32;
        overflow_count += overflow_count_before_wrap;
        let ss = scene.layout.options.supersample as i32;
        let scale = scene.layout.options.output_scale;
        let (export_width, export_height) = scene
            .layout
            .options
            .export_dimensions(width as f64, height as f64)?;
        let mut surface = sk::surfaces::raster_n32_premul((export_width * ss, export_height * ss))
            .context("Cannot allocate page")?;
        let c = surface.canvas();
        c.scale(((ss as f64 * scale) as f32, (ss as f64 * scale) as f32));
        c.clear(palette.background);
        if modern {
            header.draw(
                c,
                fonts,
                skin,
                &palette,
                scene,
                metadata,
                cover.as_ref(),
                width,
                mirror,
            )?;
        } else {
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
                palette.text,
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
                palette.muted,
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
            fonts.draw(
                c,
                &fc,
                (width as f64 - 205.).max(20.),
                101.,
                10.,
                palette.muted,
                false,
                (width as f32 - 40.).min(185.),
            );
            if !metadata.artist.is_empty() {
                fonts.draw(
                    c,
                    &metadata.artist,
                    text_x,
                    if compact { 64. } else { 84. },
                    10.5,
                    palette.muted,
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
                    palette.muted,
                    false,
                    width as f32 - 40.,
                );
            }
            // Compact visual legend instead of debug implementation text.
            let mut ly = if print { 131. } else { 141. };
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
                let color = if print {
                    match label {
                        "TAP" => print_note_color("tap"),
                        "SLIDE" => print_note_color("slide"),
                        "FLICK" => print_note_color("flick"),
                        "TRACE" => print_note_color("trace"),
                        _ => Color::from_rgb(129, 78, 9),
                    }
                } else {
                    color
                };
                if lx + 80. > width as f64 {
                    if print {
                        lx = 20.;
                        ly += 14.;
                    } else {
                        break;
                    }
                }
                c.draw_circle((lx as f32 + 3., ly as f32 - 3.), 2.4, &paint(color));
                fonts.draw(c, label, lx + 11., ly, 9., palette.muted, false, 80.);
                lx += if label == "CRITICAL" { 83. } else { 66. };
            }
            stroke(
                c,
                (20., layout::HEADER - 31.),
                (width as f64 - 20., layout::HEADER - 31.),
                palette.rule,
                0.8,
            );
            if !print {
                fonts.draw(
                    c,
                    &format!("{:02} / {:02}", page + 1, pages.len()),
                    width as f64 - 83.,
                    layout::HEADER - 42.,
                    10.,
                    palette.muted,
                    false,
                    64.,
                );
            }
        }
        c.save();
        c.translate((0., header_shift as f32));
        for (local, ci) in range.clone().enumerate() {
            let col = &scene.layout.columns[ci];
            let used_width =
                range.len() as f64 * (scene.layout.column_width + rail_width) + layout::GAP;
            let x = (width as f64 - used_width).max(0.) / 2.
                + layout::GAP
                + local as f64 * (scene.layout.column_width + rail_width)
                + layout::LEFT;
            let track = 24. * scene.layout.options.pixels_per_lane;
            let event_x = x + track + rail_width;
            let bottom = scene.layout.y(col, col.start as f64);
            let top = scene.layout.y(col, col.end as f64);
            let rect = Rect::new(x as f32, top as f32, (x + track) as f32, bottom as f32);
            panel(c, rect, palette.panel);
            fonts.draw(
                c,
                &format!("{:03}—{:03}", col.first_bar, col.last_bar),
                x,
                if modern {
                    top - layout::PAD - 10.
                } else {
                    layout::HEADER - 5.
                },
                if modern { 14. } else { 11. },
                palette.text,
                true,
                (scene.layout.column_width - layout::LEFT - layout::GAP) as f32,
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
                    &paint(if print {
                        Color::from_rgb(248, 249, 251)
                    } else {
                        Color::from_argb(9, 152, 180, 213)
                    }),
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
                    &paint(if print {
                        Color::from_argb(12, 202, 155, 60)
                    } else {
                        Color::from_argb(9, 255, 205, 118)
                    }),
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
                        palette.lane
                    } else {
                        palette.fine_lane
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
                let major = modern && (bar.number == col.first_bar || bar.number % 4 == 1);
                stroke(
                    c,
                    (x, y),
                    (x + track, y),
                    palette.bar,
                    if major {
                        1.15
                    } else if modern {
                        0.8
                    } else {
                        0.9
                    },
                );
                fonts.draw(
                    c,
                    &format!("{:03}", bar.number),
                    x - 26.,
                    y + 3.,
                    if modern { 10.5 } else { 9.5 },
                    palette.muted,
                    major,
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
                                palette.beat
                            } else {
                                palette.subdivision
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
                        if print {
                            Color::from_argb(140, 98, 112, 130)
                        } else {
                            Color::from_argb(108, 182, 201, 222)
                        },
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
                    if part == NotePart::Arrow && callout_ids.contains(&n.id) {
                        continue;
                    }
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
                    let missing_arrow = if print {
                        draw_print_note(c, skin, &request, part)?
                    } else {
                        skin.draw_preview_part(c, &request, part)?
                    };
                    if missing_arrow {
                        if scene.layout.options.strict_assets {
                            missing.insert(n.id);
                        } else {
                            fallbacks.insert(n.id);
                        }
                    }
                }
            }
            c.restore();
            for callout in callouts.iter().filter(|v| v.column == ci) {
                let note = ids[&callout.note_id];
                let end_x = x + track + callout.offset_x;
                let start_x = x + note.right.min(24.) * scene.layout.options.pixels_per_lane;
                let mut leader = paint(palette.muted);
                leader.set_stroke_width(0.7);
                leader.set_path_effect(sk::PathEffect::dash(&[2., 2.], 0.));
                c.draw_line(
                    (start_x as f32, callout.y as f32),
                    ((end_x - 13.) as f32, callout.y as f32),
                    &leader,
                );
                c.draw_circle(
                    (start_x as f32, callout.y as f32),
                    1.4,
                    &paint(palette.muted),
                );
                draw_callout(
                    c,
                    note,
                    end_x,
                    callout.y,
                    scene.layout.options.arrow_height,
                    theme,
                );
            }
            let (annotations, details) = display_events(scene, ci);
            let desired: Vec<f64> = annotations
                .iter()
                .map(|a| scene.layout.y(col, a.tick as f64) + 3.)
                .collect();
            let (ys, overflow) = pack_labels(&desired, top - 12., bottom + 18., 12.);
            for (a, &y) in annotations.iter().zip(&ys) {
                let color = event_color(&a.kind, theme);
                let anchor = scene.layout.y(col, a.tick as f64);
                stroke(c, (x + track, anchor), (event_x + 4., y - 3.), color, 0.6);
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
                    event_x + 6.,
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
                    if modern { "↳ NOTES" } else { "↳ appendix" },
                    event_x + 4.,
                    bottom + 31.,
                    8.,
                    palette.muted,
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
                palette.muted,
                false,
                (track as f32).min(80.),
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
                if track < 140. { x } else { x + track - 70. },
                bottom + if track < 140. { 34. } else { 20. },
                9.,
                palette.muted,
                false,
                (track as f32).min(70.),
            );
        }
        c.restore();
        let notes_top = scene.layout.height as f64 + header_shift - layout::FOOTER + 24.;
        if modern && !overflows.is_empty() {
            fonts.draw(
                c,
                "CHART NOTES",
                24. * header.scale,
                notes_top + 17. * header.scale,
                13. * header.scale as f32,
                palette.text,
                true,
                width as f32 - 48.,
            );
        }
        for (i, label) in overflows.iter().enumerate() {
            let x = if modern { 24. * header.scale } else { 20. }
                + (i % appendix_columns) as f64
                    * (width as f64 - if modern { 48. * header.scale } else { 40. })
                    / appendix_columns as f64;
            let y = if modern {
                notes_top + 46. * header.scale + (i / appendix_columns) as f64 * appendix_step
            } else {
                scene.layout.height as f64 + 15. + (i / appendix_columns) as f64 * 17.
            };
            fonts.draw(
                c,
                label,
                x,
                y,
                if modern {
                    12. * header.scale as f32
                } else {
                    10.
                },
                palette.muted,
                false,
                if modern {
                    (width as f32 - 48. * header.scale as f32) / appendix_columns as f32 - 16.
                } else {
                    (width as f32 - 40.) / appendix_columns as f32 - 12.
                },
            );
        }
        if modern {
            sheet::footer(c, fonts, &palette, width, height, header.scale);
        } else {
            fonts.draw(
                c,
                &format!(
                    "moenotes bdon.moe  ·  {}  ·  upward / left to right",
                    skin.manifest.skin
                ),
                20.,
                height as f64 - 15.,
                9.,
                palette.muted,
                false,
                width as f32 - 40.,
            );
        }
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
            let mut reduced = sk::surfaces::raster_n32_premul((export_width, export_height))
                .context("Resize allocation")?;
            reduced.canvas().draw_image_rect_with_sampling_options(
                &hi,
                None,
                Rect::from_wh(export_width as f32, export_height as f32),
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
            width: export_width,
            height: export_height,
            logical_width: width,
            logical_height: height,
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
    let (body_overlaps, structural_overlaps) = overlap_count(scene, skin);
    let (inline_arrow_overlaps, _) = decoration_overlap_count(scene, skin, &BTreeSet::new());
    let (arrow_overlaps, mark_overlaps) = decoration_overlap_count(scene, skin, &callout_ids);
    if !callouts.is_empty() {
        warnings.push(format!(
            "{} dense Flick arrows shown in side rails with leaders; note coordinates unchanged",
            callouts.len()
        ));
    }
    if body_overlaps > structural_overlaps {
        warnings.push("Some body boxes still overlap at the selected density limit; increase spacing or lower note height".into());
    }
    if structural_overlaps > 0 {
        warnings.push(format!("{structural_overlaps} near-coincident structural connection pairs retain their exact tick positions; these do not drive automatic spacing"));
    }
    if arrow_overlaps > 0 {
        warnings.push("Some arrow/body boxes intersect at the selected spacing; see arrow_body_box_overlaps (conservative for external skins)".into());
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
            dense_body_overlaps: body_overlaps,
            structural_connection_overlaps: structural_overlaps,
            arrow_body_box_overlaps: arrow_overlaps,
            inline_arrow_body_box_overlaps: inline_arrow_overlaps,
            flick_callouts: callouts
                .into_iter()
                .map(|mut item| {
                    item.y += header_shift;
                    item
                })
                .collect(),
            unresolved_flick_note_ids: unresolved_callouts,
            flick_rail_width: rail_width,
            chart_offset_y: header_shift,
            mark_body_box_overlaps: mark_overlaps,
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
