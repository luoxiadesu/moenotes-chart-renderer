use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "moenotes-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_moenotes-chart-renderer"))
            .current_dir(&self.0)
            .args([
                "render",
                fixture("synthetic.json").to_str().unwrap(),
                "-o",
                "sheet.png",
                "--supersample",
                "1",
            ])
            .args(args)
            .output()
            .unwrap()
    }
    fn report(&self) -> Value {
        serde_json::from_slice(&fs::read(self.0.join("sheet.render.json")).unwrap()).unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn success(result: Output) {
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn explicit_cover_overrides_missing_master_cover_and_json_cover() {
    let w = Workspace::new();
    fs::write(w.0.join("metadata.json"), br#"{"cover":"missing.png"}"#).unwrap();
    success(w.run(&[
        "--masterdata",
        fixture("masterdata").to_str().unwrap(),
        "--chart-key",
        "test/test_03",
        "--assets",
        "missing-assets",
        "--metadata",
        "metadata.json",
        "--cover",
        fixture("cover.png").to_str().unwrap(),
    ]));
    assert_eq!(
        w.report()["metadata"]["cover"],
        fixture("cover.png").to_str().unwrap()
    );
    assert_eq!(w.report()["metadata"]["title"], "Synthetic Song");
}

#[test]
fn json_cover_is_relative_to_overlay_and_null_can_remove_master_cover() {
    let w = Workspace::new();
    fs::create_dir(w.0.join("overlay")).unwrap();
    fs::copy(fixture("cover.png"), w.0.join("overlay/cover.png")).unwrap();
    for cover in [json!("cover.png"), Value::Null] {
        fs::write(
            w.0.join("overlay/metadata.json"),
            serde_json::to_vec(&json!({"cover":cover})).unwrap(),
        )
        .unwrap();
        success(w.run(&[
            "--masterdata",
            fixture("masterdata").to_str().unwrap(),
            "--chart-key",
            "test/test_03",
            "--assets",
            "missing-assets",
            "--metadata",
            "overlay/metadata.json",
        ]));
        if cover.is_null() {
            assert!(w.report()["metadata"]["cover"].is_null());
        } else {
            assert!(
                Path::new(w.report()["metadata"]["cover"].as_str().unwrap())
                    .ends_with("overlay/cover.png")
            );
        }
    }
    let result = w.run(&[
        "--masterdata",
        fixture("masterdata").to_str().unwrap(),
        "--chart-key",
        "test/test_03",
        "--assets",
        "missing-assets",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("not present"));
}

#[test]
fn duplicate_music_or_difficulty_associations_are_rejected() {
    let w = Workspace::new();
    for entry in fs::read_dir(fixture("masterdata")).unwrap() {
        let path = entry.unwrap().path();
        fs::copy(&path, w.0.join(path.file_name().unwrap())).unwrap();
    }
    let original: Value =
        serde_json::from_slice(&fs::read(w.0.join("MasterLiveMusic.json")).unwrap()).unwrap();
    for same_music in [false, true] {
        let mut table = original.clone();
        if same_music {
            table["_allData"][0]["_hardID"] = json!(10103);
        } else {
            let mut other = table["_allData"][0].clone();
            other["_id"] = json!(202);
            table["_allData"].as_array_mut().unwrap().push(other);
        }
        let bytes = serde_json::to_vec(&table).unwrap();
        fs::write(w.0.join("MasterLiveMusic.json"), &bytes).unwrap();
        let mut provenance: Value =
            serde_json::from_slice(&fs::read(w.0.join("provenance.json")).unwrap()).unwrap();
        provenance["files"]["MasterLiveMusic.json"] =
            moenotes_chart_renderer::sha256(&bytes).into();
        fs::write(
            w.0.join("provenance.json"),
            serde_json::to_vec(&provenance).unwrap(),
        )
        .unwrap();
        let error = moenotes_chart_renderer::metadata::resolve(&w.0, "test/test_03", "en", None)
            .unwrap_err();
        assert!(error.to_string().contains("Ambiguous score owner"));
    }
}

#[test]
fn narrow_print_sheet_keeps_metadata_and_builtin_ignores_stale_pack_setting() {
    let w = Workspace::new();
    let result = Command::new(env!("CARGO_BIN_EXE_moenotes-chart-renderer"))
        .current_dir(&w.0)
        .env("MOENOTES_ASSETS_DIR", "missing-pack")
        .args([
            "render",
            fixture("synthetic.json").to_str().unwrap(),
            "-o",
            "sheet.png",
            "--long",
            "--pixels-per-lane",
            "2",
            "--supersample",
            "1",
            "--metadata",
            fixture("metadata.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    success(result);
    let report = w.report();
    assert_eq!(report["layout"]["theme"], "white");
    assert_eq!(report["images"].as_array().unwrap().len(), 1);
    assert!(report["images"][0]["width"].as_i64().unwrap() >= 360);
}
