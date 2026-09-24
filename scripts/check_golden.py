#!/usr/bin/env python3
"""Compare a synthetic, redistributable sheet to reviewed visual baselines."""
import argparse,json,subprocess,tempfile
from pathlib import Path
from PIL import Image,ImageChops,ImageStat
ROOT=Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--update',action='store_true');a=p.parse_args();gold=ROOT/'tests/golden/synthetic.png'
 with tempfile.TemporaryDirectory() as temp:
  output=Path(temp)/'synthetic.png';r=subprocess.run([str(a.binary.resolve()),'render',str(ROOT/'tests/fixtures/synthetic.json'),'-o',str(output),'--metadata',str(ROOT/'tests/fixtures/metadata.json'),'--supersample','1','--pixels-per-beat','24','--fixed-spacing','--bars-per-column','2'],capture_output=True,text=True);assert r.returncode==0,r.stderr
  image=Image.open(output).convert('RGB')
  if a.update:gold.parent.mkdir(exist_ok=True);image.save(gold);print('Golden updated explicitly');return
  reference=Image.open(gold).convert('RGB');assert image.size==reference.size
  diff=ImageChops.difference(image,reference);mean=sum(ImageStat.Stat(diff).mean)/3
  # Different FreeType rasterizers may vary glyph edge pixels; allow a small
  # global error, while catching moved notes, geometry and layout regressions.
  assert mean<=1.0,f'Visual baseline error {mean:.4f} > 1.0'
  print(json.dumps({'mean_absolute_channel_error':mean,'dimensions':image.size,'threshold':1.0}))
if __name__=='__main__':main()
