//! Responsive sheet furniture. The score's tick/lane geometry is not reflowed.
use super::*;

pub(super) struct Header {
    pub height: f64,
    pub scale: f64,
    margin: f64,
    compact: bool,
    cover_size: f64,
    hero_top: f64,
    text_x: f64,
    text_width: f64,
    title_size: f32,
    title_lines: Vec<String>,
    artist_y: f64,
    stats_y: f64,
    stats_wrap: bool,
    credit_y: f64,
    credit_lines: Vec<String>,
    legend_y: f64,
    /// Left edge of the chart-facts row, when it fits beside the title.
    facts_x: Option<f64>,
    facts_cell: f64,
}

impl Header {
    pub fn plan(width: i32, fonts: &Fonts, metadata: &Metadata, cover: bool) -> Self {
        let width = width as f64;
        let compact = width < 760.;
        let scale = (width / 1500.).clamp(1., 2.5);
        let margin = if compact { 20. } else { 24. * scale };
        let hero_top = if compact { 82. } else { margin };
        let cover_size = if compact { 64. } else { 76. * scale };
        let text_x = margin + if cover { cover_size + 18. * scale } else { 0. };
        let brand_x = width - margin - 210. * scale;
        let text_width = width - margin - text_x - if compact { 0. } else { 240. * scale };
        let title_size = if compact { 22. } else { 30. * scale as f32 };
        // Chart facts sit between the title and the brand when six cells fit
        // after the measured title/artist line; otherwise they are omitted.
        let facts_cell = 96. * scale;
        let facts_width = 6. * facts_cell;
        let measured = fonts
            .width(&metadata.title, title_size, true)
            .max(fonts.width(&metadata.artist, 15. * scale as f32, false))
            as f64;
        let facts_x = (!compact)
            .then_some(brand_x - 36. * scale - facts_width)
            .filter(|&fx| fx - 44. * scale >= text_x + measured);
        let text_width = match facts_x {
            Some(fx) => (fx - 44. * scale - text_x).min(text_width),
            None => text_width,
        };
        let title_lines = fonts.lines(&metadata.title, title_size, true, text_width as f32, 2);
        let title_bottom = hero_top
            + title_size as f64
            + (title_lines.len().saturating_sub(1)) as f64 * title_size as f64 * 1.2;
        let artist_y = title_bottom + 23. * scale;
        let stats_y = (if metadata.artist.is_empty() {
            title_bottom
        } else {
            artist_y
        })
        .max(hero_top + if cover { cover_size } else { 0. })
        // The independent wordmark/site block is present even without a cover
        // or artist. Keep full-width statistics below its underline.
        .max(if compact { 0. } else { hero_top + 76. * scale })
            + 22. * scale;
        let stats_wrap = compact;
        let credit_y = stats_y + if stats_wrap { 85. } else { 56. * scale };
        let credit_lines = fonts.lines(
            &metadata.author,
            13. * scale as f32,
            false,
            (width - 2. * margin) as f32,
            2,
        );
        let legend_y = credit_y + credit_lines.len() as f64 * 19. * scale + 19. * scale;
        let legend_rows = 7usize.div_ceil(legend_cells(width)) as f64;
        let height = legend_y + legend_rows * 28. * scale + 18. * scale;
        Self {
            height,
            scale,
            margin,
            compact,
            cover_size,
            hero_top,
            text_x,
            text_width,
            title_size,
            title_lines,
            artist_y,
            stats_y,
            stats_wrap,
            credit_y,
            credit_lines,
            legend_y,
            facts_x,
            facts_cell,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        canvas: &Canvas,
        fonts: &Fonts,
        skin: &Skin,
        palette: &Palette,
        scene: &Scene,
        metadata: &Metadata,
        cover: Option<&sk::Image>,
        width: i32,
        mirror: bool,
    ) -> Result<()> {
        let c = canvas;
        let s = self.scale;
        let margin = self.margin;
        let width = width as f64;
        let accent = if scene.layout.options.theme == Theme::Print {
            Color::from_rgb(37, 78, 118)
        } else {
            Color::from_rgb(164, 201, 235)
        };
        if self.compact {
            fonts.draw(c, "moenotes", margin, 42., 26., palette.text, true, 180.);
            let site_width = fonts.width("bdon.moe", 15., false);
            fonts.draw(
                c,
                "bdon.moe",
                width - margin - site_width as f64,
                40.,
                15.,
                accent,
                false,
                site_width + 1.,
            );
            stroke(c, (margin, 61.), (width - margin, 61.), palette.rule, 1.);
        } else {
            let bx = width - margin - 210. * s;
            fonts.draw(
                c,
                "moenotes",
                bx,
                self.hero_top + 35. * s,
                29. * s as f32,
                palette.text,
                true,
                210. * s as f32,
            );
            fonts.draw(
                c,
                "bdon.moe",
                bx,
                self.hero_top + 66. * s,
                20. * s as f32,
                accent,
                false,
                210. * s as f32,
            );
            stroke(
                c,
                (bx, self.hero_top + 82. * s),
                (bx + 62. * s, self.hero_top + 82. * s),
                accent,
                2.,
            );
        }
        if let Some(image) = cover {
            let side = image.width().min(image.height()) as f32;
            let src = Rect::from_xywh(
                (image.width() as f32 - side) / 2.,
                (image.height() as f32 - side) / 2.,
                side,
                side,
            );
            let dest = Rect::from_xywh(
                margin as f32,
                self.hero_top as f32,
                self.cover_size as f32,
                self.cover_size as f32,
            );
            c.draw_image_rect_with_sampling_options(
                image,
                Some((&src, sk::canvas::SrcRectConstraint::Strict)),
                dest,
                sk::FilterMode::Linear,
                &Paint::default(),
            );
            let mut edge = paint(palette.rule);
            edge.set_style(sk::paint::Style::Stroke)
                .set_stroke_width(1.);
            c.draw_rect(dest, &edge);
        }
        for (i, line) in self.title_lines.iter().enumerate() {
            fonts.draw(
                c,
                line,
                self.text_x,
                self.hero_top + self.title_size as f64 * (1. + i as f64 * 1.2),
                self.title_size,
                palette.text,
                true,
                self.text_width as f32,
            );
        }
        if !metadata.artist.is_empty() {
            fonts.draw(
                c,
                &metadata.artist,
                self.text_x,
                self.artist_y,
                15. * s as f32,
                palette.muted,
                false,
                self.text_width as f32,
            );
        }
        let difficulty = if metadata.difficulty.is_empty() {
            "CHART"
        } else {
            &metadata.difficulty
        };
        let badge_width = 176. * s;
        let tone = difficulty_colors(difficulty, scene.layout.options.theme);
        let (badge_fill, badge_text) = match tone {
            Some((fill, text)) => (paint(fill), text),
            None => {
                let mut fill = paint(accent);
                fill.set_alpha_f(if scene.layout.options.theme == Theme::Print {
                    0.075
                } else {
                    0.12
                });
                (fill, accent)
            }
        };
        let badge = Rect::from_xywh(
            margin as f32,
            self.stats_y as f32,
            badge_width as f32,
            36. * s as f32,
        );
        c.draw_round_rect(badge, 4. * s as f32, 4. * s as f32, &badge_fill);
        fonts.draw(
            c,
            difficulty,
            margin + 12. * s,
            self.stats_y + 24. * s,
            13. * s as f32,
            badge_text,
            true,
            100. * s as f32,
        );
        // Keep the master level text verbatim; shrink decimal levels such as
        // "27.5" to the badge's number field instead of truncating them.
        let level_space = 54. * s as f32;
        let level_size = (23. * s as f32)
            .min(
                23. * s as f32 * level_space
                    / fonts.width(&metadata.level, 23. * s as f32, true).max(1.),
            )
            .max(15. * s as f32);
        fonts.draw(
            c,
            &metadata.level,
            margin + 117. * s,
            self.stats_y + 27. * s,
            level_size,
            badge_text,
            true,
            level_space + 1.,
        );
        let bpm = metric_bpm(scene);
        let fc = metadata
            .master_full_combo
            .unwrap_or(scene.statistics.reconstructed_full_combo as u64);
        let stats = if self.stats_wrap {
            format!("{bpm}   ·   COMBO {fc}")
        } else {
            format!("{bpm}     ·     TOTAL COMBO {fc}")
        };
        let sx = if self.stats_wrap {
            margin
        } else {
            margin + badge_width + 24. * s
        };
        let sy = self.stats_y + if self.stats_wrap { 62. } else { 25. * s };
        fonts.draw(
            c,
            &stats,
            sx,
            sy,
            if self.stats_wrap { 14. } else { 17. * s as f32 },
            palette.text,
            true,
            (width
                - margin
                - sx
                - if mirror && !self.stats_wrap {
                    86. * s
                } else {
                    0.
                }) as f32,
        );
        if mirror {
            fonts.draw(
                c,
                "MIRROR",
                width - margin - 75. * s,
                self.stats_y + 24. * s,
                12. * s as f32,
                accent,
                true,
                75. * s as f32,
            );
        }
        if let Some(panel_x) = self.facts_x {
            // Chart facts fill the space between the title block and the brand:
            // label above value, in text ink; no color carries meaning here.
            let facts = SheetStats::of(scene);
            let cell = self.facts_cell;
            for (i, (label, value)) in facts.items().iter().enumerate() {
                let fx = panel_x + i as f64 * cell;
                fonts.draw(
                    c,
                    label,
                    fx,
                    self.hero_top + 22. * s,
                    10.5 * s as f32,
                    palette.muted,
                    false,
                    (cell - 8. * s) as f32,
                );
                fonts.draw(
                    c,
                    value,
                    fx,
                    self.hero_top + 52. * s,
                    22. * s as f32,
                    palette.text,
                    true,
                    (cell - 8. * s) as f32,
                );
            }
            stroke(
                c,
                (panel_x - 20. * s, self.hero_top + 4. * s),
                (panel_x - 20. * s, self.hero_top + 60. * s),
                palette.rule,
                1.,
            );
        }
        for (i, line) in self.credit_lines.iter().enumerate() {
            fonts.draw(
                c,
                line,
                margin,
                self.credit_y + i as f64 * 19. * s,
                13. * s as f32,
                palette.muted,
                false,
                (width - 2. * margin) as f32,
            );
        }
        let labels = [
            ("tap", "TAP"),
            ("slide", "SLIDE"),
            ("flick", "FLICK"),
            ("trace", "TRACE"),
            ("tap", "CRITICAL"),
            ("guide", "GUIDE"),
            ("fever", "FEVER"),
        ];
        let cells = legend_cells(width);
        let cell_width = ((width - 2. * margin) / cells as f64).min(125. * s);
        for (i, (role, label)) in labels.iter().enumerate() {
            let x = margin + (i % cells) as f64 * cell_width;
            let y = self.legend_y + (i / cells) as f64 * 28. * s;
            if *role == "guide" {
                let mut p = paint(palette.muted);
                p.set_style(sk::paint::Style::Stroke)
                    .set_stroke_width(s as f32);
                if scene.layout.options.theme == Theme::Print {
                    p.set_path_effect(sk::PathEffect::dash(&[3. * s as f32, 3. * s as f32], 0.));
                }
                c.draw_rect(
                    Rect::from_xywh(x as f32, (y - 7. * s) as f32, 22. * s as f32, 7. * s as f32),
                    &p,
                );
            } else if *role == "fever" {
                c.draw_rect(
                    Rect::from_xywh(
                        (x + 4. * s) as f32,
                        (y - 11. * s) as f32,
                        19. * s as f32,
                        13. * s as f32,
                    ),
                    &paint(if scene.layout.options.theme == Theme::Print {
                        PRINT_FEVER
                    } else {
                        BLACK_FEVER
                    }),
                );
                stroke(
                    c,
                    (x + 2. * s, y - 11. * s),
                    (x + 2. * s, y + 2. * s),
                    FEVER_RAIL,
                    2.5 * s as f32,
                );
            } else {
                c.save();
                c.translate((x as f32, (y - 4. * s) as f32));
                c.scale((s as f32, s as f32));
                let note = PreviewNote {
                    role,
                    width_lanes: 3.,
                    center: (11., 0.),
                    lane_px: 7.,
                    body_height: 5.,
                    arrow_height: 5.,
                    critical: *label == "CRITICAL",
                    native_critical: scene.layout.options.native_critical,
                    strict_assets: scene.layout.options.strict_assets,
                };
                for part in [NotePart::Body, NotePart::Arrow, NotePart::Mark] {
                    draw_note_part(c, skin, &note, part, scene.layout.options.theme)?;
                }
                c.restore();
            }
            fonts.draw(
                c,
                label,
                x + 31. * s,
                y,
                10. * s as f32,
                palette.muted,
                false,
                (cell_width - 33. * s) as f32,
            );
        }
        stroke(
            c,
            (margin, self.height - 34. * s),
            (width - margin, self.height - 34. * s),
            palette.rule,
            1.,
        );
        Ok(())
    }
}

fn legend_cells(width: f64) -> usize {
    if width < 690. {
        3
    } else if width < 900. {
        4
    } else {
        7
    }
}

/// Header facts computed from the parsed chart. Counts are visible glyphs by
/// kind; critical is a flag across kinds, so it is listed separately.
pub(super) struct SheetStats {
    pub duration_ms: i32,
    pub tap: usize,
    pub slide: usize,
    pub flick: usize,
    pub trace: usize,
    pub critical: usize,
}
impl SheetStats {
    pub fn of(scene: &Scene) -> Self {
        let mut s = Self {
            duration_ms: scene
                .glyphs
                .iter()
                .map(|g| g.note.time_ms)
                .max()
                .unwrap_or(0)
                .max(0),
            tap: 0,
            slide: 0,
            flick: 0,
            trace: 0,
            critical: 0,
        };
        for g in &scene.glyphs {
            match g.note.role() {
                Some("tap") => s.tap += 1,
                Some("slide" | "connection" | "slide_end") => s.slide += 1,
                Some(r) if r.starts_with("flick") => s.flick += 1,
                Some("trace") => s.trace += 1,
                _ => {}
            }
            s.critical += usize::from(g.note.critical);
        }
        s
    }
    pub fn duration(&self) -> String {
        let seconds = self.duration_ms / 1000;
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
    pub fn items(&self) -> [(&'static str, String); 6] {
        [
            ("LENGTH", self.duration()),
            ("TAP", self.tap.to_string()),
            ("SLIDE", self.slide.to_string()),
            ("FLICK", self.flick.to_string()),
            ("TRACE", self.trace.to_string()),
            ("CRITICAL", self.critical.to_string()),
        ]
    }
}

/// Difficulty badge fill and text colors. Standard names get an ordered hue;
/// anything else keeps the neutral accent tint. The name is always printed.
pub(super) fn difficulty_colors(name: &str, theme: Theme) -> Option<(Color, Color)> {
    let print = theme == Theme::Print;
    let rgb = |h: u32| Color::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8);
    // Validated as an adjacent 4-slot categorical set on each surface
    // (dataviz validate_palette.js: light #ffffff / dark #08090c).
    let (fill, text) = match name.to_ascii_uppercase().as_str() {
        "EASY" => (
            if print { 0x256abf } else { 0x3987e5 },
            if print { 0xffffff } else { 0x0b0d12 },
        ),
        "NORMAL" => (
            if print { 0x008300 } else { 0x199e70 },
            if print { 0xffffff } else { 0x0b0d12 },
        ),
        "HARD" => (if print { 0xeda100 } else { 0xc98500 }, 0x111827),
        "EXPERT" => (
            if print { 0xc0302d } else { 0xe34948 },
            if print { 0xffffff } else { 0x0b0d12 },
        ),
        _ => return None,
    };
    Some((rgb(fill), rgb(text)))
}

pub(super) fn location(scene: &Scene, tick: i32) -> String {
    let index = scene
        .layout
        .bars
        .partition_point(|bar| bar.tick <= tick)
        .saturating_sub(1);
    let bar = &scene.layout.bars[index];
    let beat = 1. + (tick - bar.tick) as f64 / 480.;
    let beat = format!("{beat:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned();
    format!("BAR {:03} · BEAT {beat}", bar.number)
}

pub(super) fn footer(
    c: &Canvas,
    fonts: &Fonts,
    palette: &Palette,
    width: i32,
    height: i32,
    scale: f64,
) {
    // The header carries the moenotes / bdon.moe wordmark; the footer is a
    // single quiet credit line with the reading direction.
    let margin = if width < 760 { 20. } else { 24. * scale };
    let y = height as f64 - 29. * scale;
    stroke(
        c,
        (margin, y - 31. * scale),
        (width as f64 - margin, y - 31. * scale),
        palette.rule,
        1.,
    );
    let size = 12. * scale as f32;
    let credit = "moenotes  ·  bdon.moe";
    fonts.draw(
        c,
        credit,
        margin,
        y,
        size,
        palette.muted,
        false,
        (width as f64 - 2. * margin) as f32,
    );
    let direction = "TIME ↑   ·   COLUMNS →";
    let direction_width = fonts.width(direction, size, false) as f64;
    if margin + fonts.width(credit, size, false) as f64 + 24. * scale + direction_width
        <= width as f64 - margin
    {
        fonts.draw(
            c,
            direction,
            width as f64 - margin - direction_width,
            y,
            size,
            palette.muted,
            false,
            direction_width as f32 + 1.,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_song_statistics_clear_the_brand_block() -> Result<()> {
        let fonts = Fonts::builtin()?;
        let metadata = Metadata {
            title: "No cover".into(),
            ..Metadata::default()
        };
        for width in [760, 780, 1032, 4432] {
            let header = Header::plan(width, &fonts, &metadata, false);
            assert!(header.stats_y >= header.hero_top + 94. * header.scale);
            assert!(header.height > layout::HEADER);
        }
        Ok(())
    }

    #[test]
    fn chart_facts_fit_beside_title_or_are_omitted() -> Result<()> {
        let fonts = Fonts::builtin()?;
        let short = Metadata {
            title: "起死開戦".into(),
            artist: "millsage".into(),
            ..Metadata::default()
        };
        let wide = Header::plan(5348, &fonts, &short, true);
        let fx = wide.facts_x.expect("room for facts on a wide sheet");
        let brand_x = 5348. - wide.margin - 210. * wide.scale;
        assert!(fx + 6. * wide.facts_cell <= brand_x);
        // The title never runs under the facts row.
        assert!(wide.text_x + wide.text_width <= fx);
        assert!(Header::plan(700, &fonts, &short, true).facts_x.is_none());
        let long = Metadata {
            title: "A very long song title that fills the whole header line ".repeat(3),
            ..Metadata::default()
        };
        assert!(Header::plan(2400, &fonts, &long, true).facts_x.is_none());
        Ok(())
    }

    #[test]
    fn chart_facts_count_visible_notes_by_kind() -> Result<()> {
        let score = crate::parser::Score::parse(
            br#"{"events":{},"notes":[{"t":0,"pos":0,"size":4,"crit":true},{"type":"flick","dir":"left","t":480,"pos":4,"size":4},{"type":"trace","t":960,"pos":8,"size":4},{"type":"long","node":[{"t":1440,"pos":0,"size":4},{"t":1920,"pos":0,"size":4},{"t":2400,"pos":0,"size":4}]}]}"#,
            false,
        )?;
        let scene = Scene::build(
            &score,
            crate::layout::Layout::build(&score, crate::layout::Options::default())?,
        )?;
        let s = SheetStats::of(&scene);
        assert_eq!((s.tap, s.flick, s.trace, s.critical), (1, 1, 1, 1));
        assert_eq!(s.tap + s.slide + s.flick + s.trace, scene.glyphs.len());
        assert_eq!(s.slide, 3);
        assert_eq!(s.duration(), format!("0:{:02}", s.duration_ms / 1000));
        Ok(())
    }

    #[test]
    fn standard_difficulties_have_distinct_badges_and_others_stay_neutral() {
        for theme in [Theme::Print, Theme::Black] {
            let fills: Vec<_> = ["EASY", "NORMAL", "HARD", "EXPERT"]
                .iter()
                .map(|d| difficulty_colors(d, theme).unwrap().0)
                .collect();
            for (i, a) in fills.iter().enumerate() {
                assert!(fills[i + 1..].iter().all(|b| a != b));
            }
            assert_eq!(
                difficulty_colors("expert", theme),
                difficulty_colors("EXPERT", theme)
            );
            assert!(difficulty_colors("TEST", theme).is_none());
            assert!(difficulty_colors("", theme).is_none());
        }
    }

    #[test]
    fn annotation_locations_follow_measure_changes_and_quarter_beats() -> Result<()> {
        let score = crate::parser::Score::parse(br#"{"events":{"sig":[{"t":0,"sig":[3,4]},{"t":1440,"sig":[5,8]}]},"notes":[{"t":2640}]}"#, false)?;
        let scene = Scene::build(
            &score,
            crate::layout::Layout::build(&score, crate::layout::Options::default())?,
        )?;
        assert_eq!(location(&scene, 0), "BAR 001 · BEAT 1");
        assert_eq!(location(&scene, 240), "BAR 001 · BEAT 1.5");
        assert_eq!(location(&scene, 1440), "BAR 002 · BEAT 1");
        assert_eq!(location(&scene, 2640), "BAR 003 · BEAT 1");
        Ok(())
    }
}
