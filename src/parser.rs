//! RAII boundary around the pinned C parser. No C allocations escape this module.
use anyhow::{Result, bail, ensure};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    ffi::{CStr, c_void},
    ptr::{self, NonNull},
};

#[allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    dead_code,
    unused_imports,
    clippy::all
)]
mod ffi {
    include!("parser_ffi.rs");
}

#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub id: i32,
    pub kind: u32,
    pub tick: i32,
    pub time_ms: i32,
    pub bar: i32,
    pub left: f64,
    pub right: f64,
    pub width: f64,
    pub source_left: f64,
    pub source_right: f64,
    pub source_width: f64,
    pub direction: u32,
    pub critical: bool,
    pub auto: bool,
    pub slide_along: bool,
    pub pair: i32,
    pub alpha: u32,
    pub ease_left: u32,
    pub ease_right: u32,
}
impl Note {
    pub fn role(&self) -> Option<&'static str> {
        match self.kind {
            1 | 101 => Some("tap"),
            20 => Some("slide"),
            21 => Some("connection"),
            22 => Some("slide_end"),
            40..=42 | 102 => Some(match self.direction {
                1 => "flick_left",
                2 => "flick_right",
                _ => "flick",
            }),
            60..=63 | 104 | 105 => Some("trace"),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Branch {
    pub id: i32,
    pub guide: bool,
    pub members: Vec<i32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Bpm {
    pub tick: i32,
    pub bpm: f64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Signature {
    pub tick: i32,
    pub numerator: i32,
    pub denominator: i32,
}
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub kind: u32,
    pub tick: i32,
    pub end_tick: i32,
    pub rhythms: Vec<f64>,
}
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Position {
    pub bar: i32,
    pub rhythm: i32,
    pub bar_ticks: i32,
    pub time_ms: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Statistics {
    pub source_judgements: usize,
    pub derived_combos: usize,
    pub skipped_combos: usize,
    pub reconstructed_full_combo: usize,
}
pub struct Score {
    handle: NonNull<ffi::moenotes_score_t>,
    pub notes: BTreeMap<i32, Note>,
    pub branches: Vec<Branch>,
    pub bpms: Vec<Bpm>,
    pub signatures: Vec<Signature>,
    pub events: Vec<Event>,
    pub warnings: u32,
    pub mirror: bool,
    pub statistics: Statistics,
}
fn checked(code: u32) -> Result<()> {
    if code != 0 {
        bail!("C parser query failed ({code})");
    }
    Ok(())
}
impl Score {
    pub fn parse(data: &[u8], mirror: bool) -> Result<Self> {
        ensure!(
            data.len() <= 64 * 1024 * 1024,
            "Chart exceeds 64 MiB input limit"
        );
        // SAFETY: the version pointer is a static NUL-terminated C string.
        ensure!(
            unsafe { CStr::from_ptr(ffi::moenotes_version_string()) }.to_bytes() == b"0.3.0",
            "Parser must be v0.3.0"
        );
        let mut options = ffi::moenotes_parse_options_t::default();
        let mut handle = ptr::null_mut();
        let mut error = [0i8; 256];
        // SAFETY: all buffers live through parse, options have the generated ABI.
        unsafe {
            ffi::moenotes_default_parse_options(&mut options);
            options.mirror = u8::from(mirror);
            let result = ffi::moenotes_score_parse(
                data.as_ptr().cast::<c_void>(),
                data.len(),
                &options,
                ptr::null(),
                &mut handle,
                error.as_mut_ptr(),
                error.len(),
            );
            if result != 0 {
                bail!(
                    "Chart parse failed ({result}): {}",
                    CStr::from_ptr(error.as_ptr()).to_string_lossy()
                );
            }
        }
        let mut s = Self {
            handle: NonNull::new(handle).ok_or_else(|| anyhow::anyhow!("Parser returned null"))?,
            notes: BTreeMap::new(),
            branches: vec![],
            bpms: vec![],
            signatures: vec![],
            events: vec![],
            warnings: 0,
            mirror,
            statistics: Statistics {
                source_judgements: 0,
                derived_combos: 0,
                skipped_combos: 0,
                reconstructed_full_combo: 0,
            },
        };
        s.read_model()?;
        s.statistics = Self::statistics(data, mirror)?;
        Ok(s)
    }
    fn statistics(data: &[u8], mirror: bool) -> Result<Statistics> {
        let mut options = ffi::moenotes_parse_options_t::default();
        let mut p = ptr::null_mut();
        let mut error = [0i8; 256];
        // SAFETY: input/output live through C parse. Free on every return after
        // successful creation via the local guard.
        unsafe {
            ffi::moenotes_default_parse_options(&mut options);
            options.mirror = u8::from(mirror);
            options.slide_combo_unit = 8;
            let result = ffi::moenotes_score_parse(
                data.as_ptr().cast(),
                data.len(),
                &options,
                ptr::null(),
                &mut p,
                error.as_mut_ptr(),
                error.len(),
            );
            if result != 0 {
                bail!(
                    "Combo statistics failed ({result}): {}",
                    CStr::from_ptr(error.as_ptr()).to_string_lossy()
                );
            }
        }
        struct Guard(*mut ffi::moenotes_score_t);
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe {
                    ffi::moenotes_score_free(self.0);
                }
            }
        }
        let _guard = Guard(p);
        let mut stats = Statistics {
            source_judgements: 0,
            derived_combos: 0,
            skipped_combos: 0,
            reconstructed_full_combo: 0,
        };
        unsafe {
            stats.reconstructed_full_combo = ffi::moenotes_score_full_combo_count(p, 1, 0) as usize;
            for i in 0..ffi::moenotes_score_note_count(p) {
                let mut n = ffi::moenotes_note_view_t::default();
                checked(ffi::moenotes_score_note_at(p, i, &mut n))?;
                match n.operate_type {
                    120 => stats.derived_combos += 1,
                    121 => stats.skipped_combos += 1,
                    _ => {
                        stats.source_judgements += usize::from(
                            ffi::moenotes_operate_type_is_judgement(n.operate_type) != 0,
                        );
                    }
                }
            }
        }
        Ok(stats)
    }
    fn read_model(&mut self) -> Result<()> {
        let p = self.handle.as_ptr();
        // SAFETY: p is owned/live, output structs are generated from this header.
        unsafe {
            ensure!(ffi::moenotes_score_lane_count(p) == 24, "Expected 24 lanes");
            self.warnings = ffi::moenotes_score_warnings(p);
            ensure!(
                self.warnings & 1 == 0,
                "Nonmonotonic lines are not supported for preview"
            );
            let count = ffi::moenotes_score_note_count(p);
            ensure!(count <= 1_000_000, "Too many notes");
            for i in 0..count {
                let mut n = ffi::moenotes_note_view_t::default();
                let mut source = n;
                checked(ffi::moenotes_score_note_at(p, i, &mut n))?;
                checked(ffi::moenotes_score_source_note_at(p, i, &mut source))?;
                self.notes.insert(
                    n.id,
                    Note {
                        id: n.id,
                        kind: n.operate_type,
                        tick: n.tick,
                        time_ms: n.position.time_ms,
                        bar: n.position.bar,
                        left: n.lane_start_float,
                        right: n.lane_end_float + 1.,
                        width: n.width,
                        source_left: source.lane_start_float,
                        source_right: source.lane_end_float + 1.,
                        source_width: source.width,
                        direction: n.direction,
                        critical: n.critical != 0,
                        auto: n.pos_auto != 0,
                        slide_along: n.slide_along != 0,
                        pair: n.pair_note_id,
                        alpha: n.alpha,
                        ease_left: n.ease_left,
                        ease_right: n.ease_right,
                    },
                );
            }
            for i in 0..ffi::moenotes_score_line_count(p) {
                let mut l = ffi::moenotes_line_view_t::default();
                checked(ffi::moenotes_score_line_at(p, i, &mut l))?;
                let mut members = vec![];
                for j in 0..ffi::moenotes_score_line_member_count(p, l.id) {
                    let mut n = ffi::moenotes_note_view_t::default();
                    checked(ffi::moenotes_score_line_member_at(p, l.id, j, &mut n))?;
                    members.push(n.id);
                }
                self.branches.push(Branch {
                    id: l.id,
                    guide: l.guide != 0,
                    members,
                });
            }
            for i in 0..ffi::moenotes_score_bpm_count(p) {
                let mut v = ffi::moenotes_bpm_event_t::default();
                checked(ffi::moenotes_score_bpm_at(p, i, &mut v))?;
                self.bpms.push(Bpm {
                    tick: v.tick,
                    bpm: v.bpm,
                });
            }
            for i in 0..ffi::moenotes_score_signature_count(p) {
                let mut v = ffi::moenotes_signature_event_t::default();
                checked(ffi::moenotes_score_signature_at(p, i, &mut v))?;
                self.signatures.push(Signature {
                    tick: v.tick,
                    numerator: v.numerator,
                    denominator: v.denominator,
                });
            }
            for i in 0..ffi::moenotes_score_event_count(p) {
                let mut v = ffi::moenotes_event_t::default();
                checked(ffi::moenotes_score_event_at(p, i, &mut v))?;
                let mut rhythms = vec![];
                for j in 0..ffi::moenotes_score_call_rhythm_count(p, i) {
                    let mut x = 0.;
                    checked(ffi::moenotes_score_call_rhythm_at(p, i, j, &mut x))?;
                    rhythms.push(x);
                }
                self.events.push(Event {
                    kind: v.type_,
                    tick: v.tick,
                    end_tick: v.end_tick,
                    rhythms,
                });
            }
        }
        Ok(())
    }
    pub fn position(&self, tick: i32) -> Result<Position> {
        let mut p = ffi::moenotes_position_t::default();
        // SAFETY: immutable query on the owned live score, valid output pointer.
        checked(unsafe {
            ffi::moenotes_score_position_at_tick(self.handle.as_ptr(), tick, &mut p)
        })?;
        Ok(Position {
            bar: p.bar,
            rhythm: p.rhythm,
            bar_ticks: p.rhythmic_unit,
            time_ms: p.time_ms,
        })
    }
    pub fn note_time(&self, tick: i32) -> Result<i32> {
        let mut p = ffi::moenotes_position_t::default();
        // SAFETY: read-only query on the live owned score.
        checked(unsafe {
            ffi::moenotes_score_note_position_at_tick(self.handle.as_ptr(), tick, &mut p)
        })?;
        Ok(p.time_ms)
    }
    pub fn sample(&self, line: i32, tick: i32) -> Result<(f64, f64)> {
        let mut v = ffi::moenotes_line_sample_t::default();
        // SAFETY: library checks index/range; outputs used only on success.
        checked(unsafe {
            ffi::moenotes_score_sample_line(self.handle.as_ptr(), line, tick, &mut v)
        })?;
        ensure!(
            v.lane_start.is_finite() && v.lane_end.is_finite(),
            "Nonfinite line geometry"
        );
        Ok((v.lane_start, v.lane_end + 1.))
    }
    pub fn max_tick(&self) -> i32 {
        self.notes
            .values()
            .map(|n| n.tick)
            .chain(self.events.iter().flat_map(|e| [e.tick, e.end_tick]))
            .chain(self.bpms.iter().map(|e| e.tick))
            .chain(self.signatures.iter().map(|e| e.tick))
            .max()
            .unwrap_or(0)
    }
}
impl Drop for Score {
    fn drop(&mut self) {
        // SAFETY: handle is uniquely owned, and freed exactly once.
        unsafe {
            ffi::moenotes_score_free(self.handle.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_source_and_mirror_remain_separate() -> Result<()> {
        let data=br#"{"events":{},"notes":[{"type":"long","node":[{"t":0,"pos":0,"size":6,"ease":["out","in"]},{"t":240,"pos":"auto"},{"t":480,"pos":0,"size":12}]}]}"#;
        let s = Score::parse(data, true)?;
        let sample = s.sample(0, 240)?;
        assert_eq!(sample, (16.5, 24.));
        let auto = s.notes.values().find(|n| n.auto).unwrap();
        assert_ne!(auto.left, auto.source_left);
        assert_eq!(s.position(480)?.time_ms, 500);
        Ok(())
    }
    #[test]
    fn empty_and_invalid() {
        assert!(Score::parse(br#"{"events":{},"notes":[]}"#, false).is_ok());
        assert!(Score::parse(b"legacy SUS", false).is_err());
    }
}
