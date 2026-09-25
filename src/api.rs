//! Supported embedding interface. Rendering returns owned PNG bytes and a report;
//! A chart produces exactly one complete multi-column PNG.
//! It performs no output writes and makes no network requests.
pub use crate::layout::{CurveMode, FlickLayout, Options as RenderOptions, Theme};
pub use crate::render::{Metadata, Page, Rendered, Report};
use crate::{layout::Layout, parser::Score, render, scene::Scene, skin::Skin, typography::Fonts};
use std::{fmt, path::Path};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    Input,
    Resources,
    Layout,
    Render,
}
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for Error {}
fn err(kind: ErrorKind, e: anyhow::Error) -> Error {
    Error {
        kind,
        message: format!("{e:#}"),
    }
}
pub struct Renderer {
    skin: Skin,
    fonts: Fonts,
}
/// Serializable byte-in request shared by native and future browser workers.
/// Resource fetching and viewport zoom remain the host's responsibility.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderRequest {
    pub chart: Vec<u8>,
    #[serde(default)]
    pub options: RenderOptions,
    pub metadata: Metadata,
    #[serde(default)]
    pub mirror: bool,
    #[serde(default)]
    pub cover: Option<Vec<u8>>,
}
impl Renderer {
    pub fn render_request(&self, request: &RenderRequest) -> Result<Rendered, Error> {
        self.render(
            &request.chart,
            request.options.clone(),
            request.metadata.clone(),
            request.mirror,
            "chart.png",
            request.cover.as_deref(),
        )
    }
    pub fn builtin() -> Result<Self, Error> {
        Ok(Self {
            skin: Skin::builtin().map_err(|e| err(ErrorKind::Resources, e))?,
            fonts: Fonts::builtin().map_err(|e| err(ErrorKind::Resources, e))?,
        })
    }
    pub fn from_skin_pack(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            skin: Skin::load(path).map_err(|e| err(ErrorKind::Resources, e))?,
            fonts: Fonts::builtin().map_err(|e| err(ErrorKind::Resources, e))?,
        })
    }
    pub fn render(
        &self,
        chart: &[u8],
        options: RenderOptions,
        metadata: Metadata,
        mirror: bool,
        basename: &str,
        cover: Option<&[u8]>,
    ) -> Result<Rendered, Error> {
        let score = Score::parse(chart, mirror).map_err(|e| err(ErrorKind::Input, e))?;
        let layout = Layout::build(&score, options).map_err(|e| err(ErrorKind::Layout, e))?;
        let scene = Scene::build(&score, layout).map_err(|e| err(ErrorKind::Layout, e))?;
        render::render_memory(
            &scene,
            &self.skin,
            &self.fonts,
            &metadata,
            mirror,
            basename,
            cover,
        )
        .map_err(|e| err(ErrorKind::Render, e))
    }
}
