use anyhow::{Context, Result, ensure};
use moenotes_chart_renderer::{DrawInfo, ROLES, Skin, sha256};
use serde_json::{Value, json};
use skia_safe::{self as sk, Canvas, Color, Data, Font, FontMgr, Paint, Rect, Surface};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const BG: Color = Color::from_rgb(16, 22, 33);
const PANEL: Color = Color::from_rgb(23, 32, 46);
const FG: Color = Color::from_rgb(224, 231, 241);
const MUTED: Color = Color::from_rgb(142, 159, 180);
const WARN: Color = Color::from_rgb(255, 184, 94);
fn paint(color: Color) -> Paint {
    let mut p = Paint::default();
    p.set_color(color).set_anti_alias(true);
    p
}
fn text(c: &Canvas, font: &Font, value: &str, x: f32, y: f32, size: f32, color: Color) {
    let mut f = font.clone();
    f.set_size(size);
    c.draw_str(value, (x, y), &f, &paint(color));
}
fn surface(w: i32, h: i32) -> Result<Surface> {
    ensure!(
        w > 0 && h > 0 && (w as u64) * (h as u64) <= 16_000_000,
        "Sample exceeds 64 MB pixel budget"
    );
    let mut s = sk::surfaces::raster_n32_premul((w, h)).context("Cannot allocate CPU surface")?;
    s.canvas().clear(BG);
    Ok(s)
}
fn save(mut s: Surface, path: &Path) -> Result<Value> {
    let image = s.image_snapshot();
    let data = image
        .encode(None, sk::EncodedImageFormat::PNG, 100)
        .context("PNG encode failed")?;
    fs::write(path, data.as_bytes())?;
    Ok(
        json!({"file":path.file_name().unwrap().to_string_lossy(),"width":image.width(),"height":image.height(),"sha256":sha256(data.as_bytes())}),
    )
}
fn baseline(c: &Canvas, x: f32, y: f32, w: f32) {
    let mut p = paint(Color::from_rgb(51, 67, 86));
    p.set_stroke_width(1.);
    c.draw_line((x - w / 2., y), (x + w / 2., y), &p);
    c.draw_line((x, y - 7.), (x, y + 7.), &p);
}
fn overview(skin: &Skin, font: &Font, out: &Path) -> Result<(Value, Vec<DrawInfo>)> {
    let mut s = surface(1800, 1560)?;
    let c = s.canvas();
    let mut records = vec![];
    text(
        c,
        font,
        &format!("{}  /  FLAT NOTE SKIN", skin.manifest.skin),
        32.,
        46.,
        28.,
        FG,
    );
    text(
        c,
        font,
        "Original sprites / zero tilt / CPU Skia / upper: normal, lower: critical",
        32.,
        77.,
        16.,
        MUTED,
    );
    let widths = [0.25, 1., 3., 6., 12., 24.];
    for (j, w) in widths.iter().enumerate() {
        text(
            c,
            font,
            &format!("{w} lanes"),
            245. + j as f32 * 260.,
            112.,
            16.,
            MUTED,
        );
    }
    for (i, role) in ROLES.iter().enumerate() {
        let y = 132. + i as f32 * 138.;
        c.draw_rect(Rect::from_xywh(20., y, 1760., 128.), &paint(PANEL));
        text(c, font, role, 34., y + 35., 18., FG);
        for (j, w) in widths.iter().enumerate() {
            let x = 295. + j as f32 * 260.;
            for (critical, dy) in [(false, 52.), (true, 103.)] {
                baseline(c, x, y + dy, 180.);
                let info = skin.draw_note(c, role, *w, (x, y + dy), 14., critical)?;
                if info.empty_arrow {
                    text(
                        c,
                        font,
                        "EMPTY ARROW SLOT",
                        x - 82.,
                        y + dy - 31.,
                        11.,
                        WARN,
                    );
                }
                records.push(info);
            }
        }
    }
    text(c, font, "LINE MATERIAL STUDIES", 32., 1270., 19., FG);
    text(
        c,
        font,
        "Skin gradients and guide color; screen-space glow is approximate. Guide has no endpoint glyph.",
        32.,
        1296.,
        15.,
        MUTED,
    );
    for (i, kind) in ["normal", "pressed", "disabled", "guide"]
        .iter()
        .enumerate()
    {
        let x = 70. + i as f32 * 430.;
        text(c, font, kind, x, 1330., 17., FG);
        skin.draw_band(c, Rect::from_xywh(x, 1345., 220., 150.), kind)?;
    }
    text(
        c,
        font,
        "Widths are semantic lanes. Static prefab arrow transforms; animation and perspective disabled.",
        32.,
        1536.,
        14.,
        MUTED,
    );
    Ok((
        save(s, &out.join(format!("{}-overview.png", skin.manifest.skin)))?,
        records,
    ))
}
fn boundaries(skin: &Skin, font: &Font, out: &Path) -> Result<(Value, Vec<DrawInfo>)> {
    let mut s = surface(1800, 1270)?;
    let c = s.canvas();
    let mut records = vec![];
    text(
        c,
        font,
        &format!("{}  /  ARROW THRESHOLD BOUNDARIES", skin.manifest.skin),
        30.,
        43.,
        26.,
        FG,
    );
    text(
        c,
        font,
        "Triplets: threshold - 0.001 / equal / + 0.001 lanes. Equality selects the next slot.",
        30.,
        74.,
        16.,
        MUTED,
    );
    let thresholds = [5., 7., 10., 12., 13., 16., 18., 19., 21.];
    for (j, role) in ["flick", "flick_left", "flick_right"].iter().enumerate() {
        text(c, font, role, 190. + j as f32 * 570., 110., 19., FG);
        for (i, t) in thresholds.iter().enumerate() {
            let y = 135. + i as f32 * 121.;
            let x = 100. + j as f32 * 570.;
            c.draw_rect(Rect::from_xywh(x, y, 545., 111.), &paint(PANEL));
            if j == 0 {
                text(c, font, &format!("{t}"), 30., y + 58., 20., MUTED);
            }
            for (k, offset) in [-0.001, 0., 0.001].iter().enumerate() {
                let px = x + 90. + k as f32 * 180.;
                let w = t + offset;
                baseline(c, px, y + 66., 140.);
                let info = skin.draw_note(c, role, w, (px, y + 66.), 9., false)?;
                text(c, font, &format!("{w:.3}"), px - 25., y + 96., 12., MUTED);
                if info.empty_arrow {
                    text(c, font, "EMPTY", px - 22., y + 23., 13., WARN);
                }
                records.push(info);
            }
        }
    }
    Ok((
        save(
            s,
            &out.join(format!("{}-boundaries.png", skin.manifest.skin)),
        )?,
        records,
    ))
}
fn crop_study(skin: &Skin, font: &Font, packs: &Path, out: &Path) -> Result<Value> {
    let mut s = surface(1440, 1060)?;
    let c = s.canvas();
    text(
        c,
        font,
        &format!("{}  /  CROP AND PIVOT AUDIT", skin.manifest.skin),
        30.,
        44.,
        26.,
        FG,
    );
    text(
        c,
        font,
        "Blue: original rectangle. Orange: decoded mesh bounds. Cross: original pivot.",
        30.,
        76.,
        16.,
        MUTED,
    );
    for (i, role) in ROLES.iter().enumerate() {
        let n = &skin.manifest.notes[*role];
        let p = n.parts.iter().find(|p| p.tilt == 0).unwrap();
        let chosen = [
            Some(&n.main),
            p.left.as_ref(),
            p.right.as_ref(),
            n.mark.as_ref(),
            n.arrows.first().and_then(|a| a.sprite.as_ref()),
        ];
        let y = 104. + i as f32 * 115.;
        text(c, font, role, 20., y + 55., 15., FG);
        for (j, id) in chosen.iter().enumerate() {
            let x = 260. + j as f32 * 250.;
            c.draw_rect(Rect::from_xywh(x - 115., y, 234., 105.), &paint(PANEL));
            if let Some(id) = id {
                let a = &skin.manifest.sprites[*id];
                let u = (160. / a.rect.width).min(65. / a.rect.height) * a.pixels_per_unit;
                let px = x;
                let py = y + 60.;
                let rw = a.rect.width / a.pixels_per_unit * u;
                let rh = a.rect.height / a.pixels_per_unit * u;
                let original =
                    Rect::from_xywh(px - rw * a.pivot[0], py - rh * (1. - a.pivot[1]), rw, rh);
                let b = a.bounds_units;
                let crop = Rect::new(px + b[0] * u, py - b[3] * u, px + b[2] * u, py - b[1] * u);
                let image = sk::Image::from_encoded(Data::new_copy(&fs::read(
                    packs.join(&skin.manifest.skin).join(&a.file),
                )?))
                .context("Audit image decode")?;
                c.draw_image_rect_with_sampling_options(
                    &image,
                    None,
                    crop,
                    sk::FilterMode::Linear,
                    &paint(Color::WHITE),
                );
                let mut outline = paint(Color::from_rgb(84, 164, 240));
                outline
                    .set_style(sk::paint::Style::Stroke)
                    .set_stroke_width(1.);
                c.draw_rect(original, &outline);
                outline.set_color(WARN);
                c.draw_rect(crop, &outline);
                baseline(c, px, py, 14.);
                let label = match j {
                    0 => "main / sliced",
                    1 => "left cap",
                    2 => "right cap",
                    3 => "center mark",
                    _ => "first arrow",
                };
                text(c, font, label, x - 60., y + 18., 12., MUTED);
            }
        }
    }
    save(
        s,
        &out.join(format!("{}-crop-audit.png", skin.manifest.skin)),
    )
}
fn main() -> Result<()> {
    let mut args: Vec<_> = env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--version") {
        println!("moenotes-chart-renderer {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.is_empty() {
        print!("{}", moenotes_chart_renderer::cli::help());
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--help" || a == "-h") {
        print!("{}", moenotes_chart_renderer::cli::help());
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|a| a == "render" || a == "inspect")
    {
        return moenotes_chart_renderer::cli::run(&args);
    }
    if args.first().is_some_and(|a| a == "samples") {
        args.remove(0);
    }
    ensure!(
        args.len() <= 2,
        "usage: moenotes-chart-renderer [PACK_DIRECTORY] [OUTPUT_DIRECTORY]"
    );

    let packs = args.first().map(PathBuf::from).map(Ok).unwrap_or_else(|| {
        moenotes_chart_renderer::resources::pack_directory(None)?
            .context("samples requires a game skin pack directory")
    })?;
    let out = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples"));
    fs::create_dir_all(&out)?;
    let pack_manifest: Value = serde_json::from_slice(&fs::read(packs.join("manifest.json"))?)?;
    let font_bytes = fs::read(packs.join("fonts/DejaVuSans.ttf"))?;
    ensure!(
        sha256(&font_bytes)
            == pack_manifest["font"]["sha256"]
                .as_str()
                .context("Font hash missing")?,
        "Font changed"
    );
    let face = FontMgr::new()
        .new_from_data(Data::new_copy(&font_bytes), None)
        .context("Cannot load pinned font")?;
    let font = Font::new(face, 16.);
    let mut images = vec![];
    let mut records = vec![];
    for name in ["skin001", "skin002", "skin003"] {
        let skin = Skin::load(&packs.join(name).join("skin.json"))?;
        let (v, r) = overview(&skin, &font, &out)?;
        images.push(v);
        records.push(json!({"skin":name,"overview":r}));
        let (v, r) = boundaries(&skin, &font, &out)?;
        images.push(v);
        records.push(json!({"skin":name,"boundaries":r}));
        images.push(crop_study(&skin, &font, &packs, &out)?);
        println!("Rendered {name}");
    }
    let mut combined = surface(1800, 520)?;
    for (i, name) in ["skin001", "skin002", "skin003"].iter().enumerate() {
        let image = sk::Image::from_encoded(Data::new_copy(&fs::read(
            out.join(format!("{name}-overview.png")),
        )?))
        .context("Cannot read overview")?;
        combined.canvas().draw_image_rect_with_sampling_options(
            image,
            None,
            Rect::from_xywh(i as f32 * 600., 0., 600., 520.),
            sk::FilterMode::Linear,
            &paint(Color::WHITE),
        );
    }
    images.push(save(combined, &out.join("comparison.png"))?);
    let report = json!({"renderer":"skia-safe 0.153.3 / CPU raster","images":images,"cases":records,
        "font_sha256":sha256(&font_bytes),"policies":{"tilt":0,"arrow_threshold":"strict less-than","empty_arrow":"omit and report; no substitution","lane_local_units":0.5,"glow":"approximate screen-space blur","animation":"disabled; prefab static transforms","critical":"black center mark at 2x","narrow_caps":"equal scale, no negative center"}});
    fs::write(
        out.join("render-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}
