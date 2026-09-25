import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { gzipSync } from 'node:zlib';
import { createRenderer, fitViewport, zoomAt } from '../dist/wasm/renderer.mjs';
import { chartObjectKey, objectURL, fetchResource } from '../dist/wasm/resources.mjs';

const chart = new Uint8Array(await fs.readFile(new URL('fixtures/synthetic.json', import.meta.url)));
const cover = new Uint8Array(await fs.readFile(new URL('fixtures/cover.png', import.meta.url)));
const wasmBinary = new Uint8Array(await fs.readFile(new URL('../dist/wasm/moenotes-wasm.wasm', import.meta.url)));
const renderer = await createRenderer({ wasmBinary });
const results = [];
for (const theme of ['white', 'black']) {
  const result = renderer.render({ chart, cover, metadata: { title: 'MoeNotes 合成谱面 🜁', author: 'Synthetic' }, options: { theme, supersample: 1 } });
  assert.deepEqual([...result.png.slice(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  assert.equal(result.report.images.length, 1);
  assert.equal(result.width, result.logicalWidth);
  results.push({ theme, width: result.width, height: result.height, pngBytes: result.png.length, fc: result.report.statistics.reconstructed_full_combo });
}
assert.equal(results[0].fc, results[1].fc);
const compressed = renderer.render({ chart: new Uint8Array(gzipSync(chart)), metadata: { title: 'Gzip' }, options: { output_scale: 0.5, supersample: 1 } });
assert.equal(compressed.width, Math.ceil(compressed.logicalWidth * 0.5));
assert.throws(() => renderer.render({ chart: new Uint8Array([1, 2]), metadata: { title: 'Bad' } }), /parse/i);
assert.throws(() => renderer.render({ chart: new Uint8Array([31,139,8,0]), metadata: { title: 'Bad gzip' } }), /parse/i);
assert.throws(() => renderer.render({ chart, cover: new Uint8Array([137,80,78,71]), metadata: { title: 'Bad cover' } }), /decode/i);
assert.throws(() => renderer.render({ chart, metadata: { title: 'Bad scale' }, options: { output_scale: 100 } }), /output-scale/i);
const dense = new TextEncoder().encode(JSON.stringify({events:{},notes:[
  {type:'flick',dir:'right',t:480,pos:4,size:8},
  {type:'flick',dir:'left',t:510,pos:4,size:8},
  {type:'tap',t:540,pos:4,size:8}
]}));
const denseResult = renderer.render({chart:dense,metadata:{title:'Dense'},options:{supersample:1}});
assert.equal(denseResult.report.arrow_body_box_overlaps,0);
assert.equal(denseResult.report.flick_callouts.length,2);
const creditedDense = renderer.render({chart:dense,metadata:{title:'Dense',author:'作詞作曲編曲 '.repeat(30)},options:{supersample:1}});
const headerShift = creditedDense.report.chart_offset_y - denseResult.report.chart_offset_y;
assert(headerShift > 0);
assert(Math.abs(creditedDense.report.flick_callouts[0].y - denseResult.report.flick_callouts[0].y - headerShift) < 0.01);
const largeChart = new TextEncoder().encode(JSON.stringify({events:{},notes:[{t:690720,pos:4,size:6}]}));
const largeOptions = {pixels_per_beat:160,auto_spacing:false,supersample:1,output_scale:0.25};
const largeResult = renderer.render({chart:largeChart,metadata:{title:'Downscaled large sheet'},options:largeOptions});
assert(largeResult.logicalWidth * largeResult.logicalHeight > 64_000_000);
assert.equal(largeResult.width,Math.ceil(largeResult.logicalWidth*0.25));
assert.throws(() => renderer.render({chart:largeChart,metadata:{title:'Too large'},options:{...largeOptions,output_scale:1}}), /Scaled PNG exceeds/);
const recovered = renderer.render({ chart, metadata: { title: 'Recovery' }, options: { supersample: 1 } });
assert.equal(recovered.report.glyphs, compressed.report.glyphs);
const fit = fitViewport(1000, 500, 500, 500);
const zoom = zoomAt(fit, 2, 250, 250);
assert.equal((250 - fit.x) / fit.scale, (250 - zoom.x) / zoom.scale);
assert.equal((250 - fit.y) / fit.scale, (250 - zoom.y) / zoom.scale);
assert.equal(chartObjectKey('0069/0069_03'), 'Live/MusicScore/0069/0069_03/0069_03.json');
assert.throws(() => objectURL('Live/MusicScore/../a.json'));
const resource = await fetchResource(chartObjectKey('test/test_03'), { fetch: async () => new Response(chart) });
assert.deepEqual(resource.bytes, chart);
await assert.rejects(fetchResource(chartObjectKey('test/test_03'), { sha256: '0'.repeat(64), fetch: async () => new Response(chart) }), /SHA-256/);
renderer.dispose();
assert.throws(() => renderer.render({ chart, metadata: { title: 'Disposed' } }), /disposed/);
console.log(JSON.stringify({ wasm: 'passed', themes: results, gzip: true, malformedImages: true, denseFlick: true, headerCoordinates: true, errorRecovery: true, exportScale: true, viewport: true, resources: true }));
