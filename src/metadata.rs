//! Metadata joins use explicit chart keys and a commit-pinned online snapshot.
use crate::{render::Metadata, sha256};
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug)]
pub struct Resolved {
    pub metadata: Metadata,
    pub cover: Option<PathBuf>,
}
fn table(dir: &Path, name: &str) -> Result<Vec<Value>> {
    ensure!(
        fs::metadata(dir.join(format!("{name}.json")))?.len() <= 64 * 1024 * 1024,
        "Master table too large"
    );
    let v: Value = serde_json::from_slice(&fs::read(dir.join(format!("{name}.json")))?)?;
    Ok(v["_allData"]
        .as_array()
        .context("Invalid master table")?
        .clone())
}
fn string(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_owned()
}
pub fn snapshot_directory(path: &Path) -> Result<PathBuf> {
    if path.join("current.json").is_file() {
        let p: Value = serde_json::from_slice(&fs::read(path.join("current.json"))?)?;
        let dir = p["directory"]
            .as_str()
            .context("Invalid masterdata current pointer")?;
        ensure!(
            dir.len() == 40 && dir.bytes().all(|b| b.is_ascii_hexdigit()),
            "Unsafe masterdata snapshot directory"
        );
        ensure!(
            p["commit"].as_str() == Some(dir),
            "Masterdata current pointer commit mismatch"
        );
        let snapshot = path.join(dir);
        let provenance: Value =
            serde_json::from_slice(&fs::read(snapshot.join("provenance.json"))?)?;
        ensure!(
            provenance["commit"].as_str() == Some(dir),
            "Masterdata snapshot commit mismatch"
        );
        ensure!(snapshot.is_dir(), "Current snapshot is missing");
        Ok(snapshot)
    } else {
        Ok(path.to_path_buf())
    }
}
pub fn resolve(
    snapshot: &Path,
    chart_key: &str,
    language: &str,
    assets: Option<&Path>,
) -> Result<Resolved> {
    let selected = snapshot_directory(snapshot)?;
    let snapshot = selected.as_path();
    ensure!(
        std::fs::metadata(snapshot.join("provenance.json"))?.len() <= 1024 * 1024,
        "Masterdata provenance too large"
    );
    let provenance: Value = serde_json::from_slice(
        &fs::read(snapshot.join("provenance.json"))
            .context("Masterdata requires sync_masterdata.py provenance")?,
    )?;
    ensure!(
        provenance["repository"] == "https://github.com/StarMoe-org/moenotes-masterdata",
        "Unexpected masterdata source"
    );
    for name in [
        "MasterLiveMusic.json",
        "MasterLiveMusicScore.json",
        "MasterText.json",
        "MasterBand.json",
    ] {
        let hash = provenance["files"][name]
            .as_str()
            .context("Missing required masterdata hash")?;
        ensure!(
            sha256(&fs::read(snapshot.join(name))?) == hash,
            "Masterdata hash mismatch: {name}"
        );
    }
    let scores = table(snapshot, "MasterLiveMusicScore")?;
    let musics = table(snapshot, "MasterLiveMusic")?;
    let texts = table(snapshot, "MasterText")?;
    let bands = table(snapshot, "MasterBand")?;
    let chart_key = chart_key.trim_end_matches(".gz").trim_end_matches(".json");
    let found: Vec<_> = scores
        .iter()
        .filter(|s| {
            let key = s["_musicScoreTextFileName"].as_str().unwrap_or("");
            key == chart_key
                || (!chart_key.contains('/') && key.rsplit('/').next() == Some(chart_key))
        })
        .collect();
    ensure!(
        found.len() == 1,
        "Chart key must match exactly one master score, got {}",
        found.len()
    );
    let score = found[0];
    let sid = score["_id"].as_i64().context("Score ID")?;
    let difficulties = [
        ("_easyID", "EASY"),
        ("_normalID", "NORMAL"),
        ("_hardID", "HARD"),
        ("_expertID", "EXPERT"),
    ];
    let (music, difficulty) = musics
        .iter()
        .find_map(|m| {
            difficulties
                .iter()
                .find(|(key, _)| m[*key].as_i64() == Some(sid))
                .map(|(_, label)| (m, *label))
        })
        .context("Score has no music")?;
    let lang = match language {
        "ja" => "_japanese",
        "en" => "_english",
        "zh-Hant" => "_traditionalChinese",
        "zh-Hans" => "_simplifiedChinese",
        "ko" => "_korean",
        _ => bail!("Unsupported metadata language"),
    };
    let textmap: BTreeMap<_, _> = texts
        .iter()
        .filter_map(|t| t["_id"].as_str().map(|id| (id, t)))
        .collect();
    let lookup = |id: &str| -> Result<String> {
        if id.is_empty() {
            return Ok(String::new());
        }
        let t = textmap
            .get(id)
            .with_context(|| format!("Missing MasterText {id}"))?;
        let chosen = t[lang]
            .as_str()
            .filter(|v| !v.is_empty())
            .or_else(|| t["_japanese"].as_str())
            .context("Missing translated text")?;
        Ok(chosen.into())
    };
    let mut authors = vec![];
    for (key, label) in [
        ("_lyricistTextID", "Lyrics"),
        ("_composerTextID", "Music"),
        ("_arrangerTextID", "Arrangement"),
    ] {
        let value = lookup(&string(music, key))?;
        if !value.is_empty() {
            authors.push(format!("{label}: {value}"));
        }
    }
    let mut artist = vec![];
    for id in music["_bandIDs"].as_array().into_iter().flatten() {
        if let Some(b) = bands.iter().find(|b| b["_id"] == *id) {
            let name = b["_nameTextID"]
                .as_str()
                .or_else(|| b["_bandNameTextID"].as_str())
                .unwrap_or("");
            let value = lookup(name)?;
            if !value.is_empty() {
                artist.push(value);
            }
        }
    }
    let jacket = string(music, "_jacketAssetName");
    let mut cover = None;
    if let Some(assets) = assets {
        ensure!(
            !jacket.is_empty()
                && Path::new(&jacket).components().count() == 1
                && Path::new(&jacket)
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "Unsafe jacket key"
        );
        let directory = assets.join("Image/Jacket").join(&jacket);
        if directory.is_dir() {
            let mut candidates: Vec<_> = fs::read_dir(directory)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|s| s == "png"))
                .collect();
            candidates.sort();
            cover = candidates.into_iter().next();
        }
        ensure!(
            cover.is_some(),
            "Cover {jacket} not present in supplied by-key assets"
        );
    }
    let metadata = Metadata {
        title: lookup(&string(music, "_titleTextID"))?,
        difficulty: difficulty.into(),
        level: {
            let level = score["_musicScoreDisplayLevel"]
                .as_f64()
                .context("Display level")?;
            if level.fract() == 0. {
                format!("{level:.0}")
            } else {
                level.to_string()
            }
        },
        artist: artist.join(" / "),
        author: authors.join("  ·  "),
        cover: cover.as_ref().map(|p| p.to_string_lossy().into_owned()),
        master_full_combo: score["_fullComboCount"].as_u64(),
        provenance: Some(provenance),
    };
    Ok(Resolved { metadata, cover })
}
