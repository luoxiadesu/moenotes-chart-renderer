use moenotes_chart_renderer::api::{ErrorKind, Metadata, RenderOptions, Renderer};
const CHART: &[u8] = include_bytes!("fixtures/synthetic.json");
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
