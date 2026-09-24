use std::{env, path::PathBuf};
fn main() {
    let source = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("vendor/moenotes-chart-parser");
    for f in [
        "src/moenotes_chart_parser.c",
        "third_party/yyjson.c",
        "third_party/yyjson.h",
        "include/moenotes_chart_parser.h",
    ] {
        println!("cargo:rerun-if-changed={}", source.join(f).display());
    }
    println!("cargo:rerun-if-changed=src/ffi/abi_check.c");
    println!("cargo:rerun-if-changed=src/ffi/parser_build.c");
    let mut b = cc::Build::new();
    b.file("src/ffi/abi_check.c");
    b.file("src/ffi/parser_build.c")
        .file(source.join("third_party/yyjson.c"))
        .include(source.join("include"))
        .include(source.join("third_party"))
        .std("c17")
        .flag_if_supported("-ffp-contract=off")
        .flag_if_supported("/fp:strict");
    if let Some(include) = env::var_os("DEP_Z_INCLUDE") {
        b.include(include);
    }
    if env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "msvc" {
        b.define("_CRT_SECURE_NO_WARNINGS", None);
    }
    b.compile("moenotes_chart_parser");
    if env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default() == "unix" {
        println!("cargo:rustc-link-lib=m");
    }
}
