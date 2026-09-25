//! Beat-accurate balanced columns. No stretching of time to equalize column heights.
use crate::parser::Score;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
pub const HEADER: f64 = 184.;
pub const FOOTER: f64 = 66.;
pub const LEFT: f64 = 28.;
pub const RIGHT: f64 = 60.;
pub const GAP: f64 = 12.;
pub const PAD: f64 = 26.;
pub const MIN_SHEET_WIDTH: f64 = 360.;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// White paper, restrained fills and dark outlines for print readability.
    #[default]
    #[serde(rename = "white", alias = "print")]
    Print,
    /// Deep neutral black counterpart to the white preset.
    Black,
    /// Legacy blue-black screen palette.
    Dark,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlickLayout {
    #[default]
    Callout,
    Inline,
}

pub(crate) fn body_height(role: &str, requested: f64) -> f64 {
    requested
        * match role {
            "trace" => 0.72,
            "connection" => 0.53,
            _ => 1.,
        }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CurveMode {
    #[default]
    Musical,
    NativeParameters,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    pub theme: Theme,
    pub flick_layout: FlickLayout,
    /// Zero selects target-height balanced columns, positive is explicit legacy bar count.
    pub bars_per_column: usize,
    pub pixels_per_beat: f64,
    pub pixels_per_lane: f64,
    pub long: bool,
    /// Reserved compatibility field. Must be zero; charts are never paginated.
    pub columns_per_page: usize,
    pub target_beats: f64,
    pub note_height: f64,
    pub arrow_height: f64,
    pub supersample: u32,
    /// Export pixel multiplier; leaves logical chart layout and ticks unchanged.
    pub output_scale: f64,
    pub curve_mode: CurveMode,
    pub strict_assets: bool,
    pub native_critical: bool,
    pub auto_spacing: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            theme: Theme::Print,
            flick_layout: FlickLayout::Callout,
            bars_per_column: 0,
            pixels_per_beat: 64.,
            pixels_per_lane: 10.,
            long: false,
            columns_per_page: 0,
            target_beats: 24.,
            note_height: 8.,
            arrow_height: 10.,
            supersample: 2,
            output_scale: 1.,
            curve_mode: CurveMode::Musical,
            strict_assets: false,
            native_critical: false,
            auto_spacing: true,
        }
    }
}
impl Options {
    /// Validate the actual PNG and supersampled allocations, not logical units.
    pub(crate) fn export_dimensions(&self, width: f64, height: f64) -> Result<(i32, i32)> {
        ensure!(
            width.is_finite() && height.is_finite() && width > 0. && height > 0.,
            "Invalid logical image dimensions"
        );
        let w = (width * self.output_scale).ceil();
        let h = (height * self.output_scale).ceil();
        ensure!(
            w >= 1. && h >= 1. && w <= 32768. && h <= 32768. && w * h <= 64_000_000.,
            "Scaled PNG exceeds image limits; reduce output-scale"
        );
        ensure!(
            w * h * f64::from(self.supersample).powi(2) <= 256_000_000.,
            "Supersampled PNG exceeds memory limit; reduce output-scale or supersample"
        );
        Ok((w as i32, h as i32))
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Bar {
    pub tick: i32,
    pub number: i32,
    pub time_ms: i32,
}
#[derive(Debug, Clone, Serialize)]
pub struct Column {
    pub start: i32,
    pub end: i32,
    pub first_bar: i32,
    pub last_bar: i32,
}
#[derive(Debug, Clone, Serialize)]
pub struct Layout {
    pub options: Options,
    pub bars: Vec<Bar>,
    pub columns: Vec<Column>,
    pub column_width: f64,
    pub height: i32,
    pub track_height: f64,
    pub unused_vertical_fraction: f64,
}
impl Layout {
    pub fn build(score: &Score, mut options: Options) -> Result<Self> {
        ensure!(
            options.columns_per_page == 0,
            "Pagination is disabled; render a complete multi-column image"
        );
        ensure!(
            options.bars_per_column <= 64,
            "bars-per-column must be 1..64 when supplied"
        );
        ensure!(
            (4. ..=160.).contains(&options.pixels_per_beat),
            "pixels-per-beat must be 4..160"
        );
        ensure!(
            (2. ..=32.).contains(&options.pixels_per_lane),
            "pixels-per-lane must be 2..32"
        );
        ensure!(
            (4. ..=128.).contains(&options.target_beats),
            "target-beats must be 4..128"
        );
        ensure!(
            (3. ..=20.).contains(&options.note_height),
            "note-height must be 3..20"
        );
        ensure!(
            (3. ..=30.).contains(&options.arrow_height),
            "arrow-height must be 3..30"
        );
        ensure!(
            (0.25..=4.).contains(&options.output_scale),
            "output-scale must be 0.25..4"
        );
        ensure!(
            (1..=3).contains(&options.supersample),
            "supersample must be 1..3"
        );
        if options.auto_spacing {
            let mut notes: Vec<_> = score
                .notes
                .values()
                .filter(|n| n.role().is_some() && n.width > 0.)
                .collect();
            notes.sort_by_key(|n| n.tick);
            let mut spacing = options.pixels_per_beat;
            for (i, a) in notes.iter().enumerate() {
                for b in notes.iter().skip(i + 1) {
                    let delta = b.tick - a.tick;
                    if delta > 480 {
                        break;
                    }
                    if delta > 0 && a.left < b.right && b.left < a.right {
                        let ar = a.role().unwrap();
                        let br = b.role().unwrap();
                        // Structural slide anchors may deliberately be 1–2 ticks
                        // apart. Preserve their coordinates without stretching the
                        // entire chart in an unsuccessful attempt to separate them.
                        if ar == "connection" && br == "connection" && delta <= 2 {
                            continue;
                        }
                        // Reserve the full requested height: external packs may
                        // use full-height connection/trace sprites.
                        let mut gap = options.note_height + 3.;
                        if ar.starts_with("flick") {
                            let center = (a.left + a.right) / 2.;
                            let half = (a.width * options.pixels_per_lane).min(24.)
                                / options.pixels_per_lane
                                / 2.;
                            if b.left < center + half && b.right > center - half {
                                gap += options.arrow_height + 1.5;
                            }
                        }
                        spacing = spacing.max(gap * 480. / delta as f64);
                    }
                }
            }
            options.pixels_per_beat = spacing.min(160.);
        }
        let mut bars = vec![];
        let mut tick = 0;
        loop {
            ensure!(bars.len() < 4096, "Chart exceeds 4096 layout bars");
            let p = score.position(tick)?;
            bars.push(Bar {
                tick,
                number: p.bar + 1,
                time_ms: p.time_ms,
            });
            if tick > score.max_tick() {
                break;
            }
            let mut end = tick
                .checked_add(p.bar_ticks.checked_sub(p.rhythm).context("Invalid meter")?)
                .context("Tick overflow")?;
            if let Some(s) = score
                .signatures
                .iter()
                .find(|s| s.tick > tick && s.tick < end)
            {
                end = s.tick;
            }
            ensure!(end > tick, "Nonadvancing measure");
            tick = end;
        }
        let count = bars.len() - 1;
        let cuts = if options.long {
            vec![0, count]
        } else if options.bars_per_column > 0 {
            (0..count)
                .step_by(options.bars_per_column)
                .chain([count])
                .collect()
        } else {
            balanced_cuts(&bars, options.target_beats * 480.)
        };
        let columns: Vec<_> = cuts
            .windows(2)
            .map(|w| Column {
                start: bars[w[0]].tick,
                end: bars[w[1]].tick,
                first_bar: bars[w[0]].number,
                last_bar: bars[w[1] - 1].number,
            })
            .collect();
        let track_height = columns
            .iter()
            .map(|c| (c.end - c.start) as f64 / 480. * options.pixels_per_beat)
            .fold(0., f64::max);
        let column_width = LEFT + 24. * options.pixels_per_lane + RIGHT + GAP;
        let height = (HEADER + PAD * 2. + track_height + FOOTER).ceil();
        ensure!(
            height <= i32::MAX as f64,
            "Logical image height exceeds coordinate range"
        );
        let used =
            (bars.last().unwrap().tick - bars[0].tick) as f64 / 480. * options.pixels_per_beat;
        let unused = 1. - used / (columns.len() as f64 * track_height);
        let l = Self {
            options,
            bars,
            columns,
            column_width,
            height: height as i32,
            track_height,
            unused_vertical_fraction: unused,
        };
        for range in l.pages() {
            l.page_width(range.len())?;
        }
        Ok(l)
    }
    pub fn owner(&self, tick: i32) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.start <= tick && tick < c.end)
    }
    pub fn y(&self, c: &Column, tick: f64) -> f64 {
        HEADER + PAD + self.track_height
            - (tick - c.start as f64) / 480. * self.options.pixels_per_beat
    }
    pub fn pages(&self) -> Vec<std::ops::Range<usize>> {
        std::iter::once(0..self.columns.len()).collect()
    }
    pub fn page_width(&self, n: usize) -> Result<i32> {
        let w = (n as f64 * self.column_width + GAP)
            .ceil()
            .max(MIN_SHEET_WIDTH);
        ensure!(
            w <= i32::MAX as f64,
            "Logical image width exceeds coordinate range"
        );
        self.options.export_dimensions(w, self.height as f64)?;
        Ok(w as i32)
    }
}
/// Minimize per-column height variance for a fixed near-target column count.
/// Dynamic programming considers all bar boundaries; no musical time is warped.
fn balanced_cuts(bars: &[Bar], target: f64) -> Vec<usize> {
    let n = bars.len() - 1;
    let total = (bars[n].tick - bars[0].tick) as f64;
    let k = ((total / target).ceil() as usize)
        .clamp(1, n)
        .max(n.div_ceil(128));
    // Bound DP allocation/work for pathological tiny-meter charts.
    if k.saturating_mul(n) > 1_000_000 {
        let mut cuts = vec![0];
        let mut last = 0;
        for part in 1..k {
            let goal = total * part as f64 / k as f64;
            let lo = last + 1;
            let hi = n - (k - part);
            let end = (lo..=hi)
                .min_by(|a, b| {
                    ((bars[*a].tick - bars[0].tick) as f64 - goal)
                        .abs()
                        .total_cmp(&((bars[*b].tick - bars[0].tick) as f64 - goal).abs())
                })
                .unwrap();
            cuts.push(end);
            last = end;
        }
        cuts.push(n);
        return cuts;
    }
    let ideal = total / k as f64;
    let mut dp = vec![vec![f64::INFINITY; n + 1]; k + 1];
    let mut prev = vec![vec![0; n + 1]; k + 1];
    dp[0][0] = 0.;
    // Only candidates around each ideal cumulative boundary are necessary for
    // this ordered, convex cost; bound work while retaining large-meter bars.
    for c in 1..=k {
        for end in c..=n - (k - c) {
            let max_back = 128.min(end);
            for start in (end - max_back)..end {
                if !dp[c - 1][start].is_finite() {
                    continue;
                }
                let span = (bars[end].tick - bars[start].tick) as f64;
                let cost = dp[c - 1][start] + (span - ideal).powi(2);
                if cost < dp[c][end] {
                    dp[c][end] = cost;
                    prev[c][end] = start;
                }
            }
        }
    }
    let mut cuts = vec![n];
    let mut end = n;
    for c in (1..=k).rev() {
        end = prev[c][end];
        cuts.push(end);
    }
    cuts.reverse();
    cuts
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_limits_use_scaled_pixels_including_rounding_and_supersampling() -> Result<()> {
        let options = Options {
            output_scale: 0.5,
            supersample: 3,
            ..Options::default()
        };
        assert_eq!(options.export_dimensions(20412., 4142.)?, (10206, 2071));
        assert_eq!(options.export_dimensions(40000., 302.)?, (20000, 151));
        assert_eq!(options.export_dimensions(360., 40000.)?, (180, 20000));
        // 32M pixels pass the final budget but exceed 256M at 3x supersampling.
        assert!(options.export_dimensions(16000., 8000.).is_err());
        let rounded = Options {
            output_scale: 0.5,
            supersample: 1,
            ..Options::default()
        };
        assert_eq!(rounded.export_dimensions(31999., 8000.)?, (16000, 4000));
        assert!(rounded.export_dimensions(32001., 8000.).is_err());
        assert!(rounded.export_dimensions(65537., 1.).is_err());
        Ok(())
    }
    #[test]
    fn downscaling_allows_long_and_wide_logical_layouts() -> Result<()> {
        let score = Score::parse(br#"{"events":{},"notes":[{"t":479999}]}"#, false)?;
        for long in [false, true] {
            let layout = Layout::build(
                &score,
                Options {
                    long,
                    pixels_per_beat: 64.,
                    output_scale: 0.25,
                    supersample: 1,
                    bars_per_column: if long { 0 } else { 1 },
                    ..Options::default()
                },
            )?;
            assert!(if long {
                layout.height > 32768
            } else {
                layout.page_width(layout.columns.len())? > 32768
            });
        }
        Ok(())
    }
    #[test]
    fn nearby_structural_anchors_do_not_stretch_the_sheet() -> Result<()> {
        let s = Score::parse(br#"{"events":{},"notes":[{"type":"long","node":[{"t":0,"pos":0,"size":6},{"t":480,"pos":0,"size":6},{"t":481,"pos":0,"size":6},{"t":1920,"pos":0,"size":6}]}]}"#, false)?;
        let l = Layout::build(&s, Options::default())?;
        assert_eq!(l.options.pixels_per_beat, 64.);
        assert!(s.notes.values().any(|n| n.tick == 481));
        Ok(())
    }
    #[test]
    fn auto_spacing_reserves_flick_arrow_clearance() -> Result<()> {
        let s = Score::parse(
            br#"{"events":{},"notes":[{"type":"flick","t":480,"pos":4,"size":6},{"t":600,"pos":4,"size":6}]}"#,
            false,
        )?;
        let l = Layout::build(&s, Options::default())?;
        assert!(l.options.pixels_per_beat * 120. / 480. >= 22.5);
        let fixed = Layout::build(
            &s,
            Options {
                auto_spacing: false,
                ..Options::default()
            },
        )?;
        assert_eq!(fixed.options.pixels_per_beat, 64.);
        Ok(())
    }
    #[test]
    fn meter_and_cut_ownership() -> Result<()> {
        let s=Score::parse(br#"{"events":{"sig":[{"t":0,"sig":[3,4]},{"t":2880,"sig":[4,4]}]},"notes":[{"t":4800}]}"#,false)?;
        let l = Layout::build(
            &s,
            Options {
                bars_per_column: 2,
                ..Options::default()
            },
        )?;
        assert_eq!(
            l.bars.iter().map(|b| b.tick).collect::<Vec<_>>(),
            [0, 1440, 2880, 4800, 6720]
        );
        assert_eq!(l.owner(2880), Some(1));
        Ok(())
    }
    #[test]
    fn empty_has_one_measure() -> Result<()> {
        let s = Score::parse(br#"{"events":{},"notes":[]}"#, false)?;
        assert_eq!(Layout::build(&s, Options::default())?.columns[0].end, 1920);
        Ok(())
    }
    #[test]
    fn target_columns_balance_meter_changes() -> Result<()> {
        let s=Score::parse(br#"{"events":{"sig":[{"t":0,"sig":[3,4]},{"t":14400,"sig":[5,4]}]},"notes":[{"t":38000}]}"#,false)?;
        let l = Layout::build(&s, Options::default())?;
        assert!(l.unused_vertical_fraction < 0.16);
        assert_eq!(l.pages().len(), 1);
        assert_eq!(l.pages()[0].len(), l.columns.len());
        Ok(())
    }
}
