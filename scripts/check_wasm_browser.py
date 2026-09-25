#!/usr/bin/env python3
"""Headless SDK integration test; this is a test harness, not a frontend app.

Requires Python Playwright with Chromium installed. Tests a dedicated Worker,
transferred PNG bytes, complete image decode, themes, scaling and error recovery.
"""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import threading

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--chart", type=Path, default=ROOT / "tests/fixtures/synthetic.json")
    parser.add_argument("--cover", type=Path, default=ROOT / "tests/fixtures/cover.png")
    parser.add_argument("--chromium", type=Path, help="Optional existing Chromium executable")
    parser.add_argument("--large-scale", type=float, default=2, help="Explicit export scale for the third render")
    args = parser.parse_args()
    resources = {"/test-chart": args.chart.read_bytes(), "/test-cover": args.cover.read_bytes()}

    class Handler(SimpleHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            if self.path == "/test-host":
                data = b"<!doctype html><title>WASM SDK test</title>"
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(data)
            elif self.path in resources:
                self.send_response(200)
                self.send_header("Content-Type", "application/octet-stream")
                self.end_headers()
                self.wfile.write(resources[self.path])
            else:
                super().do_GET()

    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(Handler, directory=str(ROOT)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True, executable_path=str(args.chromium) if args.chromium else None)
            page = browser.new_page()
            page.goto(f"http://127.0.0.1:{server.server_port}/test-host")
            result = page.evaluate("""async (largeScale) => {
              const root = location.origin;
              const code = `import {createRenderer} from '${root}/dist/wasm/renderer.mjs';
                const ready = createRenderer();
                self.onmessage = async ({data}) => {
                  try {
                    const renderer = await ready;
                    const chart = new Uint8Array(await (await fetch('${root}/test-chart')).arrayBuffer());
                    const cover = new Uint8Array(await (await fetch('${root}/test-cover')).arrayBuffer());
                    let rejected = false;
                    try {renderer.render({chart:new Uint8Array([0]),metadata:{title:'bad'}});} catch {rejected=true;}
                    const result = renderer.render({chart,cover,metadata:{title:'MoeNotes 合成谱面 🜁'}, options:{theme:data.theme,output_scale:data.scale,supersample:1}});
                    self.postMessage({result,rejected},[result.png.buffer]);
                  } catch(e) {self.postMessage({error:e.message});}
                };`;
              const url = URL.createObjectURL(new Blob([code],{type:'text/javascript'}));
              const worker = new Worker(url,{type:'module'});
              const results=[];
              try {
                for (const [theme,scale] of [['white',1],['black',0.5],['white',largeScale]]) {
                  const reply = await new Promise((resolve,reject)=>{
                    const timer=setTimeout(()=>reject(new Error('WASM Worker timeout')),120000);
                    worker.onmessage=({data})=>{clearTimeout(timer); data.error ? reject(new Error(data.error)) : resolve(data);};
                    worker.onerror=e=>{clearTimeout(timer);reject(new Error(e.message));};
                    worker.postMessage({theme,scale});
                  });
                  const {result,rejected}=reply;
                  const bitmap=await createImageBitmap(new Blob([result.png],{type:'image/png'}));
                  if (!rejected || bitmap.width!==result.width || bitmap.height!==result.height || result.report.images.length!==1 || result.width!==Math.ceil(result.logicalWidth*scale)) throw new Error('PNG/worker contract mismatch');
                  results.push({theme,scale,width:bitmap.width,height:bitmap.height,glyphs:result.report.glyphs,fc:result.report.statistics.reconstructed_full_combo,pngBytes:result.png.length,arrowOverlaps:result.report.arrow_body_box_overlaps});
                  bitmap.close();
                }
              } finally {worker.terminate();URL.revokeObjectURL(url);}
              return {browserWorker:'passed',exportDecode:'passed',errorRecovery:'passed',results};
            }""", args.large_scale)
            browser.close()
            print(json.dumps(result, ensure_ascii=False))
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
