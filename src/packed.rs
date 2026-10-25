//! Master full-combo overrides from chart packaging metadata.
//!
//! Online charts are also published as packed snapshots
//! (`"format": "nnnotes.live-score/1"`) whose note list is already expanded
//! into native notes. Such a snapshot has no `meta`/`score.events`, so it
//! cannot be joined to a master score and the master `_fullComboCount` would
//! otherwise be lost. The pack's `judgementNoteCount` marks the snapshot as
//! packed; the reported count is the parser's own count on the same nodes
//! (some packs omit Combos from `judgementNoteCount`).
use anyhow::{Result, bail};
use serde_json::{Value, json};
const PACK_FORMAT: &str = "nnnotes.live-score/1";
/// True for an already-expanded native snapshot. Source charts keep `meta` or
/// `score`, even when they also declare the pack format.
pub fn is_snapshot(chart: &Value) -> bool {
    chart.get("meta").is_none()
        && chart.get("score").is_none()
        && chart.get("format").and_then(Value::as_str) == Some(PACK_FORMAT)
        && chart.get("notes").is_some_and(|notes| {
            notes.as_array().is_some_and(|notes| {
                notes
                    .first()
                    .is_none_or(|note| note.get("op").is_some_and(|op| op.is_u64()))
            })
        })
}
/// Maps a packed snapshot's `tickSegments` (`{startTick, ...}` entries) onto
/// the source-chart event names the parser timeline reader expects. Mutates
/// `chart` in place; snapshots without `tickSegments` keep the default
/// 120 BPM 4/4 timeline.
pub fn normalize_timeline(chart: &mut Value) {
    let segments = match chart.get_mut("tickSegments").map(Value::take) {
        Some(Value::Object(segments)) => segments,
        _ => return,
    };
    let mut events = serde_json::Map::new();
    if let Some(Value::Array(bpm)) = segments.get("bpm") {
        events.insert(
            "bpm".into(),
            Value::Array(
                bpm.iter()
                    .map(|b| {
                        json!({"t": b.get("startTick").cloned().unwrap_or(json!(0)),
                               "bpm": b.get("bpm").cloned().unwrap_or(json!(120))})
                    })
                    .collect(),
            ),
        );
    }
    if let Some(Value::Array(sig)) = segments.get("sig") {
        events.insert(
            "sig".into(),
            Value::Array(
                sig.iter()
                    .filter_map(|s| {
                        let tick = s.get("startTick")?.as_i64()?;
                        Some((tick, s))
                    })
                    .map(|(tick, s)| {
                        json!({"t": tick,
                               "sig": [s.get("numerator").cloned().unwrap_or(json!(4)),
                                       s.get("denominator").cloned().unwrap_or(json!(4))]})
                    })
                    .collect(),
            ),
        );
    }
    chart["events"] = Value::Object(events);
}
/// Returns the packed snapshot's full-combo count, or `None` for ordinary
/// charts (`meta`/`score`), which keep the masterdata join.
///
/// `judgementNoteCount` is not the native FC: some packs omit Combos and
/// Skipped Combos from it (0005 HARD: 728 reported, 791 native notes with 728
/// judgements). The authoritative value is the parser's count on the same
/// snapshot, so callers pass `reconstructed` from the packed parse.
pub fn packed_full_combo(chart: &Value, reconstructed: u64) -> Result<Option<u64>> {
    if chart.get("format").and_then(Value::as_str) != Some(PACK_FORMAT) {
        return Ok(None);
    }
    if chart
        .get("judgementNoteCount")
        .and_then(Value::as_u64)
        .is_none_or(|count| count == 0)
    {
        bail!("Packed chart {PACK_FORMAT} has no positive judgementNoteCount");
    }
    Ok(Some(reconstructed))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn packed_snapshot_uses_judgement_count() {
        let chart = json!({
            "format": PACK_FORMAT,
            "judgementNoteCount": 965,
            "notes": [{"op": 1}]
        });
        assert_eq!(packed_full_combo(&chart, 965).unwrap(), Some(965));
        assert_eq!(packed_full_combo(&chart, 971).unwrap(), Some(971));
    }
    #[test]
    fn snapshot_detection_requires_pack_format_without_source_fields() {
        assert!(is_snapshot(&json!({"format": PACK_FORMAT, "notes": []})));
        assert!(!is_snapshot(
            &json!({"format": PACK_FORMAT, "notes": [], "meta": {"version": 100}})
        ));
        assert!(!is_snapshot(&json!({"format": PACK_FORMAT})));
        assert!(!is_snapshot(&json!({"events": {}, "notes": []})));
        assert!(!is_snapshot(
            &json!({"format": PACK_FORMAT, "notes": [{"t": 0, "pos": 2, "size": 6}]})
        ));
    }
    #[test]
    fn ordinary_chart_is_left_to_masterdata() {
        for chart in [
            json!({"meta": {"version": 100}, "score": {"notes": []}}),
            json!({"format": "something/else", "judgementNoteCount": 3}),
            json!({}),
        ] {
            assert_eq!(packed_full_combo(&chart, 1).unwrap(), None);
        }
    }
    #[test]
    fn incomplete_packed_snapshot_is_rejected() {
        for chart in [
            json!({"format": PACK_FORMAT}),
            json!({"format": PACK_FORMAT, "judgementNoteCount": 0}),
            json!({"format": PACK_FORMAT, "judgementNoteCount": "965"}),
        ] {
            assert!(packed_full_combo(&chart, 1).is_err());
        }
    }
}

/// Rebuilds a packed snapshot from a chart-site manifest whose score entry is
/// split into `[field, asset path, size]` parts. `read` loads one asset path
/// relative to the manifest's asset root; the snapshot keeps `events` from the
/// regular `score/...` parts so renderer annotations survive.
pub fn assemble_parts(manifest: &Value, read: impl Fn(&str) -> Result<Value>) -> Result<Value> {
    let entry = manifest["files"]
        .as_object()
        .and_then(|files| {
            files
                .iter()
                .find(|(key, _)| key.starts_with("score/") && key.ends_with(".notes.json"))
        })
        .map(|(_, value)| value)
        .ok_or_else(|| anyhow::anyhow!("Packed manifest has no score entry"))?;
    if let Some(asset) = entry["asset"].as_str() {
        let snapshot = read(asset)?;
        ensure_snapshot(&snapshot)?;
        return Ok(snapshot);
    }
    let parts = entry["parts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Packed score has neither asset nor parts"))?;
    let mut snapshot = serde_json::Map::new();
    for part in parts {
        let fields = part
            .as_array()
            .filter(|v| v.len() == 3)
            .ok_or_else(|| anyhow::anyhow!("Packed part must be [field, asset, size]"))?;
        let name = fields[0]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Packed part name"))?;
        let path = fields[1]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Packed part asset"))?;
        snapshot.insert(name.to_owned(), read(path)?);
    }
    let mut snapshot = Value::Object(snapshot);
    ensure_snapshot(&snapshot)?;
    if snapshot.get("format").is_none() {
        snapshot["format"] = PACK_FORMAT.into();
    }
    Ok(snapshot)
}

fn ensure_snapshot(snapshot: &Value) -> Result<()> {
    anyhow::ensure!(
        is_snapshot(snapshot),
        "Packed score does not describe an nnnotes.live-score/1 snapshot"
    );
    anyhow::ensure!(
        snapshot
            .get("judgementNoteCount")
            .and_then(Value::as_u64)
            .is_some(),
        "Packed snapshot has no judgementNoteCount"
    );
    Ok(())
}

#[cfg(test)]
mod assemble_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn parts_are_joined_into_a_snapshot() {
        let manifest = json!({
            "files": {"score/0111_03.notes.json": {"parts": [
                ["format", "assets/f.json", 1],
                ["notes", "assets/n.json", 2],
                ["judgementNoteCount", "assets/j.json", 3]
            ]}}
        });
        let read = |path: &str| -> Result<Value> {
            Ok(match path {
                "assets/f.json" => json!("nnnotes.live-score/1"),
                "assets/n.json" => json!([{"op": 1, "t": 0}]),
                "assets/j.json" => json!(1),
                other => anyhow::bail!("unexpected {other}"),
            })
        };
        let snapshot = assemble_parts(&manifest, read).unwrap();
        assert!(is_snapshot(&snapshot));
        assert_eq!(snapshot["judgementNoteCount"], 1);
    }
    #[test]
    fn missing_parts_fail_instead_of_guessing() {
        let manifest = json!({"files": {"score/x.notes.json": {"parts": [["notes", "a", 1]]}}});
        let read = |_: &str| -> Result<Value> { Ok(json!([{"op": 1}])) };
        assert!(assemble_parts(&manifest, read).is_err());
    }
}
