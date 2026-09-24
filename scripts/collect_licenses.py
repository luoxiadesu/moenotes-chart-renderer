#!/usr/bin/env python3
"""Collect Cargo dependency license declarations and available notice files."""
import argparse,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser();p.add_argument('--cargo',default='cargo');a=p.parse_args()
 cmd=[a.cargo,'metadata','--format-version','1','--locked']
 data=json.loads(subprocess.check_output(cmd,cwd=ROOT));out=ROOT/'assets/licenses/cargo';out.mkdir(parents=True,exist_ok=True);rows=[]
 for pkg in data['packages']:
  if pkg['name']=='moenotes-chart-renderer':continue
  root=Path(pkg['manifest_path']).parent;name=pkg['name']+'-'+pkg['version'];notices=[]
  for f in sorted(root.iterdir()):
   if f.is_file() and f.name.upper().startswith(('LICENSE','COPYING','NOTICE')):
    target=out/(name+'-'+f.name);target.write_bytes(f.read_bytes());notices.append(target.name)
  rows.append({'name':pkg['name'],'version':pkg['version'],'license':pkg.get('license'),'repository':pkg.get('repository'),'notice_files':notices})
 (out/'index.json').write_text(json.dumps(rows,indent=2)+'\n', encoding="utf-8");print('Collected declarations for',len(rows),'packages')
if __name__=='__main__':main()
