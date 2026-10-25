//! Note values for the sheet's left rail: `/4`, `/8`, `3/16`, …
//!
//! Every playable onset is labelled with the value from its tick to the next rhythm event, written as a reduced
//! fraction of a whole note (the numerator is dropped when it is one). The rule mirrors sekai-sus2img's
//! `addTickText`: a gap that crosses a bar line after more than half a bar is cut at the end of the current bar,
//! so long rests show the note's remaining value instead of the full gap.
//!
//! Charts are not guaranteed to sit on exact tick grids, so a raw fraction could print nonsense such as
//! `79/1920`; the gap snaps to the nearest standard note value instead.
use crate::layout::PPQ;
use crate::parser::Score;
use serde::Serialize;

/// Playable onsets: taps, flicks and slide endpoints. Trace lines contribute their two endpoints (61/62) — the
/// judgement clock counts them — while their interior connections (63), standalone trace drawing (60), guides
/// and hidden nodes do not open or close a line and stay out of the rail.
const TICK_OPS: &[u32] = &[1, 20, 22, 40, 41, 42, 61, 62];

/// Standard note-value denominators (binary, dotted, triplets, quintuplets, septuplets).
const NOTE_DENOMINATORS: &[i32] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 18, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 96,
];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TickLabel {
    pub tick: i32,
    pub text: String,
}

/// Sorted ticks of the playable onsets that get a note-value label; one label per tick, chords share it.
pub fn rhythm_ticks(score: &Score) -> Vec<i32> {
    let mut ticks: Vec<i32> = score
        .notes
        .values()
        .filter(|n| TICK_OPS.contains(&n.kind) && n.width > 0.)
        .map(|n| n.tick)
        .collect();
    ticks.sort_unstable();
    ticks.dedup();
    ticks
}

/// Nearest standard note value to `value` (a fraction of a whole note), as `(numerator, denominator)`.
pub fn snap_note_value(value: f64) -> (i32, i32) {
    let mut best = (0, 1);
    let mut best_error = f64::INFINITY;
    for &denominator in NOTE_DENOMINATORS {
        let numerator = (value * f64::from(denominator)).round() as i32;
        if numerator <= 0 {
            continue;
        }
        let error = (f64::from(numerator) / f64::from(denominator) - value).abs();
        if error < best_error - 1e-12
            || ((error - best_error).abs() <= 1e-12 && denominator < best.1)
        {
            best = (numerator, denominator);
            best_error = error;
        }
    }
    best
}

/// The note value from `tick` to the next rhythm event or the end of the current bar, as `/4`, `/8`, `3/16`, …
pub fn note_value_label(score: &Score, tick: i32, next_tick: Option<i32>) -> Option<String> {
    let position = score.position(tick).ok()?;
    let bar_end = tick.checked_add(position.bar_ticks - position.rhythm)?;
    let interval = match next_tick {
        None => bar_end - tick,
        Some(next) => {
            let gap = next - tick;
            // A rest that stays in the bar keeps its full value; once it crosses the bar line after more than
            // half a bar it is cut at the bar end. `next == bar_end` lands exactly on the line, so it is not a
            // crossing.
            if gap > position.bar_ticks || (gap > position.bar_ticks / 2 && next > bar_end) {
                bar_end - tick
            } else {
                gap
            }
        }
    };
    if interval <= 0 {
        return None;
    }
    let (numerator, denominator) = snap_note_value(f64::from(interval) / (4. * f64::from(PPQ)));
    Some(if numerator == 1 {
        format!("/{denominator}")
    } else {
        format!("{numerator}/{denominator}")
    })
}

/// Labels for every playable onset, in tick order. Skipping a tick whose gap cannot be measured keeps the
/// remaining labels in that same order, which the renderer relies on to drop crowded neighbours.
pub fn tick_labels(score: &Score) -> Vec<TickLabel> {
    let ticks = rhythm_ticks(score);
    let labels: Vec<TickLabel> = ticks
        .iter()
        .enumerate()
        .filter_map(|(index, &tick)| {
            let next = ticks.get(index + 1).copied();
            note_value_label(score, tick, next).map(|text| TickLabel { tick, text })
        })
        .collect();
    debug_assert!(labels.windows(2).all(|w| w[0].tick < w[1].tick));
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    fn score(json: &str) -> Result<Score> {
        Score::parse(json.as_bytes(), false)
    }

    #[test]
    fn labels_measure_to_the_next_rhythm_event() -> Result<()> {
        let s = score(
            r#"{"events":{"sig":[{"t":0,"sig":[4,4]}]},"notes":[
                {"t":0},{"t":0},{"t":480},{"t":720},{"t":840},{"t":1920}]}"#,
        )?;
        assert_eq!(rhythm_ticks(&s), [0, 480, 720, 840, 1920]);
        assert_eq!(note_value_label(&s, 0, Some(480)).as_deref(), Some("/4"));
        assert_eq!(note_value_label(&s, 480, Some(720)).as_deref(), Some("/8"));
        assert_eq!(note_value_label(&s, 720, Some(840)).as_deref(), Some("/16"));
        // 840 -> 1920 crosses the bar line by more than half a bar, so the label stops at the bar end:
        // 1080 ticks = 9/16 of a whole note.
        assert_eq!(
            note_value_label(&s, 840, Some(1920)).as_deref(),
            Some("9/16")
        );
        // The last onset has no successor; its bar holds a whole note.
        assert_eq!(note_value_label(&s, 1920, None).as_deref(), Some("/1"));
        Ok(())
    }

    #[test]
    fn odd_gaps_snap_to_standard_note_values() -> Result<()> {
        assert_eq!(snap_note_value(79. / 1920.), (1, 24));
        assert_eq!(snap_note_value(401. / 1920.), (5, 24));
        // A chart with humanised ticks still prints musical fractions.
        let s = score(r#"{"events":{},"notes":[{"t":0},{"t":79},{"t":480}]}"#)?;
        assert_eq!(note_value_label(&s, 0, Some(79)).as_deref(), Some("/24"));
        assert_eq!(note_value_label(&s, 79, Some(480)).as_deref(), Some("5/24"));
        Ok(())
    }

    #[test]
    fn triplet_gaps_use_their_own_denominators() -> Result<()> {
        let s = score(
            r#"{"events":{"sig":[{"t":0,"sig":[12,8]}]},"notes":[
                {"t":0},{"t":320},{"t":640},{"t":960}]}"#,
        )?;
        assert_eq!(note_value_label(&s, 0, Some(320)).as_deref(), Some("/6"));
        assert_eq!(note_value_label(&s, 320, Some(640)).as_deref(), Some("/6"));
        // The last onset keeps the rest of the 12/8 bar: 1920 ticks from 960 is a whole note.
        assert_eq!(note_value_label(&s, 960, None).as_deref(), Some("/1"));
        Ok(())
    }

    #[test]
    fn slides_and_flicks_count_as_onsets_but_guides_do_not() -> Result<()> {
        let s = score(
            r#"{"events":{},"notes":[
                {"t":0},
                {"type":"flick","t":480,"pos":4,"size":6},
                {"type":"long","node":[{"t":960,"pos":0,"size":6},{"t":1200,"pos":0,"size":6}]},
                {"type":"guide","node":[{"t":1440,"pos":0,"size":6},{"t":1920,"pos":0,"size":6}]}]}"#,
        )?;
        // Tap, flick, and the long note's two endpoints; the guide stays out of the rhythm rail.
        assert_eq!(rhythm_ticks(&s), [0, 480, 960, 1200]);
        let labels = tick_labels(&s);
        assert_eq!(labels.len(), 4);
        assert_eq!(labels[0].text, "/4");
        Ok(())
    }

    #[test]
    fn long_contributes_exactly_its_two_endpoints() -> Result<()> {
        let s = score(
            r#"{"events":{},"notes":[{"type":"long","node":[
                {"t":480,"pos":0,"size":6},{"t":720,"pos":4,"size":6},{"t":1200,"pos":8,"size":6}]}]}"#,
        )?;
        // The middle slide connection is not a rhythm event.
        assert_eq!(rhythm_ticks(&s), [480, 1200]);
        Ok(())
    }

    #[test]
    fn guide_contributes_nothing_to_the_rail() -> Result<()> {
        let s = score(
            r#"{"events":{},"notes":[
                {"type":"guide","node":[
                    {"t":480,"pos":0,"size":6},{"t":720,"pos":4,"size":6},{"t":1200,"pos":8,"size":6}]},
                {"t":1920}]}"#,
        )?;
        // Only the standalone tap is a rhythm event; none of the guide's nodes are.
        assert_eq!(rhythm_ticks(&s), [1920]);
        assert_eq!(tick_labels(&s).len(), 1);
        Ok(())
    }

    #[test]
    fn a_rest_landing_on_the_bar_line_keeps_its_value() -> Result<()> {
        // From the second beat of a 4/4 bar to the next bar line: 1440 ticks = 3/4. The line itself is not a
        // crossing, so the label must not be cut and rewritten as a different fraction.
        let s = score(r#"{"events":{},"notes":[{"t":480},{"t":1920}]}"#)?;
        assert_eq!(
            note_value_label(&s, 480, Some(1920)).as_deref(),
            Some("3/4")
        );
        Ok(())
    }

    #[test]
    fn label_collisions_are_left_to_the_renderer() -> Result<()> {
        // Two onsets 120 ticks apart are both labelled; dropping the crowded one is a pixel-space decision.
        let s = score(r#"{"events":{},"notes":[{"t":0},{"t":120},{"t":1920}]}"#)?;
        assert_eq!(tick_labels(&s).len(), 3);
        Ok(())
    }
}
