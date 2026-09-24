//! Renderer-independent geometry. Integer-tick samples come from the C helper;
//! cuts are explicit samples so adjacent columns share exactly the same edge.
use crate::{
    layout::Layout,
    parser::{Note, Score},
};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Row {
    pub tick: i32,
    pub left: f64,
    pub right: f64,
}
#[derive(Debug, Serialize)]
pub struct Ribbon {
    pub id: i32,
    pub guide: bool,
    pub rows: Vec<Row>,
    pub segments: Vec<Vec<Row>>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Annotation {
    pub tick: i32,
    pub label: String,
    pub kind: String,
}
#[derive(Debug, Serialize)]
pub struct Glyph {
    pub note: Note,
    pub column: usize,
}
#[derive(Debug, Serialize)]
pub struct Scene {
    pub layout: Layout,
    pub glyphs: Vec<Glyph>,
    pub ribbons: Vec<Ribbon>,
    pub pairs: Vec<(i32, i32)>,
    pub annotations: Vec<Annotation>,
    pub fever: Vec<(i32, i32)>,
    pub source_notes: usize,
    pub warnings: Vec<String>,
    pub hidden_notes: usize,
    pub zero_width: usize,
    pub points: usize,
    pub instantaneous_segments: usize,
    pub statistics: crate::parser::Statistics,
}
fn interpolate(score: &Score, a: &Note, b: &Note, tick: i32, layout: &Layout) -> Result<Row> {
    let u = if b.tick == a.tick {
        0.
    } else if layout.options.curve_mode == crate::layout::CurveMode::NativeParameters
        && b.time_ms != a.time_ms
    {
        ((score.note_time(tick)? - a.time_ms) as f32 / (b.time_ms - a.time_ms) as f32).clamp(0., 1.)
    } else {
        (tick - a.tick) as f32 / (b.tick - a.tick) as f32
    };
    let (el, er) = if score.mirror && layout.options.curve_mode == crate::layout::CurveMode::Musical
    {
        (a.ease_right, a.ease_left)
    } else {
        (a.ease_left, a.ease_right)
    };
    let ease = |e| match e {
        1 => u * (2. - u),
        2 => u * u,
        _ => u,
    };
    let al = a.source_left as f32;
    let bl = b.source_left as f32;
    let ar = al + a.source_width as f32;
    let br = bl + b.source_width as f32;
    let (left, right) = if layout.options.curve_mode == crate::layout::CurveMode::NativeParameters {
        // Match the cached-center operations in the local ARM64 helper before
        // converting from lane-center to lane-boundary coordinates.
        let ca = (a.source_left as f32 + (a.source_right - 1.) as f32) * 0.5;
        let cb = (b.source_left as f32 + (b.source_right - 1.) as f32) * 0.5;
        let la = ca - a.source_width as f32 * 0.5;
        let ra = ca + a.source_width as f32 * 0.5;
        let lb = cb - b.source_width as f32 * 0.5;
        let rb = cb + b.source_width as f32 * 0.5;
        (
            la + ease(el) * (lb - la) + 0.5,
            ra + ease(er) * (rb - ra) + 0.5,
        )
    } else {
        (al + ease(el) * (bl - al), ar + ease(er) * (br - ar))
    };
    ensure!(left.is_finite() && right.is_finite(), "Nonfinite segment");
    Ok(Row {
        tick,
        left: left as f64,
        right: right as f64,
    })
}
#[allow(clippy::too_many_arguments)]
fn subdivide(
    score: &Score,
    anchors: (&Note, &Note),
    a: Row,
    b: Row,
    layout: &Layout,
    points: &mut Vec<Row>,
    budget: &mut usize,
) -> Result<()> {
    ensure!(*budget < 1_000_000, "Line sample budget exceeded");
    if b.tick - a.tick > 1 {
        let mid = a.tick + (b.tick - a.tick) / 2;
        let m = interpolate(score, anchors.0, anchors.1, mid, layout)?;
        let t = (mid - a.tick) as f64 / (b.tick - a.tick) as f64;
        let error = (m.left - (a.left + (b.left - a.left) * t))
            .abs()
            .max((m.right - (a.right + (b.right - a.right) * t)).abs())
            * layout.options.pixels_per_lane;
        let height = (b.tick - a.tick) as f64 / 480. * layout.options.pixels_per_beat;
        if error > 0.2 || height > 10. {
            subdivide(score, anchors, a, m, layout, points, budget)?;
            subdivide(score, anchors, m, b, layout, points, budget)?;
            return Ok(());
        }
    }
    points.push(b);
    *budget += 1;
    Ok(())
}
impl Scene {
    pub fn build(score: &Score, layout: Layout) -> Result<Self> {
        let mut warnings = vec![];
        if score.warnings != 0 {
            warnings.push(format!("parser_warning_flags={}", score.warnings));
        }
        if score.notes.values().any(|n| n.alpha != 0) {
            warnings.push("Source fade metadata is shown as FI/FO annotations; temporal opacity is not simulated".into());
        }
        let mut glyphs = vec![];
        let mut hidden = 0;
        let mut zero = 0;
        for n in score.notes.values() {
            if n.role().is_none() {
                hidden += 1;
                continue;
            }
            if n.width <= 0. {
                zero += 1;
                continue;
            }
            let column = layout
                .owner(n.tick)
                .ok_or_else(|| anyhow::anyhow!("Note outside layout"))?;
            glyphs.push(Glyph {
                note: n.clone(),
                column,
            });
        }
        glyphs.sort_by_key(|g| (g.note.tick, g.note.id));
        let mut ribbons = vec![];
        let mut budget = 0;
        let mut instantaneous_segments = 0;
        for branch in &score.branches {
            let anchors: Vec<&Note> = branch
                .members
                .iter()
                .map(|id| &score.notes[id])
                .filter(|n| !n.slide_along)
                .collect();
            let mut segments = vec![];
            for pair in anchors.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                ensure!(b.tick >= a.tick, "Nonmonotonic render anchors");
                if a.tick == b.tick {
                    instantaneous_segments += 1;
                    segments.push(vec![
                        Row {
                            tick: a.tick,
                            left: a.source_left,
                            right: a.source_right,
                        },
                        Row {
                            tick: b.tick,
                            left: b.source_left,
                            right: b.source_right,
                        },
                    ]);
                    budget += 2;
                    continue;
                }
                let mut cuts: BTreeSet<i32> = [a.tick, b.tick].into_iter().collect();
                cuts.extend(
                    layout
                        .columns
                        .iter()
                        .flat_map(|c| [c.start, c.end])
                        .filter(|t| a.tick < *t && *t < b.tick),
                );
                cuts.extend(
                    score
                        .bpms
                        .iter()
                        .map(|v| v.tick)
                        .filter(|t| a.tick < *t && *t < b.tick),
                );
                cuts.extend(
                    score
                        .signatures
                        .iter()
                        .map(|v| v.tick)
                        .filter(|t| a.tick < *t && *t < b.tick),
                );
                let mut segment = vec![interpolate(score, a, b, a.tick, &layout)?];
                budget += 1;
                for tick in cuts.into_iter().skip(1) {
                    let prev = *segment.last().unwrap();
                    let next = interpolate(score, a, b, tick, &layout)?;
                    subdivide(
                        score,
                        (a, b),
                        prev,
                        next,
                        &layout,
                        &mut segment,
                        &mut budget,
                    )?;
                }
                segments.push(segment);
            }
            let rows = segments.iter().flatten().copied().collect();
            ribbons.push(Ribbon {
                id: branch.id,
                guide: branch.guide,
                rows,
                segments,
            });
        }
        // Cached pair links need not be symmetric; deduplicate actual edges.
        let visible: BTreeMap<i32, &Note> = glyphs.iter().map(|g| (g.note.id, &g.note)).collect();
        let mut pairs = BTreeSet::new();
        for n in visible.values() {
            if let Some(p) = visible.get(&n.pair)
                && p.tick == n.tick
                && p.id != n.id
            {
                pairs.insert((n.id.min(p.id), n.id.max(p.id)));
            }
        }
        let mut annotations = vec![];
        for b in &score.bpms {
            annotations.push(Annotation {
                tick: b.tick,
                label: format!("{:.2} BPM", b.bpm),
                kind: "bpm".into(),
            });
        }
        for s in &score.signatures {
            annotations.push(Annotation {
                tick: s.tick,
                label: format!("{}/{}", s.numerator, s.denominator),
                kind: "meter".into(),
            });
        }
        let mut fever = vec![];
        for e in &score.events {
            match e.kind {
                0 => annotations.push(Annotation {
                    tick: e.tick,
                    label: "SKILL".into(),
                    kind: "skill".into(),
                }),
                1 => fever.push((e.tick, e.end_tick)),
                2 => annotations.push(Annotation {
                    tick: e.tick,
                    label: format!(
                        "CALL {}",
                        e.rhythms
                            .iter()
                            .map(|p| format!("{:.0}%", p * 100.))
                            .collect::<Vec<_>>()
                            .join("/")
                    ),
                    kind: "call".into(),
                }),
                _ => {}
            }
        }
        for n in score.notes.values().filter(|n| n.alpha != 0) {
            annotations.push(Annotation {
                tick: n.tick,
                label: if n.alpha == 1 {
                    "FADE IN".into()
                } else {
                    "FADE OUT".into()
                },
                kind: "fade".into(),
            });
        }
        annotations.sort_by_key(|a| a.tick);
        Ok(Self {
            layout,
            glyphs,
            ribbons,
            pairs: pairs.into_iter().collect(),
            annotations,
            fever,
            source_notes: score.notes.len(),
            warnings,
            hidden_notes: hidden,
            zero_width: zero,
            points: budget,
            instantaneous_segments,
            statistics: score.statistics.clone(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Options;
    #[cfg(feature = "research-tests")]
    #[test]
    fn corpus_positive_duration_segments_match_c_sampling() -> Result<()> {
        use std::path::{Path, PathBuf};
        fn walk(path: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
            for entry in std::fs::read_dir(path)? {
                let p = entry?.path();
                if p.is_dir() {
                    walk(&p, out)?;
                } else if p.extension().is_some_and(|e| e == "json") {
                    out.push(p);
                }
            }
            Ok(())
        }
        let root = std::path::PathBuf::from(
            std::env::var_os("MOENOTES_TEST_CHARTS")
                .expect("Set MOENOTES_TEST_CHARTS for research tests"),
        );
        let mut paths = vec![];
        walk(&root, &mut paths)?;
        assert!(!paths.is_empty());
        let mut checks = 0;
        for path in paths {
            let bytes = std::fs::read(&path)?;
            for mirror in [false, true] {
                let s = Score::parse(&bytes, mirror)?;
                let layout = Layout::build(
                    &s,
                    Options {
                        pixels_per_beat: 12.,
                        supersample: 1,
                        ..Options::default()
                    },
                )?;
                for branch in &s.branches {
                    let anchors: Vec<_> = branch
                        .members
                        .iter()
                        .map(|id| &s.notes[id])
                        .filter(|n| !n.slide_along)
                        .collect();
                    for w in anchors.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        if b.tick - a.tick < 2 {
                            continue;
                        }
                        for div in [4, 2] {
                            let tick = a.tick + (b.tick - a.tick) / div;
                            if tick <= a.tick || tick >= b.tick {
                                continue;
                            }
                            let actual = interpolate(&s, a, b, tick, &layout)?;
                            let expected = s.sample(branch.id, tick)?;
                            assert!(
                                (actual.left - expected.0).abs() < 0.00001
                                    && (actual.right - expected.1).abs() < 0.00001,
                                "{} mirror={mirror} line={} tick={tick}: {actual:?} != {expected:?}",
                                path.display(),
                                branch.id
                            );
                            checks += 1;
                        }
                    }
                }
            }
        }
        assert!(checks > 100_000);
        println!("positive-duration C geometry comparisons: {checks}");
        Ok(())
    }
    #[test]
    fn same_tick_transition_has_two_sides_and_no_diagonal_shortcut() -> Result<()> {
        let s=Score::parse(br#"{"events":{},"notes":[{"type":"long","node":[{"t":0,"pos":0,"size":4},{"t":480,"pos":0,"size":4,"visible":false},{"t":480,"pos":12,"size":4,"visible":false},{"t":960,"pos":12,"size":4}]}]}"#,false)?;
        let scene = Scene::build(&s, Layout::build(&s, Options::default())?)?;
        assert_eq!(scene.instantaneous_segments, 1);
        let segments = &scene.ribbons[0].segments;
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].last().unwrap().left, 0.);
        assert_eq!(segments[1][0].tick, 480);
        assert_eq!(segments[1][1].tick, 480);
        assert_eq!(segments[1][0].left, 0.);
        assert_eq!(segments[1][1].left, 12.);
        assert_eq!(segments[2][0].left, 12.);
        Ok(())
    }
    #[test]
    fn native_parameter_mirror_is_explicitly_different() -> Result<()> {
        let s=Score::parse(br#"{"events":{},"notes":[{"type":"long","node":[{"t":0,"pos":0,"size":6,"ease":["out","in"]},{"t":480,"pos":0,"size":12}]}]}"#,true)?;
        let a = &s.notes[&s.branches[0].members[0]];
        let b = &s.notes[&s.branches[0].members[1]];
        let musical = Layout::build(&s, Options::default())?;
        let native = Layout::build(
            &s,
            Options {
                curve_mode: crate::layout::CurveMode::NativeParameters,
                ..Options::default()
            },
        )?;
        let m = interpolate(&s, a, b, 240, &musical)?;
        let n = interpolate(&s, a, b, 240, &native)?;
        assert_eq!(m.left, 16.5);
        assert_eq!(n.left, 13.5);
        assert_eq!(m.right, 24.);
        assert_eq!(n.right, 24.);
        Ok(())
    }
    #[test]
    fn independent_edges_match_c_sampler_away_from_zero_duration() -> Result<()> {
        for mirror in [false, true] {
            let s=Score::parse(br#"{"events":{},"notes":[{"type":"long","node":[{"t":0,"pos":1,"size":4,"ease":["out","in"]},{"t":480,"pos":12,"size":8},{"t":960,"pos":4,"size":2}]}]}"#,mirror)?;
            let l = Layout::build(&s, Options::default())?;
            let a = &s.notes[&s.branches[0].members[0]];
            let b = &s.notes[&s.branches[0].members[1]];
            for tick in [0, 1, 119, 120, 240, 479, 480] {
                let r = interpolate(&s, a, b, tick, &l)?;
                let expected = s.sample(0, tick)?;
                assert!(
                    (r.left - expected.0).abs() < 0.00001 && (r.right - expected.1).abs() < 0.00001
                );
            }
        }
        Ok(())
    }
    #[test]
    fn ribbon_cut_is_continuous_and_hidden_anchor_retained() -> Result<()> {
        let s=Score::parse(br#"{"events":{},"notes":[{"type":"long","node":[{"t":0,"pos":0,"size":6,"ease":"out"},{"t":960,"pos":4,"size":8,"visible":false},{"t":5000,"pos":18,"size":6}]}]}"#,false)?;
        let l = Layout::build(
            &s,
            Options {
                bars_per_column: 1,
                ..Options::default()
            },
        )?;
        let scene = Scene::build(&s, l)?;
        for tick in [960, 1920, 3840] {
            let row = scene.ribbons[0]
                .rows
                .iter()
                .find(|r| r.tick == tick)
                .unwrap();
            assert_eq!((row.left, row.right), s.sample(0, tick)?);
        }
        assert_eq!(scene.glyphs.len(), 2);
        assert_eq!(scene.ribbons.len(), 1);
        Ok(())
    }
}
