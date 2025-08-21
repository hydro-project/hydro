#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# ts-prune
npx -y ts-prune -p tsconfig.json > dev_reports/ts-prune.txt

echo "ts-prune: wrote dev_reports/ts-prune.txt"

# madge
npx -y madge --ts-config tsconfig.json --json . > dev_reports/madge.json

echo "madge: wrote dev_reports/madge.json"

# size-top40
node - <<'NODE'
const fs=require('fs');
const path=require('path');
const walk=(dir)=>fs.readdirSync(dir,{withFileTypes:true}).flatMap(d=>{
  const p=path.join(dir,d.name);
  if(d.isDirectory()) return walk(p);
  if(/[.](ts|tsx|js)$/.test(d.name)) return [p];
  return [];
});
const files=walk('.');
const sizes=files.map(f=>({f,lines:fs.readFileSync(f,'utf8').split('\n').length}));
sizes.sort((a,b)=>b.lines-a.lines);
const out=sizes.slice(0,40).map(s=>`${s.lines}\t${s.f}`).join('\n');
fs.writeFileSync('dev_reports/size-top40.txt', out+'\n');
console.log('size-top40: wrote dev_reports/size-top40.txt');
NODE

# knip (run from repo root to resolve workspace)
(
  cd ../../..
  # Write report to docs path explicitly
  npx -y knip --reporter json --reporter-options '{"path":"docs/src/components/visualizer-v4/dev_reports/knip.json"}' || true
)

echo "knip: wrote dev_reports/knip.json (exit non-zero allowed for findings)"

echo "All dev reports refreshed."
