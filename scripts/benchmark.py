#!/usr/bin/env python3
"""Linux measurement of wall time and OS-reported peak RSS, high quality defaults."""
import argparse,json,subprocess,tempfile,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,default=ROOT/'target/release/moenotes-chart-renderer');p.add_argument('--chart',type=Path,default=ROOT/'tests/fixtures/synthetic.json');p.add_argument('--runs',type=int,default=3);p.add_argument('--output',type=Path,default=ROOT/'output/engineering/benchmark.json');a=p.parse_args();rows=[]
 with tempfile.TemporaryDirectory() as temp:
  t=Path(temp)
  for i in range(a.runs):
   stats=t/'time.txt';start=time.perf_counter();r=subprocess.run(['/usr/bin/time','-f','%e %M','-o',str(stats),str(a.binary.resolve()),'render',str(a.chart.resolve()),'-o',str(t/'chart.png')],capture_output=True,text=True);assert r.returncode==0,r.stderr
   report=json.loads((t/'chart.render.json').read_text());assert len(report['images'])==1; elapsed,rss=stats.read_text().strip().split();rows.append({'wall_seconds':time.perf_counter()-start,'process_wall_seconds':float(elapsed),'peak_rss_kib':int(rss),'images':len(report['images']),'source_notes':report['source_notes'],'glyphs':report['glyphs'],'curve_points':report['curve_points']})
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'configuration':'builtin / default quality / CPU / Linux time peak RSS','input':a.chart.name,'runs':rows},indent=2)+'\n');print(rows)
if __name__=='__main__':main()
