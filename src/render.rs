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
/// White-sheet body fills follow the built-in screen hues, darker outlines
/// stay in [`print_note_color`]. Tap and slide differ in hue and lightness.
fn print_note_fill(role: &str) -> Color {
    match role {
        "tap" => Color::from_rgb(126, 206, 232),
        "trace" => Color::from_rgb(208, 188, 245),
        "flick" => Color::from_rgb(246, 200, 110),
        "flick_left" => Color::from_rgb(132, 215, 170),
        "flick_right" => Color::from_rgb(245, 162, 188),
        "connection" => Color::from_rgb(214, 219, 248),
        _ => Color::from_rgb(150, 162, 240),
    }
}
fn black_arrow_color(role: &str) -> Color {
    match role {
        "flick_left" => Color::from_rgb(108, 225, 172),
        "flick_right" => Color::from_rgb(255, 159, 192),
        _ => GOLD,
    }
}
fn mix(a: Color, b: Color, t: f32) -> Color {
    let channel = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::from_rgb(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
    )
}
const PRINT_FEVER: Color = Color::from_rgb(251, 245, 231);
const BLACK_FEVER: Color = Color::from_rgb(28, 25, 19);
const FEVER_RAIL: Color = Color::from_rgb(168, 143, 90);
/// Half height of the default critical diamond. Legacy dark keeps its mark.
fn critical_mark_half(theme: Theme, note_height: f64) -> f64 {
    if theme == Theme::Dark {
        2.3
    } else {
        (note_height * 0.6).max(3.)
    }
}
/// Default white/black critical mark: a gold diamond sized to the body, with
/// an outline that also separates it from pale or bright note fills.
fn draw_critical_diamond(c: &Canvas, n: &PreviewNote<'_>, theme: Theme) {
    let (x, y) = n.center;
    let hd = critical_mark_half(theme, n.body_height as f64) as f32;
    let hw = (hd * 1.2).min(n.width_lanes * n.lane_px * 0.3).max(2.5);
    let mut path = sk::PathBuilder::new();
    path.move_to((x, y - hd));
    path.line_to((x + hw, y));
    path.line_to((x, y + hd));
    path.line_to((x - hw, y));
    path.close();
    let path = path.detach();
    let (fill, edge) = if theme == Theme::Print {
        let mut halo = paint(Color::WHITE);
        halo.set_style(sk::paint::Style::Stroke)
            .set_stroke_width(2.4);
        c.draw_path(&path, &halo);
        (Color::from_rgb(240, 182, 36), Color::from_rgb(116, 70, 6))
    } else {
        (Color::from_rgb(255, 214, 102), Color::from_rgb(58, 38, 4))
    };
    c.draw_path(&path, &paint(fill));
    let mut outline = paint(edge);
    outline
        .set_style(sk::paint::Style::Stroke)
        .set_stroke_width(0.9);
    c.draw_path(&path, &outline);
}
/// Vector Flick direction inside the unchanged arrow box (`w` x `height`):
/// one chevron up, or one/two side chevrons depending on available width.
fn draw_flick_arrow(c: &Canvas, role: &str, center: (f32, f32), w: f32, height: f32, color: Color) {
    let (x, cy) = center;
    let stroke_width = (height * 0.24).clamp(1.6, 2.8);
    let h = (height / 2. - stroke_width / 2. - 0.2).max(1.);
    let mut path = sk::PathBuilder::new();
    if role == "flick" {
        let half = (w / 2. - stroke_width / 2.).max(1.);
        path.move_to((x - half, cy + h));
        path.line_to((x, cy - h));
        path.line_to((x + half, cy + h));
    } else {
        let sign = if role == "flick_left" { -1. } else { 1. };
        let (offsets, span): (&[f32], f32) = if w < 14. {
            (&[0.], w * 0.26)
        } else {
            (&[-w * 0.22, w * 0.22], w * 0.15)
        };
        for offset in offsets {
            path.move_to((x + offset - sign * span, cy - h));
            path.line_to((x + offset + sign * span, cy));
            path.line_to((x + offset - sign * span, cy + h));
        }
    }
    let mut p = paint(color);
    p.set_style(sk::paint::Style::Stroke)
        .set_stroke_width(stroke_width)
        .set_stroke_cap(sk::paint::Cap::Round)
        .set_stroke_join(sk::paint::Join::Round);
    c.draw_path(&path.detach(), &p);
}
/// One entry point for chart notes and the legend. Legacy dark delegates to
/// the skin unchanged; white/black add their shared presentation layer.
fn draw_note_part(
    c: &Canvas,
    skin: &Skin,
    request: &PreviewNote<'_>,
    part: NotePart,
    theme: Theme,
) -> Result<bool> {
    let modern_critical = request.critical && !request.native_critical;
    match theme {
        Theme::Print => draw_print_note(c, skin, request, part),
        Theme::Dark => skin.draw_preview_part(c, request, part),
        Theme::Black => match part {
            NotePart::Body => {
                if modern_critical {
                    let (x, y) = request.center;
                    let width = request.width_lanes * request.lane_px;
                    let body = request.body_height * 0.8;
                    let mut glow = paint(Color::from_rgb(255, 200, 80));
                    glow.set_alpha_f(0.55);
                    glow.set_mask_filter(sk::MaskFilter::blur(sk::BlurStyle::Normal, 2.2, None));
                    c.draw_round_rect(
                        Rect::from_xywh(x - width / 2., y - body / 2., width, body),
                        2.,
                        2.,
                        &glow,
                    );
                }
                skin.draw_preview_part(c, request, part)
            }
            NotePart::Arrow
                if skin.manifest.skin == "builtin" && request.role.starts_with("flick") =>
            {
                let (x, y) = request.center;
                let width = request.width_lanes * request.lane_px;
                let cy = y - request.body_height / 2. - 1.5 - request.arrow_height / 2.;
                draw_flick_arrow(
                    c,
                    request.role,
                    (x, cy),
                    (width - 2.).clamp(2., 22.),
                    request.arrow_height,
                    black_arrow_color(request.role),
                );
                Ok(false)
            }
            NotePart::Mark if modern_critical => {
                skin.draw_preview_part(
                    c,
                    &PreviewNote {
                        critical: false,
                        ..*request
                    },
                    part,
                )?;
                draw_critical_diamond(c, request, theme);
                Ok(false)
            }
            _ => skin.draw_preview_part(c, request, part),
        },
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
    /// Chart text/gutter multiplier (1 for legacy dark). Schema v4.
    pub chart_text_scale: f64,
    /// True when column tops share one line (white/black). Schema v4.
    pub columns_top_aligned: bool,
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
    let role = note.role().unwrap();
    if theme != Theme::Dark {
        let color = if theme == Theme::Print {
            print_note_color(role)
        } else {
            black_arrow_color(role)
        };
        draw_flick_arrow(c, role, (x as f32, y as f32), 20., height as f32, color);
        return;
    }
    let color = black_arrow_color(role);
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
/// Axis-aligned grid rules placed on whole output pixels. `origin` is the
/// logical offset of the current canvas translation; `scale` is the export
/// scale. Rules are `px` output pixels wide with `alpha` standing in for the
/// sub-pixel coverage a thinner antialiased line used to have, so the grid
/// keeps its weight hierarchy without smearing across two pixel rows.
#[derive(Clone, Copy)]
struct Crisp {
    origin: (f64, f64),
    scale: f64,
}
impl Crisp {
    /// Snap a logical coordinate to the nearest output-pixel boundary.
    fn snap(&self, v: f64, origin: f64) -> f64 {
        ((v + origin) * self.scale).round() / self.scale - origin
    }
    fn rule(&self, v: f64, origin: f64, px: u32) -> (f64, f32) {
        let edge = self.snap(v - px as f64 / self.scale / 2., origin);
        (
            edge + px as f64 / self.scale / 2.,
            (px as f64 / self.scale) as f32,
        )
    }
    fn tinted(color: Color, alpha: f32) -> Color {
        color.with_a((color.a() as f32 * alpha).round() as u8)
    }
    fn horizontal(&self, c: &Canvas, x: (f64, f64), y: f64, color: Color, px: u32, alpha: f32) {
        let (y, w) = self.rule(y, self.origin.1, px);
        stroke(c, (x.0, y), (x.1, y), Self::tinted(color, alpha), w);
    }
    fn vertical(&self, c: &Canvas, x: f64, y: (f64, f64), color: Color, px: u32, alpha: f32) {
        let (x, w) = self.rule(x, self.origin.0, px);
        stroke(c, (x, y.0), (x, y.1), Self::tinted(color, alpha), w);
    }
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
            let modern_critical = n.critical && !n.native_critical;
            if modern_critical {
                // Warm underlay keeps long critical runs visible at thumbnail size.
                let mut glow = paint(Color::from_rgb(240, 182, 36));
                glow.set_alpha_f(0.45);
                c.draw_round_rect(rect.with_outset((1.6, 1.6)), 2.2, 2.2, &glow);
            }
            let mut fill = paint(print_note_fill(n.role));
            if n.role == "connection" {
                fill.set_alpha_f(0.85);
                c.draw_round_rect(rect, 1., 1., &paint(Color::WHITE));
            }
            c.draw_round_rect(rect, 1.2, 1.2, &fill);
            if n.role != "connection" && n.role != "trace" && body >= 5. {
                // One pale highlight instead of a second dark edge.
                let mut shine = paint(Color::WHITE);
                shine.set_alpha_f(0.55);
                c.draw_rect(
                    Rect::from_xywh(
                        rect.left + 1.5,
                        rect.top + 1.2,
                        (rect.width() - 3.).max(0.),
                        (body * 0.22).max(1.),
                    ),
                    &shine,
                );
            }
            let mut edge = paint(if modern_critical {
                mix(color, Color::from_rgb(116, 70, 6), 0.55)
            } else {
                color
            });
            edge.set_style(sk::paint::Style::Stroke)
                .set_stroke_width(if n.role == "connection" { 0.9 } else { 1.2 });
            c.draw_round_rect(rect, 1.2, 1.2, &edge);
            if n.role == "trace" {
                stroke(
                    c,
                    ((x - width / 2. + 2.) as f64, y as f64),
                    ((x + width / 2. - 2.) as f64, y as f64),
                    color,
                    0.8,
                );
            }
        }
        NotePart::Arrow if n.role.starts_with("flick") => {
            let cy = y - body / 2. - 1.5 - n.arrow_height / 2.;
            let w = (width - 2.).clamp(2., 22.);
            draw_flick_arrow(c, n.role, (x, cy), w, n.arrow_height, color);
        }
        NotePart::Mark if n.critical => {
            if n.native_critical {
                return skin.draw_preview_part(c, n, part);
            }
            draw_critical_diamond(c, n, Theme::Print);
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
/// White/black omit an integral constant BPM (already exact in the header) and
/// keep short Call rhythms beside the chart as a two-line label.
fn display_events(scene: &Scene, ci: usize) -> (Vec<crate::scene::Annotation>, Vec<String>) {
    let modern = scene.layout.options.theme != Theme::Dark;
    let events: Vec<_> = scene
        .annotations
        .iter()
        .filter(|a| scene.layout.owner(a.tick) == Some(ci))
        .cloned()
        .collect();
    let mut bpm: Vec<_> = events.iter().filter(|a| a.kind == "bpm").cloned().collect();
    if modern && header_states_constant_bpm(scene) {
        bpm.clear();
    }
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
        if event.kind != "call" {
            continue;
        }
        if modern && call_rhythm_line(&event.label).is_some() {
            continue;
        }
        if event.label.len() > 12 {
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
/// True when every BPM event has one integral value, exactly as the sheet
/// header prints it.
fn header_states_constant_bpm(scene: &Scene) -> bool {
    let values: Vec<f64> = scene
        .annotations
        .iter()
        .filter(|a| a.kind == "bpm")
        .filter_map(|a| {
            a.label
                .strip_suffix(" BPM")
                .and_then(|s| s.parse::<f64>().ok())
        })
        .collect();
    let Some(&first) = values.first() else {
        return false;
    };
    values.iter().all(|v| (v - first).abs() < 0.005) && (first - first.round()).abs() < 0.005
}
/// Second line of an inline Call label. Only rhythm lists that fit the side
/// gutter unabridged at full size qualify; longer lists keep the appendix.
fn call_rhythm_line(label: &str) -> Option<String> {
    let rhythms = label.strip_prefix("CALL ")?;
    (!rhythms.is_empty()
        && rhythms.chars().count() <= 8
        && rhythms
            .chars()
            .all(|c| c.is_ascii_digit() || c == '%' || c == '/'))
    .then(|| rhythms.to_owned())
}
/// Lines occupied by one side label in white/black. Legacy dark is single-line.
fn label_lines(a: &crate::scene::Annotation, theme: Theme) -> usize {
    if theme != Theme::Dark && a.kind == "call" && call_rhythm_line(&a.label).is_some() {
        2
    } else {
        1
    }
}
/// Stable monotone label packing with leader lines; equal-time events remain
/// separate labels. Overflow is listed in an appendix panel, never summarized away.
pub fn pack_labels(desired: &[f64], top: f64, bottom: f64, gap: f64) -> (Vec<f64>, usize) {
    pack_label_blocks(desired, &vec![1; desired.len()], top, bottom, gap)
}
/// [`pack_labels`] for labels spanning `lines[i]` rows of `gap`. Returned y is
/// each label's first baseline; a whole label either fits or overflows.
pub fn pack_label_blocks(
    desired: &[f64],
    lines: &[usize],
    top: f64,
    bottom: f64,
    gap: f64,
) -> (Vec<f64>, usize) {
    let count = label_capacity(lines, top, bottom, gap).min(desired.len());
    let mut ys: Vec<f64> = Vec::new();
    for (i, &y) in desired[..count].iter().enumerate() {
        let floor = if i == 0 {
            top
        } else {
            ys[i - 1] + lines[i - 1] as f64 * gap
        };
        ys.push(y.max(floor));
    }
    if let Some(&last) = ys.last()
        && last + (lines[count - 1] - 1) as f64 * gap > bottom
    {
        let mut next = bottom + gap;
        for (y, n) in ys.iter_mut().zip(lines).rev() {
            *y = (*y).min(next - *n as f64 * gap);
            next = *y;
        }
    }
    (ys, desired.len() - count)
}
/// Leading labels whose rows fit between `top` and `bottom`.
fn label_capacity(lines: &[usize], top: f64, bottom: f64, gap: f64) -> usize {
    let rows = ((bottom - top) / gap).floor().max(0.) as usize;
    let mut used = 0;
    lines
        .iter()
        .take_while(|&&n| {
            used += n;
            used <= rows
        })
        .count()
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
    let mark = critical_mark_half(scene.layout.options.theme, scene.layout.options.note_height);
    for (i, a) in scene.glyphs.iter().enumerate() {
        for b in scene.glyphs.iter().skip(i + 1) {
            let delta = (b.note.tick - a.note.tick) as f64 * unit;
            if delta
                > scene.layout.options.arrow_height
                    + scene.layout.options.note_height
                    + (2. * mark - 1.6).max(3.)
            {
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
            if a.note.critical && delta < (scene.layout.options.note_height + 2. * mark) / 2. {
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
            let lines: Vec<_> = events.iter().map(|a| label_lines(a, theme)).collect();
            let ts = scene.layout.text_scale;
            let cap = label_capacity(&lines, top - 12. * ts, bottom + 18. * ts, 12. * ts);
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
        let ts = scene.layout.text_scale;
        let crisp = Crisp {
            origin: (0., header_shift),
            scale: scene.layout.options.output_scale,
        };
        // One heading row above the tallest column when tops are aligned.
        let heading_y = if scene.layout.top_aligned {
            Some(
                scene.layout.y(
                    &scene.layout.columns[range.start],
                    scene.layout.columns[range.start].end as f64,
                ) - scene.layout.pad_top
                    + 4. * ts,
            )
        } else {
            None
        };
        for (local, ci) in range.clone().enumerate() {
            let col = &scene.layout.columns[ci];
            let used_width =
                range.len() as f64 * (scene.layout.column_width + rail_width) + layout::GAP;
            let x = (width as f64 - used_width).max(0.) / 2.
                + layout::GAP
                + local as f64 * (scene.layout.column_width + rail_width)
                + scene.layout.left;
            let track = 24. * scene.layout.options.pixels_per_lane;
            let event_x = x + track + rail_width;
            let bottom = scene.layout.y(col, col.start as f64);
            let top = scene.layout.y(col, col.end as f64);
            let rect = Rect::new(x as f32, top as f32, (x + track) as f32, bottom as f32);
            panel(c, rect, palette.panel);
            let heading = format!("{:03}—{:03}", col.first_bar, col.last_bar);
            let heading_size = if modern { 14. * ts as f32 } else { 11. };
            // The heading starts at the track; when it alone is wider than the
            // track (narrow sheets) it is centered on the column instead.
            let heading_width = fonts.width(&heading, heading_size, true) as f64;
            let heading_x = if modern && heading_width > track {
                (x + (track - heading_width) / 2.).max(x - scene.layout.left + 4.)
            } else {
                x
            };
            fonts.draw(
                c,
                &heading,
                heading_x,
                if let Some(y) = heading_y {
                    y
                } else if modern {
                    top - scene.layout.pad_top - 10.
                } else {
                    layout::HEADER - 5.
                },
                heading_size,
                palette.text,
                true,
                (scene.layout.column_width - scene.layout.left - layout::GAP) as f32,
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
                if modern {
                    // One opaque warm field replaces the lane bands underneath, so
                    // the range reads as a block rather than grey/beige stripes.
                    c.draw_rect(
                        Rect::new(x as f32, y0 as f32, (x + track) as f32, y1 as f32),
                        &paint(if print { PRINT_FEVER } else { BLACK_FEVER }),
                    );
                } else {
                    c.draw_rect(
                        Rect::new(x as f32, y0 as f32, (x + track) as f32, y1 as f32),
                        &paint(Color::from_argb(9, 255, 205, 118)),
                    );
                }
                stroke(
                    c,
                    (x - 3., y0),
                    (x - 3., y1),
                    FEVER_RAIL,
                    if modern { 2.5 } else { 2. },
                );
            }
            for lane in 0..=24 {
                let bold = lane % 6 == 0;
                let lx = x + lane as f64 * scene.layout.options.pixels_per_lane;
                let (color, w) = if bold {
                    (palette.lane, 0.7)
                } else {
                    (palette.fine_lane, 0.35)
                };
                if modern {
                    crisp.vertical(c, lx, (top, bottom), color, 1, w);
                } else {
                    stroke(c, (lx, top), (lx, bottom), color, w);
                }
            }
            for bar in scene
                .layout
                .bars
                .iter()
                .filter(|b| col.start <= b.tick && b.tick <= col.end)
            {
                let y = scene.layout.y(col, bar.tick as f64);
                let major = modern && (bar.number == col.first_bar || bar.number % 4 == 1);
                if modern {
                    // Major measures: two crisp pixels; others one lighter pixel.
                    let (px, alpha) = if major { (2, 0.62) } else { (1, 0.8) };
                    crisp.horizontal(c, (x, x + track), y, palette.bar, px, alpha);
                } else {
                    stroke(c, (x, y), (x + track, y), palette.bar, 0.9);
                }
                let size = if modern { 10.5 * ts as f32 } else { 9.5 };
                fonts.draw(
                    c,
                    &format!("{:03}", bar.number),
                    x - 26. * ts,
                    y + 3. * ts,
                    size,
                    palette.muted,
                    major,
                    25. * ts as f32,
                );
            }
            for segment in scene.layout.bars.windows(2) {
                let mut tick = segment[0].tick + 240;
                while tick < segment[1].tick {
                    if tick >= col.start && tick < col.end {
                        let y = scene.layout.y(col, tick as f64);
                        let color = if (tick - segment[0].tick) % 480 == 0 {
                            palette.beat
                        } else {
                            palette.subdivision
                        };
                        if modern {
                            crisp.horizontal(c, (x, x + track), y, color, 1, 0.5);
                        } else {
                            stroke(c, (x, y), (x + track, y), color, 0.5);
                        }
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
                    (top - scene.layout.pad_top) as f32,
                    (x + track) as f32,
                    (bottom + scene.layout.pad_bottom) as f32,
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
                    let missing_arrow = draw_note_part(c, skin, &request, part, theme)?;
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
            let step = 12. * ts;
            let desired: Vec<f64> = annotations
                .iter()
                .map(|a| scene.layout.y(col, a.tick as f64) + 3. * ts)
                .collect();
            let lines: Vec<_> = annotations.iter().map(|a| label_lines(a, theme)).collect();
            let (ys, overflow) =
                pack_label_blocks(&desired, &lines, top - step, bottom + 18. * ts, step);
            let label_room = (scene.layout.right - 8. * ts) as f32;
            for (a, &y) in annotations.iter().zip(&ys) {
                let color = event_color(&a.kind, theme);
                let anchor = scene.layout.y(col, a.tick as f64);
                stroke(
                    c,
                    (x + track, anchor),
                    (event_x + 4. * ts, y - 3. * ts),
                    color,
                    0.6 * ts as f32,
                );
                let rhythm = (modern && a.kind == "call")
                    .then(|| call_rhythm_line(&a.label))
                    .flatten();
                let label = if rhythm.is_some() {
                    "CALL".to_owned()
                } else if let Some(bpm) = a.label.strip_suffix(" BPM") {
                    bpm.trim_end_matches('0').trim_end_matches('.').to_owned()
                } else {
                    a.label.clone()
                };
                for (i, text) in std::iter::once(label).chain(rhythm).enumerate() {
                    let full = 9.5 * ts as f32;
                    let size = (full
                        * (label_room / fonts.width(&text, full, false).max(1.)).min(1.))
                    .max(7. * ts as f32);
                    fonts.draw(
                        c,
                        &text,
                        event_x + 6. * ts,
                        y + i as f64 * step,
                        size,
                        color,
                        false,
                        label_room,
                    );
                }
            }
            if overflow > 0 || !details.is_empty() {
                fonts.draw(
                    c,
                    if modern { "↳ NOTES" } else { "↳ appendix" },
                    event_x + 4. * ts,
                    bottom + 31. * ts,
                    8. * ts as f32,
                    palette.muted,
                    false,
                    55. * ts as f32,
                );
            }
            let time = scene
                .layout
                .bars
                .iter()
                .find(|b| b.tick == col.start)
                .map(|b| b.time_ms / 1000)
                .unwrap_or(0);
            let end_time = scene
                .layout
                .bars
                .iter()
                .find(|b| b.tick == col.end)
                .map(|b| b.time_ms / 1000)
                .unwrap_or(0);
            let time_size = 9. * ts as f32;
            let start_label = format!("{}:{:02}  ↑", time / 60, time % 60);
            let end_label = format!("→  {}:{:02}", end_time / 60, end_time % 60);
            let end_width = fonts.width(&end_label, time_size, false) as f64;
            // Aligned end times share the heading row when both fit the track.
            let stacked = if scene.layout.top_aligned {
                heading_width + 16. * ts + end_width > track
            } else {
                track < 140.
            };
            if scene.layout.top_aligned && !stacked {
                // The start time stays under the column's first measure; its end
                // time shares the aligned heading row, right-aligned to the track.
                fonts.draw(
                    c,
                    &start_label,
                    x,
                    bottom + 20. * ts,
                    time_size,
                    palette.muted,
                    false,
                    (track as f32).min(80. * ts as f32),
                );
                fonts.draw(
                    c,
                    &end_label,
                    (x + track - end_width).max(x),
                    heading_y.unwrap_or(top - 8. * ts),
                    time_size,
                    palette.muted,
                    false,
                    end_width as f32 + 1.,
                );
            } else {
                // Narrow tracks stack both times below the column; legacy dark
                // keeps its original positions.
                let s = if modern { ts } else { 1. };
                fonts.draw(
                    c,
                    &start_label,
                    x,
                    bottom + 20. * s,
                    time_size,
                    palette.muted,
                    false,
                    (track as f32).min(80. * s as f32),
                );
                fonts.draw(
                    c,
                    &end_label,
                    if stacked { x } else { x + track - 70. },
                    bottom + if stacked { 34. * s } else { 20. },
                    time_size,
                    palette.muted,
                    false,
                    (track as f32).min(70. * s as f32),
                );
            }
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
            schema_version: 4,
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
            chart_text_scale: scene.layout.text_scale,
            columns_top_aligned: scene.layout.top_aligned,
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
    fn scene_with(chart: &[u8], theme: Theme) -> Result<Scene> {
        let score = crate::parser::Score::parse(chart, false)?;
        Scene::build(
            &score,
            crate::layout::Layout::build(
                &score,
                crate::layout::Options {
                    theme,
                    ..crate::layout::Options::default()
                },
            )?,
        )
    }
    #[test]
    fn crisp_rules_cover_whole_output_pixels() {
        for scale in [0.5, 1., 2.] {
            let crisp = Crisp {
                origin: (0., 13.37),
                scale,
            };
            for v in [100.2, 100.5, 333.33] {
                for px in [1, 2] {
                    let (center, width) = crisp.rule(v, crisp.origin.1, px);
                    let edge = (center - width as f64 / 2. + crisp.origin.1) * scale;
                    assert!((edge - edge.round()).abs() < 1e-6, "{scale} {v} {px}");
                    assert!((width as f64 * scale - px as f64).abs() < 1e-6);
                    // The rule stays within one pixel of the requested position.
                    assert!((center - v).abs() * scale <= 1.);
                }
            }
        }
    }
    #[test]
    fn multi_line_labels_keep_their_rows_and_fit_or_overflow_whole() {
        let (y, n) = pack_label_blocks(&[5., 5., 6.], &[1, 2, 1], 0., 60., 12.);
        assert_eq!(n, 0);
        assert!(y[1] - y[0] >= 12. && y[2] - y[1] >= 24.);
        assert!(y[2] <= 60.);
        // A two-row label does not partially fit into one remaining row.
        let (_, n) = pack_label_blocks(&[0., 0.], &[1, 2], 0., 24., 12.);
        assert_eq!(n, 1);
        assert_eq!(
            pack_labels(&[5., 5., 6., 19.], 0., 50., 12.),
            pack_label_blocks(&[5., 5., 6., 19.], &[1; 4], 0., 50., 12.)
        );
    }
    #[test]
    fn modern_sheets_keep_short_calls_beside_chart_and_omit_header_bpm() -> Result<()> {
        let chart = br#"{"events":{"bpm":[{"t":0,"bpm":170}],"sig":[{"t":0,"sig":[4,4]}],"call":[{"t":0,"timing":[0,1,0,1]}]},"notes":[{"t":480,"pos":4,"size":6}]}"#;
        for theme in [Theme::Print, Theme::Black] {
            let scene = scene_with(chart, theme)?;
            let (shown, appendix) = display_events(&scene, 0);
            assert!(appendix.is_empty(), "{appendix:?}");
            assert!(shown.iter().all(|a| a.kind != "bpm"));
            let call = shown.iter().find(|a| a.kind == "call").unwrap();
            assert_eq!(call_rhythm_line(&call.label).as_deref(), Some("50%/100%"));
            assert_eq!(label_lines(call, theme), 2);
            // The report keeps the source BPM annotation.
            assert!(scene.annotations.iter().any(|a| a.label == "170.00 BPM"));
        }
        let dark = scene_with(chart, Theme::Dark)?;
        let (shown, appendix) = display_events(&dark, 0);
        assert!(shown.iter().any(|a| a.label == "170.00 BPM"));
        assert_eq!(appendix.len(), 1);
        Ok(())
    }
    #[test]
    fn long_calls_and_non_integral_or_changing_tempo_stay_explicit() -> Result<()> {
        let long = br#"{"events":{"bpm":[{"t":0,"bpm":140},{"t":3840,"bpm":180}],"call":[{"t":0,"timing":[1,0,1,1]}]},"notes":[]}"#;
        let scene = scene_with(long, Theme::Print)?;
        let (shown, appendix) = display_events(&scene, 0);
        assert!(shown.iter().any(|a| a.kind == "bpm"));
        assert!(shown.iter().any(|a| a.label == "CALL ↳"));
        assert!(appendix.iter().any(|a| a.contains("CALL 25%/75%/100%")));
        let fractional = br#"{"events":{"bpm":[{"t":0,"bpm":127.5}]},"notes":[]}"#;
        let (shown, _) = display_events(&scene_with(fractional, Theme::Print)?, 0);
        assert!(shown.iter().any(|a| a.label == "127.50 BPM"));
        Ok(())
    }
}
