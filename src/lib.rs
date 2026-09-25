//! Offline chart preview. Use [`api`] for the supported embedding facade.
//! Other public modules are research internals, not covered by the facade's
//! compatibility contract.
#[cfg(not(any(target_pointer_width = "64", target_os = "emscripten")))]
compile_error!("Supported ABIs: native 64-bit and wasm32-unknown-emscripten");
#[cfg(not(target_os = "emscripten"))]
pub mod cli;
pub mod layout;
pub mod parser;
pub mod render;
pub mod scene;
pub mod skin;
pub use skin::*;
#[cfg(not(target_os = "emscripten"))]
pub mod resources;
pub mod typography;
extern crate libz_sys;
#[cfg(not(target_os = "emscripten"))]
pub mod metadata;

pub mod api;
#[cfg(not(target_os = "emscripten"))]
pub mod output;
