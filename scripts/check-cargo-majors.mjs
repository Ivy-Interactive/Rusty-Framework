#!/usr/bin/env node
// Reports dependencies whose latest stable release is outside the major series
// declared in the workspace manifests. Renovate parks cargo majors, so without
// this nothing surfaces them (renovate.json packageRules).
import { execFileSync } from 'node:child_process';

const FAIL_ON_DRIFT = process.env.CARGO_MAJOR_FAIL === '1';

function seriesOf(version) {
  const m = version.match(/^\D*(\d+)(?:\.(\d+))?/);
  if (!m) return null;
  const [, major, minor] = m;
  return major === '0' ? `0.${minor ?? '0'}` : major;
}

let meta;
try {
  meta = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'],
    { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
} catch (err) {
  console.error(`cargo metadata failed: ${err.message}`);
  process.exit(2);
}

const members = new Set(meta.packages.map(p => p.name));
const deps = new Map();
for (const pkg of meta.packages) {
  for (const dep of pkg.dependencies) {
    if (members.has(dep.name)) continue;
    if (!deps.has(dep.name)) deps.set(dep.name, { req: dep.req, from: new Set() });
    deps.get(dep.name).from.add(pkg.name);
  }
}

const drifted = [];
let resolved = 0;
for (const [name, info] of [...deps].sort()) {
  let latest;
  try {
    const res = await fetch(`https://crates.io/api/v1/crates/${name}`,
      { headers: { 'User-Agent': 'Ivy-Interactive/Rusty-Framework cargo-major-check' } });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    latest = (await res.json()).crate.max_stable_version;
    resolved++;
  } catch (err) {
    console.log(`::warning::could not resolve ${name} on crates.io (${err.message}); skipping`);
    continue;
  }
  const declared = seriesOf(info.req);
  const available = seriesOf(latest);
  if (declared && available && declared !== available) {
    drifted.push({ name, req: info.req, latest, declared, available, from: [...info.from].sort() });
  }
}

if (resolved === 0 && deps.size > 0) {
  console.error('Resolved 0 of ' + deps.size + ' dependencies against crates.io - treating as failure rather than reporting no drift.');
  process.exit(2);
}

if (!drifted.length) {
  console.log(`No cargo major drift across ${deps.size} external dependencies.`);
  process.exit(0);
}

console.log(`Cargo major updates available (${drifted.length} of ${deps.size} external deps):`);
for (const d of drifted) {
  console.log(`  ${d.name}: ${d.req} -> ${d.latest}  (series ${d.declared} -> ${d.available})  used by ${d.from.join(', ')}`);
}
console.log('');
console.log('Renovate parks cargo majors, so these will not arrive as PRs. Review each by hand.');
process.exit(FAIL_ON_DRIFT ? 1 : 0);
