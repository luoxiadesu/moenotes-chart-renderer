use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct Workspace(PathBuf);
static WORKSPACE_ID: AtomicU64 = AtomicU64::new(0);
impl Workspace {
    fn new() -> Self {
        Self::at_timestamp(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
    }
    fn at_timestamp(timestamp: u128) -> Self {
        let path = std::env::temp_dir().join(format!(
            "moenotes-cli-{}-{}-{}",
            std::process::id(),
            timestamp,
            WORKSPACE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        // Windows clocks can give parallel tests the same timestamp. Each test
        // owns its directory; never reuse another test's files or cleanup scope.
        fs::create_dir(&path).unwrap();
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
fn workspaces_with_identical_timestamps_do_not_share_cleanup() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let a = Workspace::at_timestamp(timestamp);
    let b = Workspace::at_timestamp(timestamp);
    assert_ne!(a.0, b.0);
    fs::write(b.0.join("owned.txt"), b"owned").unwrap();
    drop(a);
    assert_eq!(fs::read(b.0.join("owned.txt")).unwrap(), b"owned");
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
