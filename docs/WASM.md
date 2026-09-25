# WASM SDK for other frontends

This package provides a renderer, typed JS bindings and optional resource helpers.
It contains no page, viewer component or frontend framework. The consuming frontend
owns its controls, zoom/pan gestures, canvas/image presentation and download UI.

## Build

Tested on Linux with Rust 1.98.1, Emscripten 6.0.10, Skia 0.153.3 and Node 22.
Install and activate the SDK, then:

```sh
rustup target add wasm32-unknown-emscripten
python3 scripts/build_wasm.py --emsdk /path/to/emsdk
node tests/wasm-smoke.mjs
```

`dist/wasm/` contains `moenotes-wasm.wasm`, Emscripten ES module glue,
`renderer.mjs`, `renderer.d.ts`, resource helpers, licenses and a hash manifest.
Keep the glue and WASM from the same build together. Serve `.wasm` as
`application/wasm`; configure `locateFile` when a bundler relocates it.
The build script consumes Cargo's JSON artifact paths, so `CARGO_TARGET_DIR` and
Cargo-configured target directories are respected. Missing artifacts fail; no
default-directory binary is substituted.
The local module is approximately 21.2 MiB uncompressed / 7.7 MiB gzip, mostly
embedded licensed fonts and Skia. It requires no downloaded fonts, game textures,
SharedArrayBuffer or cross-origin isolation.

The target is **wasm32-unknown-emscripten**, not wasm32-unknown-unknown/wasm-bindgen.
C and Rust assert the 32-bit parser ABI. Native 64-bit ABI checks remain active.
`scripts/wasm_linker.py` removes Rust's injected `-fwasm-exceptions` because the
pinned Skia cache uses Emscripten longjmp. Undefined-symbol errors remain enabled;
the wrapper does not hide unresolved functions. PNG/gzip error handling is covered
by runtime tests. Build tooling currently targets a Unix build host.

## Rendering and export

```js
import { createRenderer } from './moenotes/renderer.mjs';

const renderer = await createRenderer();
const { png, report, width, height, logicalWidth, logicalHeight } = renderer.render({
  chart: chartBytes,                  // Uint8Array: chart JSON or gzip
  cover: coverBytes,                  // optional Uint8Array: encoded image
  metadata: { title: 'Song', difficulty: 'EXPERT', level: '28', author: 'Writer' },
  options: { theme: 'white', flick_layout: 'callout', output_scale: 1 },
  mirror: false,
});
const pngBlob = new Blob([png], { type: 'image/png' });
// The host can display/download this blob, or transfer png.buffer to its UI thread.
renderer.dispose();
```

```js
// Inside the consuming project's own worker module:
const ready = createRenderer();
self.onmessage = async ({ data }) => {
  try {
    const result = (await ready).render(data);
    self.postMessage({ result }, [result.png.buffer]);
  } catch (error) {
    self.postMessage({ error: String(error) });
  }
};
```

Instantiate one renderer in a dedicated **module Web Worker**. `render()` is
synchronous CPU work; it returns owned bytes and makes no network requests.
Register the worker message handler before awaiting initialization, then await
the initialization promise inside the handler so early messages are not lost.
Successful output always contains one complete PNG. Errors throw; ordinary input
errors do not poison the instance. `dispose()` releases cached renderer resources;
terminate the worker to return its linear memory to the browser. Cancel a running
render by terminating its worker. There is no in-render cancellation callback.

The WASM facade currently exposes the original built-in artwork. Loading arbitrary
external skin packs from byte maps is not yet exposed. Native external-skin support
remains available. Caller-supplied metadata must already be resolved; the Rust
native metadata adapter is not part of the WASM module.

## Zoom versus PNG resolution

`fitViewport()` and `zoomAt()` are pure math helpers returning `{scale, x, y}`.
They touch no DOM and attach no event listeners. Their coordinates must use the
same image coordinate space selected by the host (logical or output pixels).
Display zoom changes that transform; it does not change exported bytes.
White/black exports reserve responsive header and footer space. Their image height
can change when the title or author credits wrap. `report.chart_offset_y` describes
the logical vertical shift from Scene coordinates; `flick_callouts[].y` already
includes this shift. The note timeline and column cuts remain unchanged.

For a higher-resolution export, render again with `options.output_scale: 2`.
That changes PNG pixels without reflowing columns, moving notes or altering ticks.
`supersample` controls internal antialiasing separately. Limits are 64M final
pixels, 256M internal pixels and 32768 px per final dimension. Oversized requests
fail explicitly. Large sheets can consume hundreds of MiB; the module allows
memory growth up to 2 GiB, but browsers/devices may impose a lower limit.
Limits apply to the scaled PNG and supersampled surface, including side rails and
annotation appendices. A logical sheet over 64M pixels can therefore be exported
at 0.5x when the resulting pixel allocations fit those budgets.

## Online resources

```js
import { chartObjectKey, fetchResource } from './moenotes/resources.mjs';
const chart = await fetchResource(chartObjectKey('0069/0069_03'), {
  base: configuredObjectBase,
  signal: abortController.signal,
});
// chart.bytes goes to renderer.render(); chart.provenance records URL/key/hash.
```

The supplied service's bucket browser is an HTML directory. Its byte endpoint is:

```
https://hsjkdajbsadnmsadds.zeabur.app/Default/api/buckets/moenotes/objects/
```

Example keys:

- `Live/MusicScore/0069/0069_03/0069_03.json`
- `Image/Jacket/jkt_004_100069/jkt_004_100069__00000.png`

`__00000.png` is an observed cover filename, not a universal listing contract.
Use an explicit object key or an upstream resource manifest when filenames vary.
The adapter accepts a configured base/fetch function and optional SHA-256, caps
download bytes, rejects directory HTML, omits credentials and supports aborts.
SHA-256 records the downloaded bytes; it proves an expected version only when
compared against a separately trusted digest. Metadata freshness is independent.

Observed on 2026-09-25: both example objects return 200, but their responses do
**not** contain `Access-Control-Allow-Origin`. A cross-origin frontend therefore
needs the resource owner's CORS configuration or its own controlled same-origin
proxy. `no-cors` cannot provide readable bytes or exportable canvas content.
This project neither changes that server nor includes a hosted proxy.

Native/offline callers can download the same bytes separately:

```sh
python3 scripts/online_assets.py --chart-key 0069/0069_03 \
  --cover-key jkt_004_100069 --output data/downloaded
```

## Validation scope

Node tests exercise both presets, gzip, input rejection/recovery, buffer copying,
disposal, PNG headers, export scaling, resource hashing and zoom anchor math.
Headless Chromium tests run the SDK inside a dedicated module Worker, transfer
PNG buffers, and decode complete images at 0.5x/1x/2x. The online example was
downloaded using the native adapter and passed into the browser test: 741 visible
notes, FC 874 in each case. This is not a claim that cross-origin fetch succeeded.
WASM CI uses synthetic resources only. Safari, Firefox and mobile memory limits
have not yet been validated. Native and WASM PNGs are not promised pixel-identical.
An additional dense chart passed Worker rendering at 1x, 0.5x and an explicitly
selected 1.25x (850 visible notes, FC 1100, no remaining arrow/body intersections).
Its 2x export correctly exceeded the 64M final-pixel budget and failed explicitly.

Optional browser check:

```sh
pip install playwright==1.61.0
python3 -m playwright install chromium
python3 scripts/check_wasm_browser.py
```
