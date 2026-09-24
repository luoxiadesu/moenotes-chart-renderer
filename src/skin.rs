//! Skin pack loading and static sprite assembly.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skia_safe::{self as sk, Canvas, Color, Data, Image, Paint, Rect};
use std::{collections::BTreeMap, fs, path::Path};

pub const ROLES: [&str; 8] = [
    "tap",
    "slide",
    "connection",
    "slide_end",
    "trace",
    "flick",
    "flick_left",
    "flick_right",
];

#[derive(Debug, Deserialize)]
pub struct Sprite {
    pub name: String,
    pub file: String,
    pub size: [i32; 2],
    pub sha256: String,
    pub rect: SpriteRect,
    pub pivot: [f32; 2],
    pub pixels_per_unit: f32,
    /// min x, min y, max x, max y relative to the sprite's pivot.
    pub bounds_units: [f32; 4],
    /// Unity order: left, bottom, right, top, in source pixels.
    pub border: [f32; 4],
}
#[derive(Debug, Deserialize)]
pub struct SpriteRect {
    pub width: f32,
    pub height: f32,
}
#[derive(Debug, Deserialize)]
pub struct Parts {
    pub tilt: i32,
    pub left: Option<String>,
    pub right: Option<String>,
    pub left_overhang: f32,
    pub right_overhang: f32,
}
#[derive(Debug, Deserialize)]
pub struct Arrow {
    pub max_width: f32,
    pub sprite: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Note {
    pub main: String,
    pub mark: Option<String>,
    pub parts: Vec<Parts>,
    pub arrows: Vec<Arrow>,
    pub sub_arrow: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Transform {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub angle_degrees: f32,
}
#[derive(Debug, Deserialize)]
pub struct Profile {
    pub arrow: Option<Transform>,
    pub sub_arrow: Option<Transform>,
}
#[derive(Debug, Deserialize)]
pub struct ColorKey {
    pub at: f32,
    pub rgb: [f32; 3],
}
#[derive(Debug, Deserialize)]
pub struct AlphaKey {
    pub at: f32,
    pub alpha: f32,
}
#[derive(Debug, Deserialize)]
pub struct Gradient {
    pub colors: Vec<ColorKey>,
    pub alphas: Vec<AlphaKey>,
    pub mode: i32,
}
#[derive(Debug, Deserialize)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
#[derive(Debug, Deserialize)]
pub struct Line {
    pub width_scale: f32,
    pub glow_range_scale: f32,
    pub normal: Gradient,
    pub pressed: Gradient,
    pub disabled: Gradient,
    pub guide_color: Rgba,
    pub material_floats: BTreeMap<String, f32>,
}
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub skin: String,
    pub sprites: BTreeMap<String, Sprite>,
    pub notes: BTreeMap<String, Note>,
    pub prefab_profiles: BTreeMap<String, Profile>,
    pub line: Line,
}
pub struct Skin {
    pub manifest: Manifest,
    images: BTreeMap<String, Image>,
}

pub fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

impl Skin {
    pub fn builtin() -> Result<Self> {
        // Original vector-inspired artwork; no game texture is bundled.
        let mut sprites = BTreeMap::new();
        let mut images = BTreeMap::new();
        let mut notes = BTreeMap::new();
        let mut profiles = BTreeMap::new();
        let colors = [
            Color::from_rgb(113, 203, 234),
            Color::from_rgb(131, 152, 247),
            Color::from_rgb(131, 152, 247),
            Color::from_rgb(106, 131, 225),
            Color::from_rgb(194, 164, 248),
            Color::from_rgb(247, 199, 93),
            Color::from_rgb(106, 218, 155),
            Color::from_rgb(246, 144, 174),
        ];
        for (index, role) in ROLES.iter().enumerate() {
            let color = colors[index];
            for (part, w, h) in [
                ("main", 20, 82),
                ("left", 20, 82),
                ("right", 20, 82),
                ("mark", 22, 22),
                ("arrow", 160, 60),
            ] {
                let id = format!("{role}-{part}");
                let mut surface =
                    sk::surfaces::raster_n32_premul((w, h)).context("Built-in skin canvas")?;
                let c = surface.canvas();
                c.clear(Color::TRANSPARENT);
                let mut p = Paint::default();
                p.set_anti_alias(true).set_color(color);
                if part == "arrow" {
                    p.set_style(sk::paint::Style::Stroke).set_stroke_width(10.);
                    let mut b = sk::PathBuilder::new();
                    if *role == "flick" {
                        b.move_to((20., 48.));
                        b.line_to((80., 14.));
                        b.line_to((140., 48.));
                    } else {
                        let sign = if *role == "flick_left" { -1. } else { 1. };
                        for x in [55., 105.] {
                            b.move_to((x - sign * 18., 12.));
                            b.line_to((x + sign * 12., 30.));
                            b.line_to((x - sign * 18., 48.));
                        }
                    }
                    c.draw_path(&b.detach(), &p);
                } else if part == "mark" {
                    c.draw_circle((11., 11.), 7., &p);
                } else {
                    c.draw_round_rect(Rect::from_xywh(0., 10., w as f32, 62.), 5., 5., &p);
                    p.set_color(Color::from_argb(220, 243, 250, 255));
                    c.draw_rect(Rect::from_xywh(2., 12., w as f32 - 4., 15.), &p);
                }
                let image = surface.image_snapshot();
                let data = image
                    .encode(None, sk::EncodedImageFormat::PNG, 100)
                    .context("Built-in PNG")?;
                sprites.insert(
                    id.clone(),
                    Sprite {
                        name: id.clone(),
                        file: String::new(),
                        size: [w, h],
                        sha256: sha256(data.as_bytes()),
                        rect: SpriteRect {
                            width: w as f32,
                            height: h as f32,
                        },
                        pivot: [0.5, 0.5],
                        pixels_per_unit: 100.,
                        bounds_units: [
                            -w as f32 / 200.,
                            -h as f32 / 200.,
                            w as f32 / 200.,
                            h as f32 / 200.,
                        ],
                        border: if part == "main" {
                            [4., 0., 4., 0.]
                        } else {
                            [0.; 4]
                        },
                    },
                );
                images.insert(id, image);
            }
            notes.insert(
                role.to_string(),
                Note {
                    main: format!("{role}-main"),
                    mark: Some(format!("{role}-mark")),
                    parts: vec![Parts {
                        tilt: 0,
                        left: Some(format!("{role}-left")),
                        right: Some(format!("{role}-right")),
                        left_overhang: 0.,
                        right_overhang: 0.,
                    }],
                    arrows: if role.starts_with("flick") {
                        vec![Arrow {
                            max_width: 999.,
                            sprite: Some(format!("{role}-arrow")),
                        }]
                    } else {
                        vec![]
                    },
                    sub_arrow: None,
                },
            );
            profiles.insert(
                role.to_string(),
                Profile {
                    arrow: Some(Transform {
                        position: [0., 1.],
                        scale: [1., 1.],
                        angle_degrees: 0.,
                    }),
                    sub_arrow: None,
                },
            );
        }
        let gradient = || Gradient {
            colors: vec![
                ColorKey {
                    at: 0.,
                    rgb: [0.33, 0.49, 0.95],
                },
                ColorKey {
                    at: 1.,
                    rgb: [0.44, 0.62, 0.94],
                },
            ],
            alphas: vec![
                AlphaKey { at: 0., alpha: 0.7 },
                AlphaKey { at: 1., alpha: 0.7 },
            ],
            mode: 0,
        };
        Ok(Self {
            manifest: Manifest {
                schema_version: 1,
                skin: "builtin".into(),
                sprites,
                notes,
                prefab_profiles: profiles,
                line: Line {
                    width_scale: 0.9,
                    glow_range_scale: 2.,
                    normal: gradient(),
                    pressed: gradient(),
                    disabled: gradient(),
                    guide_color: Rgba {
                        r: 0.65,
                        g: 0.53,
                        b: 0.92,
                        a: 0.4,
                    },
                    material_floats: BTreeMap::new(),
                },
            },
            images,
        })
    }
    pub fn load(path: &Path) -> Result<Self> {
        let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
        ensure!(manifest.schema_version == 1, "Unsupported skin schema");
        let mut images = BTreeMap::new();
        for (id, s) in &manifest.sprites {
            ensure!(
                s.pixels_per_unit.is_finite() && s.pixels_per_unit > 0.,
                "Invalid PPU for {id}"
            );
            ensure!(
                s.size[0] > 0
                    && s.size[1] > 0
                    && (s.size[0] as i64) * (s.size[1] as i64) <= 16_000_000,
                "Sprite dimensions out of bounds"
            );
            ensure!(
                s.bounds_units
                    .iter()
                    .chain(s.border.iter())
                    .chain(s.pivot.iter())
                    .all(|x| x.is_finite()),
                "Invalid geometry for {id}"
            );
            ensure!(
                s.bounds_units[2] > s.bounds_units[0] && s.bounds_units[3] > s.bounds_units[1],
                "Empty sprite {id}"
            );
            let base = path
                .parent()
                .context("Skin directory missing")?
                .canonicalize()?;
            let relative = Path::new(&s.file);
            ensure!(
                relative
                    .components()
                    .all(|p| matches!(p, std::path::Component::Normal(_))),
                "Unsafe sprite path for {id}"
            );
            let file = base.join(relative).canonicalize()?;
            ensure!(
                file.starts_with(&base),
                "Sprite path escapes skin directory"
            );
            ensure!(
                fs::metadata(&file)?.len() <= 32 * 1024 * 1024,
                "Sprite file exceeds 32 MiB"
            );
            let bytes = fs::read(&file)?;
            ensure!(sha256(&bytes) == s.sha256, "Sprite checksum mismatch: {id}");
            let image = Image::from_encoded(Data::new_copy(&bytes))
                .with_context(|| format!("Cannot decode {id}"))?;
            ensure!(
                [image.width(), image.height()] == s.size,
                "Sprite dimensions mismatch: {id}"
            );
            images.insert(id.clone(), image);
        }
        for role in ROLES {
            let n = manifest
                .notes
                .get(role)
                .with_context(|| format!("Missing role {role}"))?;
            let zero = n
                .parts
                .iter()
                .find(|p| p.tilt == 0)
                .context("Missing zero tilt")?;
            ensure!(
                zero.left.is_some() && zero.right.is_some(),
                "Missing zero-tilt cap"
            );
            let mut refs = vec![&n.main];
            refs.extend(n.mark.iter());
            refs.extend(n.sub_arrow.iter());
            for p in &n.parts {
                refs.extend(p.left.iter());
                refs.extend(p.right.iter());
            }
            for a in &n.arrows {
                refs.extend(a.sprite.iter());
            }
            for id in refs {
                ensure!(images.contains_key(id), "Unresolved {role} sprite {id}");
            }
            ensure!(
                n.arrows.iter().all(|a| a.max_width.is_finite()),
                "Nonfinite arrow threshold"
            );
            ensure!(
                n.arrows.windows(2).all(|a| a[0].max_width < a[1].max_width),
                "Unsorted arrows"
            );
            let s = &manifest.sprites[&n.main];
            // Source-center nine slicing must use the original canvas, never a
            // cropped PNG. The supplied main sprites all retain that canvas.
            ensure!(
                (s.rect.width - s.size[0] as f32).abs() < 0.01
                    && (s.rect.height - s.size[1] as f32).abs() < 0.01,
                "Trimmed main sprite needs canvas reconstruction: {}",
                s.name
            );
            ensure!(
                s.border[0] + s.border[2] <= s.rect.width
                    && s.border[1] + s.border[3] <= s.rect.height,
                "Invalid nine-slice border"
            );
            ensure!(
                n.parts
                    .iter()
                    .all(|p| p.left_overhang.is_finite() && p.right_overhang.is_finite()),
                "Nonfinite cap overhang"
            );
            ensure!(
                manifest.prefab_profiles.contains_key(role),
                "Missing prefab profile"
            );
        }
        for profile in manifest.prefab_profiles.values() {
            for t in profile.arrow.iter().chain(profile.sub_arrow.iter()) {
                ensure!(
                    t.position
                        .iter()
                        .chain(t.scale.iter())
                        .chain(std::iter::once(&t.angle_degrees))
                        .all(|v| v.is_finite()),
                    "Invalid sprite transform"
                );
            }
        }
        ensure!(
            manifest.line.width_scale.is_finite() && manifest.line.glow_range_scale.is_finite(),
            "Invalid line scale"
        );
        for g in [
            &manifest.line.normal,
            &manifest.line.pressed,
            &manifest.line.disabled,
        ] {
            ensure!(
                g.mode == 0 && !g.colors.is_empty() && !g.alphas.is_empty(),
                "Unsupported/empty gradient"
            );
            ensure!(
                g.colors
                    .iter()
                    .all(|k| k.at.is_finite() && k.rgb.iter().all(|v| v.is_finite()))
                    && g.alphas
                        .iter()
                        .all(|k| k.at.is_finite() && k.alpha.is_finite()),
                "Invalid gradient"
            );
            ensure!(
                g.colors.windows(2).all(|k| k[0].at < k[1].at)
                    && g.alphas.windows(2).all(|k| k[0].at < k[1].at),
                "Unsorted gradient"
            );
        }
        Ok(Self { manifest, images })
    }

    /// Draw the actual decoded crop at its original local bounds, not centered.
    fn sprite(&self, c: &Canvas, id: &str, origin: (f32, f32), scale: (f32, f32), black: bool) {
        let s = &self.manifest.sprites[id];
        let b = s.bounds_units;
        let dst = Rect::new(
            origin.0 + b[0] * scale.0,
            origin.1 - b[3] * scale.1,
            origin.0 + b[2] * scale.0,
            origin.1 - b[1] * scale.1,
        );
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        if black {
            paint.set_color_filter(sk::color_filters::blend(Color::BLACK, sk::BlendMode::SrcIn));
        }
        c.draw_image_rect_with_sampling_options(
            &self.images[id],
            None,
            dst,
            sk::FilterMode::Linear,
            &paint,
        );
    }

    fn nine_slice(&self, c: &Canvas, id: &str, center: (f32, f32), width: f32, unit_px: f32) {
        if width <= 0. {
            return;
        }
        let s = &self.manifest.sprites[id];
        let px = unit_px / s.pixels_per_unit;
        let height = s.rect.height * px;
        let x = slice_axis(s.rect.width, s.border[0], s.border[2], width, px);
        let y = slice_axis(s.rect.height, s.border[3], s.border[1], height, px);
        let left = center.0 - width * s.pivot[0];
        let top = center.1 - height * (1. - s.pivot[1]);
        let paint = Paint::default();
        for xx in &x {
            for yy in &y {
                if xx[3] <= xx[2] || yy[3] <= yy[2] {
                    continue;
                }
                let src = Rect::new(xx[0], yy[0], xx[1], yy[1]);
                let dst = Rect::new(left + xx[2], top + yy[2], left + xx[3], top + yy[3]);
                c.draw_image_rect_with_sampling_options(
                    &self.images[id],
                    Some((&src, sk::canvas::SrcRectConstraint::Strict)),
                    dst,
                    sk::FilterMode::Linear,
                    &paint,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // Explicit rendering coordinates and style, no mutable context.
    pub fn draw_note(
        &self,
        c: &Canvas,
        role: &str,
        width_lanes: f32,
        center: (f32, f32),
        unit_px: f32,
        critical: bool,
    ) -> Result<DrawInfo> {
        ensure!(
            width_lanes.is_finite() && width_lanes >= 0. && unit_px.is_finite() && unit_px > 0.,
            "Invalid note dimensions"
        );
        let n = self.manifest.notes.get(role).context("Unknown note role")?;
        if width_lanes == 0. {
            return Ok(DrawInfo {
                role: role.into(),
                width_lanes,
                critical,
                arrow: None,
                empty_arrow: false,
                cap_scale: 0.,
            });
        }
        let p = n
            .parts
            .iter()
            .find(|p| p.tilt == 0)
            .context("No flat parts")?;
        let l = p.left.as_ref().context("No left cap")?;
        let r = p.right.as_ref().context("No right cap")?;
        let lb = self.manifest.sprites[l].bounds_units;
        let rb = self.manifest.sprites[r].bounds_units;
        // Preview convention: one lane is 0.5 local units. View speed/perspective
        // are absent; a single unit scale controls the entire sprite assembly.
        let width = width_lanes * 0.5;
        let layout = cap_layout(
            width,
            lb[2] - lb[0],
            rb[2] - rb[0],
            p.left_overhang,
            p.right_overhang,
        );
        self.nine_slice(
            c,
            &n.main,
            (center.0 + layout.main_center * unit_px, center.1),
            layout.main_width * unit_px,
            unit_px,
        );
        self.sprite(
            c,
            l,
            (center.0 + layout.left_center * unit_px, center.1),
            (unit_px * layout.scale, unit_px * layout.scale),
            false,
        );
        self.sprite(
            c,
            r,
            (center.0 + layout.right_center * unit_px, center.1),
            (unit_px * layout.scale, unit_px * layout.scale),
            false,
        );
        if let Some(mark) = &n.mark {
            let factor = if critical { 2. } else { 1. };
            self.sprite(
                c,
                mark,
                center,
                (unit_px * factor, unit_px * factor),
                critical,
            );
        }
        let selected = select_arrow(&n.arrows, width_lanes);
        let profile = &self.manifest.prefab_profiles[role];
        if let Some(a) = selected.and_then(|a| a.sprite.as_ref()) {
            let t = profile.arrow.as_ref().context("Missing arrow transform")?;
            self.draw_transformed(c, a, center, unit_px, t);
        }
        if let Some(id) = &n.sub_arrow {
            let t = profile
                .sub_arrow
                .as_ref()
                .context("Configured sub-arrow has no renderer")?;
            self.draw_transformed(c, id, center, unit_px, t);
        }
        Ok(DrawInfo {
            role: role.into(),
            width_lanes,
            critical,
            arrow: selected.and_then(|a| a.sprite.clone()),
            empty_arrow: selected.is_some_and(|a| a.sprite.is_none()),
            cap_scale: layout.scale,
        })
    }

    fn draw_transformed(
        &self,
        c: &Canvas,
        id: &str,
        center: (f32, f32),
        unit_px: f32,
        t: &Transform,
    ) {
        c.save();
        c.translate((
            center.0 + t.position[0] * unit_px,
            center.1 - t.position[1] * unit_px,
        ));
        c.rotate(-t.angle_degrees, None);
        self.sprite(
            c,
            id,
            (0., 0.),
            (unit_px * t.scale[0], unit_px * t.scale[1]),
            false,
        );
        c.restore();
    }

    /// Preview part pass. Bodies, arrows and marks are separate global passes.
    pub fn draw_preview_part(
        &self,
        c: &Canvas,
        request: &PreviewNote<'_>,
        part: NotePart,
    ) -> Result<bool> {
        let PreviewNote {
            role,
            width_lanes,
            center,
            lane_px,
            body_height,
            arrow_height,
            critical,
            native_critical,
            strict_assets,
        } = *request;
        let n = self
            .manifest
            .notes
            .get(role)
            .context("Unknown preview role")?;
        if width_lanes <= 0. {
            return Ok(false);
        }
        let main = &self.manifest.sprites[&n.main];
        let tap = &self.manifest.sprites[&self.manifest.notes["tap"].main];
        let unit = body_height / (tap.rect.height / tap.pixels_per_unit);
        let width = width_lanes * lane_px;
        let p = n
            .parts
            .iter()
            .find(|p| p.tilt == 0)
            .context("No zero tilt")?;
        if part == NotePart::Body {
            let left = p.left.as_ref().context("No cap")?;
            let right = p.right.as_ref().context("No cap")?;
            let l = &self.manifest.sprites[left];
            let r = &self.manifest.sprites[right];
            let a = cap_layout(
                width,
                (l.bounds_units[2] - l.bounds_units[0]) * unit,
                (r.bounds_units[2] - r.bounds_units[0]) * unit,
                p.left_overhang * unit,
                p.right_overhang * unit,
            );
            self.nine_slice(
                c,
                &n.main,
                (center.0 + a.main_center, center.1),
                a.main_width,
                unit,
            );
            self.sprite(
                c,
                left,
                (center.0 + a.left_center, center.1),
                (unit * a.scale, unit * a.scale),
                false,
            );
            self.sprite(
                c,
                right,
                (center.0 + a.right_center, center.1),
                (unit * a.scale, unit * a.scale),
                false,
            );
        } else if part == NotePart::Mark {
            if let Some(id) = &n.mark {
                let m = &self.manifest.sprites[id];
                let natural = (m.bounds_units[3] - m.bounds_units[1]) * unit;
                let factor = if native_critical {
                    if critical { 2. } else { 1. }
                } else {
                    (if critical { 4.2 } else { 2.4 }) / natural.max(0.01)
                };
                self.sprite(
                    c,
                    id,
                    center,
                    (unit * factor, unit * factor),
                    native_critical && critical,
                );
                if critical && !native_critical {
                    let mut b = sk::PathBuilder::new();
                    b.move_to((center.0, center.1 - 2.3));
                    b.line_to((center.0 + 2.3, center.1));
                    b.line_to((center.0, center.1 + 2.3));
                    b.line_to((center.0 - 2.3, center.1));
                    b.close();
                    let mut paint = Paint::default();
                    paint
                        .set_anti_alias(true)
                        .set_color(Color::from_rgb(255, 224, 135));
                    c.draw_path(&b.detach(), &paint);
                }
            }
        } else if let Some(a) = select_arrow(&n.arrows, width_lanes) {
            if let Some(id) = &a.sprite {
                let sprite = &self.manifest.sprites[id];
                let b = sprite.bounds_units;
                let natural_x = (unit * 0.95).min((width + 2.) / (b[2] - b[0]));
                let scale_y = arrow_height / (b[3] - b[1]);
                let scale_x = natural_x;
                let angle = self.manifest.prefab_profiles[role]
                    .arrow
                    .as_ref()
                    .map(|t| t.angle_degrees)
                    .unwrap_or(0.);
                // Flat preview anchors the visible arrow immediately above the
                // body. Its crop is still placed from mesh bounds and pivot.
                let rotation = angle.to_radians();
                let flip = rotation.cos() < 0.;
                let sign = if flip { -1. } else { 1. };
                let cx = (b[0] + b[2]) * 0.5 * sign;
                let bottom = if flip { -b[3] } else { b[1] };
                let gap = 1.5;
                let body = main.rect.height / main.pixels_per_unit * unit;
                c.save();
                c.translate((
                    center.0 - cx * scale_x,
                    center.1 - body / 2. - gap + bottom * scale_y,
                ));
                c.rotate(-angle, None);
                self.sprite(c, id, (0., 0.), (scale_x, scale_y), false);
                c.restore();
            } else if !strict_assets {
                // Explicit fallback for a NULL source slot: vector direction
                // symbol, not an invented replacement game sprite.
                let w = width.min(24.);
                let y = center.1 - body_height / 2. - arrow_height / 2. - 1.5;
                let mut b = sk::PathBuilder::new();
                let sign = if role == "flick_left" { -1. } else { 1. };
                for offset in [-w * 0.22, w * 0.22] {
                    b.move_to((center.0 + offset - sign * 3., y - arrow_height * 0.4));
                    b.line_to((center.0 + offset + sign * 2., y));
                    b.line_to((center.0 + offset - sign * 3., y + arrow_height * 0.4));
                }
                let mut paint = Paint::default();
                paint
                    .set_anti_alias(true)
                    .set_color(Color::from_rgb(255, 142, 164))
                    .set_style(sk::paint::Style::Stroke)
                    .set_stroke_width(2.);
                c.draw_path(&b.detach(), &paint);
                return Ok(true);
            } else {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Approximate screen-space glow, deliberately independent of Unity shader.
    pub fn draw_band(&self, c: &Canvas, rect: Rect, kind: &str) -> Result<()> {
        let line = &self.manifest.line;
        let (gradient, guide) = match kind {
            "normal" => (Some(&line.normal), false),
            "pressed" => (Some(&line.pressed), false),
            "disabled" => (Some(&line.disabled), false),
            "guide" => (None, true),
            _ => bail!("Unknown band state"),
        };
        // Reference ribbon uses 12 pixels/local unit and a 65-pixel minimum
        // width. Apply the recovered reference-width inset before drawing.
        let inset = if guide {
            0.
        } else {
            ((1. - line.width_scale) * 6.).min(65. / 12. / 2.) * 12. / 2.
        };
        let mut b = sk::PathBuilder::new();
        b.move_to((rect.left + 25. + inset, rect.bottom));
        b.cubic_to(
            (rect.left + inset, rect.center_y()),
            (rect.right - 65. + inset, rect.center_y()),
            (rect.right - 65. + inset, rect.top),
        );
        b.line_to((rect.right - inset, rect.top));
        b.cubic_to(
            (rect.right - inset, rect.center_y()),
            (rect.left + 95. - inset, rect.center_y()),
            (rect.left + 95. - inset, rect.bottom),
        );
        b.close();
        let path = b.detach();
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        if guide {
            paint.set_color(rgba(
                [line.guide_color.r, line.guide_color.g, line.guide_color.b],
                line.guide_color.a,
            ));
        } else if let Some(g) = gradient {
            let mut stops: Vec<f32> = g
                .colors
                .iter()
                .map(|k| k.at)
                .chain(g.alphas.iter().map(|k| k.at))
                .chain([0., 1.])
                .collect();
            stops.sort_by(f32::total_cmp);
            stops.dedup();
            let colors: Vec<sk::Color4f> = stops
                .iter()
                .map(|&v| sk::Color4f::from(gradient_color(g, v)))
                .collect();
            let gc = sk::gradient::Colors::new(&colors, Some(&stops), sk::TileMode::Clamp, None);
            let grad = sk::gradient::Gradient::new(gc, sk::gradient::Interpolation::default());
            paint.set_shader(sk::shaders::linear_gradient(
                ((rect.left, rect.bottom), (rect.left, rect.top)),
                &grad,
                None,
            ));
            let mut glow = Paint::default();
            glow.set_anti_alias(true).set_color(gradient_color(g, 0.5));
            glow.set_alpha_f(
                line.material_floats
                    .get("_GlowIntensity")
                    .copied()
                    .unwrap_or(0.5)
                    .clamp(0., 1.)
                    * 0.3,
            );
            glow.set_mask_filter(sk::MaskFilter::blur(
                sk::BlurStyle::Normal,
                line.glow_range_scale * 2.,
                None,
            ));
            c.draw_path(&path, &glow);
        }
        c.draw_path(&path, &paint);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NotePart {
    Body,
    Arrow,
    Mark,
}
pub struct PreviewNote<'a> {
    pub role: &'a str,
    pub width_lanes: f32,
    pub center: (f32, f32),
    pub lane_px: f32,
    pub body_height: f32,
    pub arrow_height: f32,
    pub critical: bool,
    pub native_critical: bool,
    pub strict_assets: bool,
}
pub fn select_arrow(arrows: &[Arrow], width: f32) -> Option<&Arrow> {
    arrows
        .iter()
        .find(|a| width < a.max_width)
        .or_else(|| arrows.last())
}

#[derive(Debug, Serialize)]
pub struct DrawInfo {
    pub role: String,
    pub width_lanes: f32,
    pub critical: bool,
    pub arrow: Option<String>,
    pub empty_arrow: bool,
    pub cap_scale: f32,
}
#[derive(Debug)]
pub struct CapLayout {
    pub scale: f32,
    pub left_center: f32,
    pub right_center: f32,
    pub main_width: f32,
    pub main_center: f32,
}
pub fn cap_layout(width: f32, left: f32, right: f32, lo: f32, ro: f32) -> CapLayout {
    // Fitting is a preview policy. Scale the complete caps equally and suppress
    // overhang for the degenerate case so they meet without negative fill.
    let inner = left + right - lo - ro;
    let scale = if width < left + right || width < inner {
        (width / (left + right).max(inner)).clamp(0., 1.)
    } else {
        1.
    };
    let (lo, ro) = if scale < 1. { (0., 0.) } else { (lo, ro) };
    let l = left * scale;
    let r = right * scale;
    CapLayout {
        scale,
        left_center: -width / 2. + l / 2. - lo,
        right_center: width / 2. - r / 2. + ro,
        main_width: (width - l - r + lo + ro).max(0.),
        main_center: (l - r + ro - lo) / 2.,
    }
}
/// Three intervals [source start,end,destination start,end]. Applied on both axes.
pub fn slice_axis(source: f32, start: f32, end: f32, target: f32, scale: f32) -> [[f32; 4]; 3] {
    let a = start * scale;
    let b = end * scale;
    let fit = if a + b > target { target / (a + b) } else { 1. };
    [
        [0., start, 0., a * fit],
        [start, source - end, a * fit, target - b * fit],
        [source - end, source, target - b * fit, target],
    ]
}
pub(crate) fn rgba(rgb: [f32; 3], a: f32) -> Color {
    Color::from_argb(
        (a.clamp(0., 1.) * 255.).round() as u8,
        (rgb[0].clamp(0., 1.) * 255.).round() as u8,
        (rgb[1].clamp(0., 1.) * 255.).round() as u8,
        (rgb[2].clamp(0., 1.) * 255.).round() as u8,
    )
}
fn lerp_keys(keys: &[(f32, f32)], v: f32) -> f32 {
    if v <= keys[0].0 {
        return keys[0].1;
    }
    for pair in keys.windows(2) {
        if v <= pair[1].0 {
            let t = (v - pair[0].0) / (pair[1].0 - pair[0].0);
            return pair[0].1 + (pair[1].1 - pair[0].1) * t;
        }
    }
    keys.last().unwrap().1
}
pub(crate) fn gradient_color(g: &Gradient, v: f32) -> Color {
    let rgb = std::array::from_fn(|i| {
        lerp_keys(
            &g.colors
                .iter()
                .map(|k| (k.at, k.rgb[i]))
                .collect::<Vec<_>>(),
            v,
        )
    });
    let a = lerp_keys(
        &g.alphas.iter().map(|k| (k.at, k.alpha)).collect::<Vec<_>>(),
        v,
    );
    rgba(rgb, a)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrow_threshold_is_strict_and_null_is_preserved() {
        let a = vec![
            Arrow {
                max_width: 5.,
                sprite: Some("small".into()),
            },
            Arrow {
                max_width: 13.,
                sprite: None,
            },
            Arrow {
                max_width: 99.,
                sprite: Some("large".into()),
            },
        ];
        assert_eq!(
            select_arrow(&a, 4.999).unwrap().sprite.as_deref(),
            Some("small")
        );
        assert!(select_arrow(&a, 5.).unwrap().sprite.is_none());
        assert!(select_arrow(&a, 12.999).unwrap().sprite.is_none());
        assert_eq!(
            select_arrow(&a, 13.).unwrap().sprite.as_deref(),
            Some("large")
        );
        assert_eq!(
            select_arrow(&a, 100.).unwrap().sprite.as_deref(),
            Some("large")
        );
    }
    #[test]
    fn asymmetric_overhang_keeps_piece_edges_together() {
        let a = cap_layout(4., 0.7, 0.5, 0.15, 0.05);
        assert!((a.main_width - 3.).abs() < 1e-6);
        assert!((a.left_center + 0.7 / 2. - (a.main_center - a.main_width / 2.)).abs() < 1e-6);
        assert!((a.right_center - 0.5 / 2. - (a.main_center + a.main_width / 2.)).abs() < 1e-6);
    }
    #[test]
    fn narrow_caps_shrink_equally() {
        let a = cap_layout(0.1, 0.7, 0.5, 0.15, 0.05);
        assert!(a.scale > 0. && a.scale < 1.);
        assert!(a.main_width < 1e-6);
        assert!(
            (a.left_center + 0.7 * a.scale / 2. - (a.right_center - 0.5 * a.scale / 2.)).abs()
                < 1e-6
        );
        assert_eq!(cap_layout(0., 0.7, 0.5, 0., 0.).scale, 0.);
    }
    #[test]
    fn sliced_borders_shrink_before_inverting() {
        let a = slice_axis(10., 4., 4., 2., 1.);
        assert_eq!(a, [[0., 4., 0., 1.], [4., 6., 1., 1.], [6., 10., 1., 2.]]);
    }
    #[cfg(feature = "research-tests")]
    #[test]
    fn packs_have_expected_sprites_and_one_explicit_gap() -> Result<()> {
        for (name, count) in [("skin001", 140), ("skin002", 139), ("skin003", 145)] {
            let s = Skin::load(
                &std::path::PathBuf::from(
                    std::env::var_os("MOENOTES_TEST_PACKS").expect("Set MOENOTES_TEST_PACKS"),
                )
                .join(format!("{name}/skin.json")),
            )?;
            assert_eq!(s.manifest.sprites.len(), count);
            let a = select_arrow(&s.manifest.notes["flick_right"].arrows, 12.).unwrap();
            assert_eq!(a.sprite.is_none(), name == "skin002");
            let tap = &s.manifest.sprites[&s.manifest.notes["tap"].main];
            assert_eq!(tap.border, [4., 0., 4., 0.]);
        }
        Ok(())
    }

    #[cfg(feature = "research-tests")]
    #[test]
    fn selection_matches_recovered_native_at_adjacent_float_thresholds() -> Result<()> {
        let root = std::path::PathBuf::from(
            std::env::var_os("MOENOTES_TEST_PACKS").expect("Set MOENOTES_TEST_PACKS"),
        );
        let report: serde_json::Value = serde_json::from_slice(&fs::read(
            std::env::var_os("MOENOTES_TEST_ARROW_ORACLE").expect("Set MOENOTES_TEST_ARROW_ORACLE"),
        )?)?;
        let skins: BTreeMap<_, _> = ["skin001", "skin002", "skin003"]
            .into_iter()
            .map(|name| Skin::load(&root.join(format!("{name}/skin.json"))).map(|s| (name, s)))
            .collect::<Result<_>>()?;
        let cases = report["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 237);
        for case in cases {
            let s = &skins[case["skin"].as_str().unwrap()];
            let n = &s.manifest.notes[case["role"].as_str().unwrap()];
            let arrow = select_arrow(&n.arrows, case["width"].as_f64().unwrap() as f32).unwrap();
            assert_eq!(arrow.sprite.as_deref(), case["sprite"].as_str(), "{case}");
        }
        Ok(())
    }
}
