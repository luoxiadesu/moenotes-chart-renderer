use moenotes_chart_renderer::api::{Metadata, RenderOptions, Renderer};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::builtin()?;
    let chart = br#"{"events":{},"notes":[{"t":480,"pos":4,"size":6}]}"#;
    let result = renderer.render(
        chart,
        RenderOptions::default(),
        Metadata {
            title: "Memory render".into(),
            ..Metadata::default()
        },
        false,
        "chart.png",
        None,
    )?;
    println!(
        "{} pages, {} PNG bytes",
        result.pages.len(),
        result.pages[0].png.len()
    );
    Ok(())
}
