use crate::{
    Skin,
    layout::{Layout, Options},
    parser::Score,
    render,
    scene::Scene,
};
use anyhow::{Context, Result, bail, ensure};
use std::{ffi::OsString, fs, path::PathBuf};

pub fn help() -> &'static str {
    "moenotes-chart-renderer render CHART -o IMAGE.png [OPTIONS]\n  --skin builtin|skin001|skin002|skin003  --packs DIR  --mirror\n  --theme white|black (white), legacy print|dark aliases\n  --flick-layout callout|inline (callout)\n  --target-beats N (24)  --bars-per-column N (explicit override)\n  --pixels-per-beat N (64) --pixels-per-lane N (10)\n  --note-height N (8) --arrow-height N (10) --supersample 1|2|3 (2)\n  --long (single column; default is a complete multi-column image)\n  --curve-mode musical|native-parameters --strict-assets --native-critical --fixed-spacing\n  --title TEXT --difficulty TEXT --level TEXT --artist TEXT --author TEXT --cover IMAGE --metadata JSON\n  --masterdata SNAPSHOT_DIR --chart-key KEY --language ja|en|zh-Hant|zh-Hans|ko --assets BY_KEY_DIR\n  --scene-json FILE --output-scale 0.25..4 (1)\nmoenotes-chart-renderer inspect CHART [-o REPORT.json] [--mirror]\nmoenotes-chart-renderer samples [PACK_DIRECTORY] [OUTPUT_DIRECTORY]\n"
}
pub fn run(args: &[OsString]) -> Result<()> {
    let command = args[0].to_str().context("Command must be UTF-8")?;
    let input = PathBuf::from(args.get(1).context("Chart filename required")?);
    let mut output = None;
    let mut packs: Option<PathBuf> = None;
    let mut skin = "builtin".to_string();
    let mut mirror = false;
    let mut options = Options::default();
    let mut metadata = render::Metadata::default();
    let mut meta_path = None;
    let mut title = None;
    let mut difficulty = None;
    let mut level = None;
    let mut artist = None;
    let mut scene_path = None;
    let mut masterdata = None;
    let mut chart_key = None;
    let mut language = "ja".to_string();
    let mut assets = None;
    let mut cover_path = None;
    let mut author = None;
    let mut timings = false;
    let mut i = 2;
    while i < args.len() {
        let flag = args[i].to_str().context("Option must be UTF-8")?;
        ensure!(
            command != "inspect" || ["--mirror", "-o", "--output"].contains(&flag),
            "inspect does not accept {flag}"
        );
        if flag == "--timings" {
            timings = true;
            i += 1;
            continue;
        }
        if flag == "--mirror" {
            mirror = true;
            i += 1;
            continue;
        }
        if flag == "--fixed-spacing" {
            options.auto_spacing = false;
            i += 1;
            continue;
        }
        if flag == "--strict-assets" {
            options.strict_assets = true;
            i += 1;
            continue;
        }
        if flag == "--native-critical" {
            options.native_critical = true;
            i += 1;
            continue;
        }
        if flag == "--long" {
            options.long = true;
            i += 1;
            continue;
        }
        i += 1;
        let value = args
            .get(i)
            .with_context(|| format!("Missing value for {flag}"))?;
        let string = || value.to_str().context("Option value must be UTF-8");
        match flag {
            "-o" | "--output" => output = Some(PathBuf::from(value)),
            "--packs" => packs = Some(PathBuf::from(value)),
            "--skin" => skin = string()?.into(),
            "--bars-per-column" => {
                options.bars_per_column = string()?.parse()?;
                ensure!(
                    options.bars_per_column > 0,
                    "bars-per-column must be positive"
                );
            }
            "--target-beats" => options.target_beats = string()?.parse()?,
            "--theme" => {
                options.theme = match string()? {
                    "white" | "print" => crate::layout::Theme::Print,
                    "black" => crate::layout::Theme::Black,
                    "dark" => crate::layout::Theme::Dark,
                    _ => bail!("Theme must be white, black, or legacy dark"),
                }
            }
            "--flick-layout" => {
                options.flick_layout = match string()? {
                    "callout" => crate::layout::FlickLayout::Callout,
                    "inline" => crate::layout::FlickLayout::Inline,
                    _ => bail!("Flick layout must be callout or inline"),
                }
            }
            "--note-height" => options.note_height = string()?.parse()?,
            "--arrow-height" => options.arrow_height = string()?.parse()?,
            "--output-scale" => options.output_scale = string()?.parse()?,
            "--supersample" => options.supersample = string()?.parse()?,
            "--curve-mode" => {
                options.curve_mode = match string()? {
                    "musical" => crate::layout::CurveMode::Musical,
                    "native-parameters" => crate::layout::CurveMode::NativeParameters,
                    _ => bail!("Unknown curve mode"),
                }
            }
            "--metadata" => meta_path = Some(PathBuf::from(value)),
            "--masterdata" => masterdata = Some(PathBuf::from(value)),
            "--chart-key" => chart_key = Some(string()?.to_owned()),
            "--language" => language = string()?.to_owned(),
            "--assets" => assets = Some(PathBuf::from(value)),
            "--cover" => cover_path = Some(PathBuf::from(value)),
            "--author" => author = Some(string()?.to_owned()),
            "--difficulty" => difficulty = Some(string()?.to_owned()),
            "--level" => level = Some(string()?.to_owned()),
            "--artist" => artist = Some(string()?.to_owned()),
            "--pixels-per-beat" => options.pixels_per_beat = string()?.parse()?,
            "--pixels-per-lane" => options.pixels_per_lane = string()?.parse()?,
            "--title" => title = Some(string()?.to_string()),
            "--scene-json" => scene_path = Some(PathBuf::from(value)),
            _ => bail!("Unknown option: {flag}"),
        }
        i += 1;
    }
    ensure!(
        fs::metadata(&input)?.len() <= 64 * 1024 * 1024,
        "Input exceeds 64 MiB"
    );
    let total_started = std::time::Instant::now();
    let score = Score::parse(&fs::read(&input)?, mirror)?;
    let parse_seconds = total_started.elapsed().as_secs_f64();
    for path in output.iter().chain(scene_path.iter()) {
        ensure!(
            !path.exists() || path.canonicalize()? != input.canonicalize()?,
            "Output must not overwrite input chart"
        );
    }
    if command == "inspect" {
        let value = serde_json::json!({"parser_version":"0.3.0","mirror":mirror,"notes":score.notes,"branches":score.branches,"bpms":score.bpms,"signatures":score.signatures,"events":score.events,"warnings":score.warnings,"statistics":score.statistics});
        let data = serde_json::to_vec_pretty(&value)?;
        if let Some(out) = output {
            if let Some(p) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(p)?;
            }
            fs::write(out, data)?;
        } else {
            println!("{}", String::from_utf8(data)?);
        }
        return Ok(());
    }
    ensure!(command == "render", "Unknown command {command}");
    let output = output.context("render requires -o IMAGE.png")?;
    if let Some(path) = &scene_path {
        ensure!(
            path != &output && path != &output.with_extension("render.json"),
            "Scene output conflicts with PNG/report output"
        );
    }
    ensure!(
        output.extension().is_some_and(|x| x == "png"),
        "Output must have .png extension"
    );
    ensure!(
        ["builtin", "skin001", "skin002", "skin003"].contains(&skin.as_str()),
        "Unknown skin"
    );
    // Resolve cover precedence before asking masterdata to locate a cover.
    // An explicit null in the JSON overlay intentionally removes that cover.
    let overlay: Option<serde_json::Value> = meta_path
        .as_ref()
        .map(|path| -> Result<_> {
            let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
            ensure!(value.is_object(), "Metadata must be an object");
            Ok(value)
        })
        .transpose()?;
    let cover_overridden =
        cover_path.is_some() || overlay.as_ref().is_some_and(|v| v.get("cover").is_some());
    if let Some(dir) = &masterdata {
        let key = chart_key
            .as_deref()
            .context("--masterdata requires --chart-key")?;
        metadata = crate::metadata::resolve(
            dir,
            key,
            &language,
            if cover_overridden {
                None
            } else {
                assets.as_deref()
            },
        )?
        .metadata;
    }
    if let (Some(path), Some(overlay)) = (&meta_path, overlay) {
        let mut base = serde_json::to_value(&metadata)?;
        for (k, v) in overlay.as_object().context("Metadata must be an object")? {
            base[k] = v.clone();
        }
        metadata = serde_json::from_value(base)?;
        if overlay.get("cover").is_some()
            && let Some(cover) = &metadata.cover
        {
            let p = PathBuf::from(cover);
            if p.is_relative() {
                metadata.cover = Some(
                    path.parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join(p)
                        .to_string_lossy()
                        .into(),
                );
            }
        }
    }
    if let Some(p) = cover_path {
        metadata.cover = Some(p.to_string_lossy().into());
    }
    if let Some(v) = author {
        metadata.author = v;
    }
    if let Some(v) = title {
        metadata.title = v;
    }
    if metadata.title.is_empty() {
        metadata.title = input
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
    }
    if let Some(v) = difficulty {
        metadata.difficulty = v;
    }
    if let Some(v) = level {
        metadata.level = v;
    }
    if let Some(v) = artist {
        metadata.artist = v;
    }
    let scene_started = std::time::Instant::now();
    let model = Scene::build(&score, Layout::build(&score, options)?)?;
    let scene_seconds = scene_started.elapsed().as_secs_f64();
    let skin = if skin == "builtin" {
        Skin::builtin()?
    } else {
        let pack_dir = crate::resources::pack_directory(packs.as_deref())?;
        Skin::load(
            &pack_dir
                .context("Game skin requires --packs or MOENOTES_ASSETS_DIR")?
                .join(&skin)
                .join("skin.json"),
        )?
    };
    let font = crate::typography::Fonts::builtin()?;
    let cover_bytes = metadata
        .cover
        .as_ref()
        .map(|path| -> Result<Vec<u8>> {
            ensure!(
                fs::metadata(path)?.len() <= 16 * 1024 * 1024,
                "Cover exceeds 16 MiB"
            );
            Ok(fs::read(path)?)
        })
        .transpose()?;
    let render_started = std::time::Instant::now();
    let mut rendered = render::render_memory(
        &model,
        &skin,
        &font,
        &metadata,
        mirror,
        output
            .file_name()
            .context("Output name")?
            .to_str()
            .context("Output name UTF-8")?,
        cover_bytes.as_deref(),
    )?;
    let render_seconds = render_started.elapsed().as_secs_f64();
    let report = &mut rendered.report;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let mut files: Vec<(String, Vec<u8>)> = rendered
        .pages
        .into_iter()
        .map(|p| (p.name, p.png))
        .collect();
    if let Some(path) = scene_path {
        let scene_parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        fs::create_dir_all(parent)?;
        fs::create_dir_all(scene_parent)?;
        ensure!(
            parent.canonicalize()? == scene_parent.canonicalize()?,
            "--scene-json must be in the output directory for group publication"
        );
        files.push((
            path.file_name()
                .context("Scene filename")?
                .to_string_lossy()
                .into(),
            serde_json::to_vec_pretty(&model)?,
        ));
    }
    files.push((
        output
            .with_extension("render.json")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into(),
        serde_json::to_vec_pretty(report)?,
    ));
    let mut protected = vec![input];
    if let Some(p) = meta_path {
        protected.push(p);
    }
    if let Some(p) = &metadata.cover {
        protected.push(PathBuf::from(p));
    }
    let publish_started = std::time::Instant::now();
    crate::output::publish(&output, files, &protected)?;
    if timings {
        eprintln!(
            "TIMINGS {}",
            serde_json::json!({"parse_seconds":parse_seconds,"scene_seconds":scene_seconds,"render_encode_seconds":render_seconds,"publish_seconds":publish_started.elapsed().as_secs_f64(),"total_seconds":total_started.elapsed().as_secs_f64()})
        );
    }
    println!(
        "Rendered {} notes, {} branches, {} columns into {} PNG(s)",
        report.glyphs,
        report.branches,
        report.columns,
        report.images.len()
    );
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    Ok(())
}
