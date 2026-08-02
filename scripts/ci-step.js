// scripts/ci-step.js — print the `run:` of one named step in one named job of
// .github/workflows/ci.yml. Dependency-free: `yaml` is only a transitive dep of
// src/frontend and does not resolve from a plain `require`.
// Usage: node scripts/ci-step.js <job> <step name>
const fs = require('fs');
const path = require('path');
const [job, step] = process.argv.slice(2);
const file = path.join(__dirname, '..', '.github', 'workflows', 'ci.yml');
const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
const ind = (l) => l.length - l.trimStart().length;
let inJobs = false, jobInd = null, inJob = false, stepInd = null;
const blocks = [];
for (const l of lines) {
  if (!l.trim() || /^\s*#/.test(l)) continue;
  if (/^jobs:\s*$/.test(l)) { inJobs = true; continue; }
  if (inJobs && ind(l) === 0) { inJobs = false; inJob = false; }
  if (!inJobs) continue;
  if (jobInd === null) jobInd = ind(l);
  if (ind(l) === jobInd && /^\s*[\w.-]+:\s*$/.test(l)) {
    inJob = l.trim().slice(0, -1) === job;
    continue;
  }
  if (!inJob) continue;
  const m = l.match(/^(\s*)-\s+(.*)$/);
  if (m) { stepInd = m[1].length; blocks.push([]); }
  if (stepInd === null || ind(l) < stepInd || !blocks.length) continue;
  blocks[blocks.length - 1].push(l);
}

const parse = (body) => {
  const rl = body.map((l) => l.replace(/^(\s*)-\s+/, '$1  '));
  const nm = rl.join('\n').match(/^\s*name:\s*(.+?)\s*$/m);
  const name = nm && nm[1].replace(/^["']|["']$/g, '').replace(/\s*#.*$/, '').trim();
  let run = null;
  for (let i = 0; i < rl.length; i++) {
    const r = rl[i].match(/^(\s*)run:\s*(.*)$/);
    if (!r) continue;
    if (/^[|>][-+]?\d*$/.test(r[2])) {
      const base = r[1].length, out = [];
      for (let j = i + 1; j < rl.length && (!rl[j].trim() || ind(rl[j]) > base); j++) out.push(rl[j].trim());
      run = out.filter(Boolean).join('\n');
    } else run = r[2].trim();
    break;
  }
  return { name, run };
};
const hits = blocks.map(parse).filter((s) => s.name === step && s.run);
if (hits.length !== 1) {
  console.error(`ci-step: ${hits.length} steps named "${step}" with a run: in job "${job}" — ci.yml was restructured; read it and fix the caller`);
  process.exit(3);
}
process.stdout.write(hits[0].run + '\n');
