//! Offline chart preview. Use [`api`] for the supported embedding facade.
//! Other public modules are research internals, not covered by the facade's
//! compatibility contract.
#[cfg(not(target_pointer_width = "64"))]
compile_error!("The checked-in parser ABI currently supports only 64-bit targets");
pub mod cli;
pub mod layout;
pub mod parser;
pub mod render;
pub mod scene;
pub mod skin;
pub use skin::*;
pub mod resources;
pub mod typography;
extern crate libz_sys;
pub mod metadata;

pub mod api;
pub mod output;
