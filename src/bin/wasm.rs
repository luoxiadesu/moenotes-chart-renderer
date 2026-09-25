//! Synchronous C ABI wrapped by wasm/renderer.mjs. One instance per Web Worker.
#[cfg(target_os = "emscripten")]
mod exports {
    use moenotes_chart_renderer::api::{Metadata, RenderOptions, Renderer};
    use serde::Deserialize;
    use std::cell::RefCell;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Config {
        #[serde(default)]
        options: RenderOptions,
        metadata: Metadata,
        #[serde(default)]
        mirror: bool,
    }
    #[derive(Default)]
    struct State {
        renderer: Option<Renderer>,
        png: Vec<u8>,
        report: Vec<u8>,
        error: Vec<u8>,
    }
    thread_local! {static STATE: RefCell<State> = RefCell::new(State::default());}

    #[unsafe(no_mangle)]
    pub extern "C" fn mn_alloc(size: usize) -> *mut u8 {
        if size == 0 || size > 64 * 1024 * 1024 {
            return std::ptr::null_mut();
        }
        let layout = std::alloc::Layout::array::<u8>(size).unwrap();
        unsafe { std::alloc::alloc(layout) }
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn mn_free(ptr: *mut u8, size: usize) {
        if !ptr.is_null() && size > 0 {
            unsafe {
                std::alloc::dealloc(ptr, std::alloc::Layout::array::<u8>(size).unwrap());
            }
        }
    }
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn mn_render(
        chart: *const u8,
        chart_len: usize,
        config: *const u8,
        config_len: usize,
        cover: *const u8,
        cover_len: usize,
    ) -> i32 {
        STATE.with(|state| {
            let mut s = state.borrow_mut();
            s.png = vec![];
            s.report = vec![];
            s.error = vec![];
            let result = (|| -> anyhow::Result<()> {
                anyhow::ensure!(
                    !chart.is_null() && chart_len > 0 && chart_len <= 64 * 1024 * 1024,
                    "Invalid chart buffer"
                );
                anyhow::ensure!(
                    !config.is_null() && config_len > 0 && config_len <= 1024 * 1024,
                    "Invalid configuration buffer"
                );
                anyhow::ensure!(
                    cover_len <= 16 * 1024 * 1024 && (cover_len == 0 || !cover.is_null()),
                    "Invalid cover buffer"
                );
                let cfg: Config = serde_json::from_slice(unsafe {
                    std::slice::from_raw_parts(config, config_len)
                })?;
                if s.renderer.is_none() {
                    s.renderer = Some(Renderer::builtin()?);
                }
                let result = s.renderer.as_ref().unwrap().render(
                    unsafe { std::slice::from_raw_parts(chart, chart_len) },
                    cfg.options,
                    cfg.metadata,
                    cfg.mirror,
                    "chart.png",
                    if cover_len > 0 {
                        Some(unsafe { std::slice::from_raw_parts(cover, cover_len) })
                    } else {
                        None
                    },
                )?;
                s.report = serde_json::to_vec(&result.report)?;
                s.png = result.pages.into_iter().next().unwrap().png;
                Ok(())
            })();
            match result {
                Ok(()) => 0,
                Err(error) => {
                    s.error = format!("{error:#}").into_bytes();
                    1
                }
            }
        })
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_png_ptr() -> *const u8 {
        STATE.with(|s| s.borrow().png.as_ptr())
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_png_len() -> usize {
        STATE.with(|s| s.borrow().png.len())
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_report_ptr() -> *const u8 {
        STATE.with(|s| s.borrow().report.as_ptr())
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_report_len() -> usize {
        STATE.with(|s| s.borrow().report.len())
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_error_ptr() -> *const u8 {
        STATE.with(|s| s.borrow().error.as_ptr())
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_error_len() -> usize {
        STATE.with(|s| s.borrow().error.len())
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_clear() {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.png = vec![];
            s.report = vec![];
            s.error = vec![];
        });
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn mn_dispose() {
        STATE.with(|s| *s.borrow_mut() = State::default());
    }
}
fn main() {}
