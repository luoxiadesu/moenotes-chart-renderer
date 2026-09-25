#!/usr/bin/env python3
"""Build an ES module + WASM SDK, without an application or frontend UI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def build_artifacts(cargo, env):
    # Cargo reports the actual paths after applying env/config target directories.
    # Never fall back to a possibly stale artifact in ROOT/target.
    result = subprocess.run(
        [cargo, "build", "--release", "--locked", "--features", "wasm", "--bin", "moenotes-wasm",
         "--target", "wasm32-unknown-emscripten", "--message-format=json-render-diagnostics"],
        cwd=ROOT, env=env, check=True, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    artifacts = [message for line in result.stdout.splitlines() if line.strip()
                 for message in [json.loads(line)]
                 if message.get("reason") == "compiler-artifact"
                 and message.get("target", {}).get("name") == "moenotes-wasm"
                 and "bin" in message.get("target", {}).get("kind", [])]
    if len(artifacts) != 1 or not artifacts[0].get("executable"):
        raise ValueError("Cargo did not report exactly one moenotes-wasm executable")
    glue = Path(artifacts[0]["executable"])
    if not glue.is_absolute():
        glue = ROOT / glue
    # Rust preserves '-' in the executable name but emits the companion Wasm
    # using the crate identifier (underscores). It is beside this exact artifact.
    wasm = glue.with_name("moenotes_wasm.wasm")
    if glue.suffix != ".js" or not glue.is_file() or not wasm.is_file():
        raise FileNotFoundError(f"Missing Cargo-reported WASM artifacts: {glue}, {wasm}")
    return glue, wasm


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--emsdk", type=Path, default=os.environ.get("EMSDK"))
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--output", type=Path, default=ROOT / "dist/wasm")
    args = parser.parse_args()
    if not args.emsdk:
        parser.error("Set EMSDK or pass --emsdk (tested with 6.0.10)")
    sdk = args.emsdk.resolve()
    args.output = args.output.resolve()
    env = os.environ.copy()
    env["EMSDK"] = str(sdk)
    env["PATH"] = str(sdk / "upstream/emscripten") + os.pathsep + env["PATH"]
    env["CC_wasm32_unknown_emscripten"] = str(sdk / "upstream/emscripten/emcc")
    env["CXX_wasm32_unknown_emscripten"] = str(sdk / "upstream/emscripten/em++")
    env["AR_wasm32_unknown_emscripten"] = str(sdk / "upstream/emscripten/emar")
    env["CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER"] = str(ROOT / "scripts/wasm_linker.py")
    exports = ["mn_alloc", "mn_free", "mn_render", "mn_png_ptr", "mn_png_len", "mn_report_ptr", "mn_report_len", "mn_error_ptr", "mn_error_len", "mn_clear", "mn_dispose"]
    flags = ["-C", "panic=abort"]
    for value in ["-sMODULARIZE=1", "-sEXPORT_ES6=1", "-sEXPORT_NAME=createMoeNotesModule",
                  "-sENVIRONMENT=web,worker,node", "-sALLOW_MEMORY_GROWTH=1", "-sMAXIMUM_MEMORY=2147483648",
                  "-sSTACK_SIZE=8388608", "-sINITIAL_MEMORY=67108864", "-sFILESYSTEM=0", "-sNO_EXIT_RUNTIME=1",
                  "-sSUPPORT_LONGJMP=emscripten", "-sDISABLE_EXCEPTION_CATCHING=1",
                  "-sEXPORTED_RUNTIME_METHODS=['HEAPU8']", "-sEXPORTED_FUNCTIONS=" + json.dumps(["_" + x for x in exports], separators=(",", ":")),
                  "--no-entry"]:
        flags.extend(["-C", "link-arg=" + value])
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(flags)
    glue_path, wasm_path = build_artifacts(args.cargo, env)
    args.output.mkdir(parents=True, exist_ok=True)
    glue = glue_path.read_text(encoding="utf-8")
    (args.output / "moenotes-wasm.mjs").write_text(glue.replace("moenotes_wasm.wasm", "moenotes-wasm.wasm"), encoding="utf-8")
    shutil.copy2(wasm_path, args.output / "moenotes-wasm.wasm")
    for file in (ROOT / "wasm").iterdir():
        if file.suffix in (".mjs", ".ts", ".json"):
            shutil.copy2(file, args.output / file.name)
    shutil.copy2(ROOT / "LICENSE", args.output / "LICENSE")
    shutil.copy2(ROOT / "THIRD_PARTY.md", args.output / "THIRD_PARTY.md")
    shutil.copy2(ROOT / "docs/WASM.md", args.output / "README.md")
    shutil.copytree(ROOT / "assets/licenses", args.output / "licenses", dirs_exist_ok=True)
    shutil.copy2(sdk / "upstream/emscripten/LICENSE", args.output / "licenses/Emscripten-LICENSE")
    for name in ["LICENSE", "third_party/LICENSE.yyjson"]:
        source = ROOT / "vendor/moenotes-chart-parser" / name
        if source.is_file():
            shutil.copy2(source, args.output / "licenses" / ("parser-" + source.name))
    shutil.copytree(ROOT / "assets/fonts", args.output / "font-notices", ignore=shutil.ignore_patterns("*.ttf", "*.otf"), dirs_exist_ok=True)
    files = {str(p.relative_to(args.output)): hashlib.sha256(p.read_bytes()).hexdigest() for p in args.output.rglob("*") if p.is_file() and p.name != "manifest.json"}
    toolchain = subprocess.check_output([str(sdk / "upstream/emscripten/emcc"), "--version"], env=env, text=True).splitlines()[0]
    (args.output / "manifest.json").write_text(json.dumps({"target": "wasm32-unknown-emscripten", "toolchain": toolchain, "sha256": files}, indent=2) + "\n", encoding="utf-8")
    print(f"WASM SDK: {args.output.resolve()}")


if __name__ == "__main__":
    main()
