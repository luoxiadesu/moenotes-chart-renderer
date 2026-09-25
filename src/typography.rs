//! Pinned font fallback. No dependency on the host's font search order.
use anyhow::{Context, Result};
use skia_safe::{Canvas, Color, Data, Font, FontMgr, Paint};
use std::path::Path;
pub struct Fonts {
    faces: Vec<skia_safe::Typeface>,
    bold: skia_safe::Typeface,
}
impl Fonts {
    pub fn builtin() -> Result<Self> {
        let mgr = FontMgr::new();
        let load = |bytes: &[u8]| {
            mgr.new_from_data(Data::new_copy(bytes), None)
                .context("Embedded font decode")
        };
        Ok(Self {
            faces: vec![
                load(include_bytes!("../assets/fonts/DejaVuSans.ttf"))?,
                load(include_bytes!("../assets/fonts/ipag.ttf"))?,
                load(include_bytes!("../assets/fonts/unifont.otf"))?,
                load(include_bytes!("../assets/fonts/unifont_upper.otf"))?,
            ],
            bold: load(include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf"))?,
        })
    }
    pub fn load(_packs: &Path) -> Result<Self> {
        Self::builtin()
    }
    fn font(&self, ch: char, size: f32, bold: bool) -> Font {
        let face = if bold && self.bold.unichar_to_glyph(ch as i32) != 0 {
            &self.bold
        } else {
            self.faces
                .iter()
                .find(|f| f.unichar_to_glyph(ch as i32) != 0)
                .unwrap_or(&self.faces[0])
        };
        Font::new(face.clone(), size)
    }
    pub fn supports(&self, s: &str) -> bool {
        s.chars()
            .all(|c| self.faces.iter().any(|f| f.unichar_to_glyph(c as i32) != 0))
    }
    pub fn width(&self, s: &str, size: f32, bold: bool) -> f32 {
        s.chars()
            .map(|ch| {
                self.font(ch, size, bold)
                    .measure_str(ch.to_string(), None)
                    .0
            })
            .sum()
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        c: &Canvas,
        s: &str,
        x: f64,
        y: f64,
        size: f32,
        color: Color,
        bold: bool,
        max: f32,
    ) {
        let mut text = s.to_owned();
        if self.width(&text, size, bold) > max {
            let ellipsis = self.width("…", size, bold);
            let mut used = ellipsis;
            let end = text
                .char_indices()
                .find_map(|(i, ch)| {
                    used += self
                        .font(ch, size, bold)
                        .measure_str(ch.to_string(), None)
                        .0;
                    (used > max).then_some(i)
                })
                .unwrap_or(text.len());
            text.truncate(end);
            if ellipsis <= max {
                text.push('…');
            } else {
                text.clear();
            }
        }
        let mut p = Paint::default();
        p.set_anti_alias(true).set_color(color);
        let mut x = x as f32;
        // Per-codepoint fallback is deterministic for CJK/Latin labels. Complex
        // script shaping is outside this release, explicitly not advertised.
        for ch in text.chars() {
            let f = self.font(ch, size, bold);
            let t = ch.to_string();
            c.draw_str(&t, (x, y as f32), &f, &p);
            x += f.measure_str(&t, None).0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_fallback_supports_chinese_and_japanese() -> Result<()> {
        let fonts = Fonts::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("output/packs"))?;
        assert!(fonts.supports("起死開戦 中文谱面预览 ガイド・トレース EXPERT 28"));
        assert!(fonts.width("起死開戦", 24., true) > 0.);
        assert!(fonts.supports("Symbol II : 🜁 / Symbol IV : 🜃"));
        assert!(fonts.width("🜁🜃", 24., true) > 0.);
        Ok(())
    }
}
