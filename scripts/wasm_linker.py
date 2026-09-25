#!/usr/bin/env python3
"""Match Rust's final link to the pinned Skia cache's Emscripten SjLj ABI.

Rust 1.98 injects -fwasm-exceptions even with panic=abort. Skia 0.153.3's
published wasm archive uses emscripten_longjmp instead. This executable wrapper
removes that injected flag; it never suppresses undefined-symbol errors.
"""
import os
from pathlib import Path
import subprocess
import sys

linker = Path(os.environ["EMSDK"]) / "upstream/emscripten/em++"
raise SystemExit(subprocess.call([str(linker), *[a for a in sys.argv[1:] if a != "-fwasm-exceptions"]]))
