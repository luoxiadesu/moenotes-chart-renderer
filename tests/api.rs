use moenotes_chart_renderer::api::{ErrorKind, Metadata, RenderOptions, Renderer};
const CHART: &[u8] = include_bytes!("fixtures/synthetic.json");
#[test]
fn large_logical_sheet_can_be_exported_below_pixel_budget() {
    let chart = br#"{"events":{},"notes":[{"t":690720,"pos":4,"size":6}]}"#;
    let renderer = Renderer::builtin().unwrap();
    let options = RenderOptions {
        pixels_per_beat: 160.,
        auto_spacing: false,
        output_scale: 0.5,
        supersample: 1,
        ..RenderOptions::default()
    };
    let metadata = Metadata {
        title: "Large logical sheet".into(),
        ..Metadata::default()
    };
    let result = renderer
        .render(
            chart,
            options.clone(),
            metadata.clone(),
            false,
            "large.png",
            None,
        )
        .unwrap();
    assert_eq!(result.pages.len(), 1);
    let image = &result.report.images[0];
    assert!(image.logical_width as i64 * image.logical_height as i64 > 64_000_000);
    assert_eq!(image.width, 10206);
    assert_eq!(
        image.height,
        (image.logical_height as f64 * 0.5).ceil() as i32
    );
    assert_eq!(result.report.glyphs, 1);
    let error = renderer
        .render(
            chart,
            RenderOptions {
                output_scale: 1.,
                ..options
            },
            metadata,
            false,
            "large.png",
            None,
        )
        .err()
        .unwrap();
    assert_eq!(error.kind, ErrorKind::Layout);
    assert!(error.message.contains("Scaled PNG exceeds"));
}
#[test]
fn dense_flick_callouts_preserve_counts_and_separate_arrows() {
    use moenotes_chart_renderer::api::{FlickLayout, Theme};
    let chart=br#"{"events":{},"notes":[{"type":"flick","dir":"right","t":480,"pos":4,"size":8},{"type":"flick","dir":"left","t":510,"pos":4,"size":8},{"type":"tap","t":540,"pos":4,"size":8}]}"#;
    let renderer = Renderer::builtin().unwrap();
    let mut counts = vec![];
    for theme in [Theme::Print, Theme::Black] {
        for mode in [FlickLayout::Inline, FlickLayout::Callout] {
            let r = renderer
                .render(
                    chart,
                    RenderOptions {
                        theme,
                        flick_layout: mode,
                        supersample: 1,
                        ..RenderOptions::default()
                    },
                    Metadata {
                        title: "Dense Flick".into(),
                        ..Metadata::default()
                    },
                    false,
                    "dense.png",
                    None,
                )
                .unwrap()
                .report;
            counts.push(r.statistics.reconstructed_full_combo);
            assert!(r.inline_arrow_body_box_overlaps > 0);
            if mode == FlickLayout::Callout {
                assert_eq!(r.arrow_body_box_overlaps, 0);
                assert_eq!(r.flick_callouts.len(), 2);
                assert!(r.unresolved_flick_note_ids.is_empty());
                assert_ne!(r.flick_callouts[0].offset_x, r.flick_callouts[1].offset_x);
            } else {
                assert!(r.arrow_body_box_overlaps > 0);
                assert!(r.flick_callouts.is_empty());
            }
        }
    }
    assert!(counts.iter().all(|n| *n == counts[0]));
}

#[test]
fn export_scale_changes_pixels_without_reflowing_chart() {
    let renderer = Renderer::builtin().unwrap();
    let mut results = vec![];
    for scale in [0.5, 1., 2.] {
        let result = renderer
            .render(
                CHART,
                RenderOptions {
                    output_scale: scale,
                    supersample: 1,
                    ..RenderOptions::default()
                },
                Metadata {
                    title: "Export".into(),
                    ..Metadata::default()
                },
                false,
                "scale.png",
                None,
            )
            .unwrap();
        let image = &result.report.images[0];
        assert_eq!(
            image.width,
            (image.logical_width as f64 * scale).ceil() as i32
        );
        assert_eq!(
            image.height,
            (image.logical_height as f64 * scale).ceil() as i32
        );
        results.push((
            image.logical_width,
            image.logical_height,
            result.report.glyphs,
        ));
    }
    assert!(results.iter().all(|r| *r == results[0]));
}

#[test]
fn white_native_critical_flag_is_applied() {
    let renderer = Renderer::builtin().unwrap();
    let mut pngs = vec![];
    for native_critical in [false, true] {
        let r = renderer
            .render(
                CHART,
                RenderOptions {
                    native_critical,
                    supersample: 1,
                    ..RenderOptions::default()
                },
                Metadata {
                    title: "Critical".into(),
                    ..Metadata::default()
                },
                false,
                "c.png",
                None,
            )
            .unwrap();
        pngs.push(r.pages[0].png.clone());
    }
    assert_ne!(pngs[0], pngs[1]);
}

#[test]
fn long_credits_expand_header_without_moving_relative_note_positions() {
    let renderer = Renderer::builtin().unwrap();
    let chart=br#"{"events":{},"notes":[{"type":"flick","t":480,"pos":4,"size":8},{"type":"tap","t":510,"pos":4,"size":8}]}"#;
    let mut reports = vec![];
    for author in [String::new(), "作詞・作曲・編曲 藤井健太郎 ".repeat(10)] {
        reports.push(
            renderer
                .render(
                    chart,
                    RenderOptions {
                        supersample: 1,
                        ..RenderOptions::default()
                    },
                    Metadata {
                        title: "A long title テスト テスト テスト テスト".into(),
                        author,
                        ..Metadata::default()
                    },
                    false,
                    "header.png",
                    None,
                )
                .unwrap()
                .report,
        );
    }
    assert_eq!(reports[0].glyphs, reports[1].glyphs);
    assert_eq!(reports[0].columns, reports[1].columns);
    let delta = reports[1].chart_offset_y - reports[0].chart_offset_y;
    assert!(delta > 0.);
    assert!((reports[1].flick_callouts[0].y - reports[0].flick_callouts[0].y - delta).abs() < 0.01);
}
#[test]
fn print_and_dark_keep_the_same_chart_counts() {
    use moenotes_chart_renderer::api::Theme;
    let renderer = Renderer::builtin().unwrap();
    let mut reports = vec![];
    let mut pngs = vec![];
    for theme in [Theme::Print, Theme::Dark] {
        let result = renderer
            .render(
                CHART,
                RenderOptions {
                    theme,
                    supersample: 1,
                    ..RenderOptions::default()
                },
                Metadata {
                    title: "Theme parity".into(),
                    ..Metadata::default()
                },
                true,
                "sheet.png",
                None,
            )
            .unwrap();
        assert_eq!(result.pages.len(), 1);
        pngs.push(result.pages[0].png.clone());
        reports.push(result.report);
    }
    assert_ne!(pngs[0], pngs[1]);
    assert_eq!(reports[0].glyphs, reports[1].glyphs);
    assert_eq!(reports[0].branches, reports[1].branches);
    assert_eq!(
        reports[0].statistics.reconstructed_full_combo,
        reports[1].statistics.reconstructed_full_combo
    );
    assert_eq!(reports[0].images[0].width, reports[1].images[0].width);
}

#[test]
fn structural_overlaps_remain_visible_and_judgement_overlaps_still_warn() {
    let renderer = Renderer::builtin().unwrap();
    let chart = br#"{"events":{},"notes":[{"type":"long","node":[{"t":0,"pos":0,"size":6},{"t":480,"pos":0,"size":6},{"t":481,"pos":0,"size":6},{"t":1920,"pos":0,"size":6}]},{"t":960,"pos":12,"size":6},{"t":961,"pos":12,"size":6}]}"#;
    let result = renderer
        .render(
            chart,
            RenderOptions {
                supersample: 1,
                ..RenderOptions::default()
            },
            Metadata {
                title: "Dense connections".into(),
                ..Metadata::default()
            },
            false,
            "dense.png",
            None,
        )
        .unwrap();
    assert_eq!(result.report.structural_connection_overlaps, 1);
    assert_eq!(result.report.dense_body_overlaps, 2);
    assert!(
        result
            .report
            .warnings
            .iter()
            .any(|s| s.contains("Some body boxes still overlap"))
    );
    assert!(
        result
            .report
            .warnings
            .iter()
            .any(|s| s.contains("structural connection pairs"))
    );
}
#[test]
fn memory_render_is_self_contained_repeatable_and_counts_combos() {
    let renderer = Renderer::builtin().unwrap();
    let options = RenderOptions {
        pixels_per_beat: 24.,
        supersample: 1,
        ..RenderOptions::default()
    };
    let metadata = Metadata {
        title: "合成谱面 / テスト".into(),
        author: "Test author".into(),
        ..Metadata::default()
    };
    let a = renderer
        .render(
            CHART,
            options.clone(),
            metadata.clone(),
            false,
            "preview.png",
            Some(include_bytes!("fixtures/cover.png")),
        )
        .unwrap();
    let b = renderer
        .render(
            CHART,
            options,
            metadata,
            false,
            "preview.png",
            Some(include_bytes!("fixtures/cover.png")),
        )
        .unwrap();
    assert!(!a.pages.is_empty());
    assert_eq!(a.pages[0].png, b.pages[0].png);
    assert_eq!(&a.pages[0].png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(a.report.statistics.derived_combos > 0);
    assert_eq!(
        a.report.statistics.reconstructed_full_combo,
        a.report.statistics.source_judgements + a.report.statistics.derived_combos
    );
    assert_eq!(a.report.metadata.author, "Test author");
}
#[test]
fn errors_have_stable_categories() {
    let r = Renderer::builtin().unwrap();
    let error = r
        .render(
            b"bad",
            RenderOptions::default(),
            Metadata::default(),
            false,
            "x.png",
            None,
        )
        .err()
        .unwrap();
    assert_eq!(error.kind, ErrorKind::Input);
    let options = RenderOptions {
        note_height: f64::NAN,
        ..RenderOptions::default()
    };
    let e = r
        .render(
            CHART,
            options,
            Metadata {
                title: "Test".into(),
                ..Metadata::default()
            },
            false,
            "x.png",
            None,
        )
        .err()
        .unwrap();
    assert_eq!(e.kind, ErrorKind::Layout);
}
#[test]
fn metadata_roles_are_not_conflated() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/masterdata");
    let r = moenotes_chart_renderer::metadata::resolve(&root, "test/test_03", "en", None).unwrap();
    assert_eq!(r.metadata.title, "Synthetic Song");
    assert_eq!(r.metadata.artist, "Test Band");
    assert!(r.metadata.author.contains("Lyrics: Writer A"));
    assert!(r.metadata.author.contains("Music: Composer B"));
    assert!(r.metadata.author.contains("Arrangement: Arranger C"));
    assert_eq!(r.metadata.master_full_combo, Some(12));
}

#[test]
fn masterdata_rejects_unknown_language_and_ambiguous_key() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/masterdata");
    assert!(moenotes_chart_renderer::metadata::resolve(&root, "missing", "en", None).is_err());
    assert!(
        moenotes_chart_renderer::metadata::resolve(&root, "test/test_03", "unknown", None).is_err()
    );
}

#[test]
fn complete_chart_remains_one_image_and_pagination_is_rejected() {
    let renderer = Renderer::builtin().unwrap();
    let options = RenderOptions {
        bars_per_column: 1,
        pixels_per_beat: 24.,
        supersample: 1,
        ..RenderOptions::default()
    };
    let result = renderer
        .render(
            CHART,
            options.clone(),
            Metadata {
                title: "Whole sheet".into(),
                ..Metadata::default()
            },
            false,
            "sheet.png",
            None,
        )
        .unwrap();
    assert_eq!(result.pages.len(), 1);
    assert!(result.report.columns > 1);
    let invalid = RenderOptions {
        columns_per_page: 1,
        ..options
    };
    assert_eq!(
        renderer
            .render(
                CHART,
                invalid,
                Metadata {
                    title: "No pages".into(),
                    ..Metadata::default()
                },
                false,
                "sheet.png",
                None
            )
            .err()
            .unwrap()
            .kind,
        ErrorKind::Layout
    );
}

#[test]
fn title_symbols_from_masterdata_render_without_font_substitution() {
    let renderer = Renderer::builtin().unwrap();
    let result = renderer
        .render(
            CHART,
            RenderOptions {
                supersample: 1,
                ..RenderOptions::default()
            },
            Metadata {
                title: "Symbol II : 🜁 / Symbol IV : 🜃".into(),
                ..Metadata::default()
            },
            false,
            "symbols.png",
            None,
        )
        .unwrap();
    assert_eq!(
        result.report.metadata.title,
        "Symbol II : 🜁 / Symbol IV : 🜃"
    );
    assert_eq!(result.pages.len(), 1);
}

#[test]
fn live_masterdata_directory_follows_the_current_pointer() {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    let root = std::env::temp_dir().join(format!(
        "moenotes-current-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/masterdata");
    for c in ['a', 'b'] {
        let commit = c.to_string().repeat(40);
        let dir = root.join(&commit);
        fs::create_dir_all(&dir).unwrap();
        for entry in fs::read_dir(&fixture).unwrap() {
            let p = entry.unwrap().path();
            fs::copy(&p, dir.join(p.file_name().unwrap())).unwrap();
        }
        let mut p: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join("provenance.json")).unwrap()).unwrap();
        p["commit"] = commit.clone().into();
        fs::write(dir.join("provenance.json"), serde_json::to_vec(&p).unwrap()).unwrap();
        fs::write(
            root.join("current.json"),
            serde_json::to_vec(&serde_json::json!({"directory":commit,"commit":commit})).unwrap(),
        )
        .unwrap();
        let resolved =
            moenotes_chart_renderer::metadata::resolve(&root, "test/test_03", "en", None).unwrap();
        assert_eq!(resolved.metadata.provenance.unwrap()["commit"], commit);
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn metadata_table_hash_mismatch_is_rejected_before_joining() {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    let root = std::env::temp_dir().join(format!(
        "moenotes-hash-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/masterdata");
    for entry in fs::read_dir(fixture).unwrap() {
        let p = entry.unwrap().path();
        fs::copy(&p, root.join(p.file_name().unwrap())).unwrap();
    }
    fs::write(root.join("MasterText.json"), b"{\"_allData\":[]}").unwrap();
    let error =
        moenotes_chart_renderer::metadata::resolve(&root, "test/test_03", "en", None).unwrap_err();
    assert!(error.to_string().contains("hash mismatch"));
    fs::remove_dir_all(root).unwrap();
}
