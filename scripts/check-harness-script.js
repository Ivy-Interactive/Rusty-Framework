// scripts/check-harness-script.js — parse-check the inline <script> in
// e2e/app/index.html, the Playwright harness renderer. A SyntaxError there
// renders no widgets at all, so every spec dies in waitForSelector with nothing
// pointing at the cause; it has survived two hand-resolved merge conflicts that
// way. `npx playwright test --list` exits 0 on a broken file, because the
// Playwright loader never looks at the HTML.
//
// Lives in a script rather than inline in ci.yml so that
// `node scripts/ci-step.js e2e 'Check harness renderer parses'` prints a
// command that actually runs, and so agents can run the exact CI check locally.
// Dependency-free; writes its temp file to the OS temp dir, never the worktree.
// Usage: node scripts/check-harness-script.js
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const file = path.join(__dirname, '..', 'e2e', 'app', 'index.html');
const html = fs.readFileSync(file, 'utf8');

const open = (html.match(/<script\b/g) || []).length;
const close = (html.match(/<\/script>/g) || []).length;
if (open !== 1 || close !== 1) {
  console.error(
    `${file}: expected exactly one inline <script>, found ${open} open / ${close} close tags.`,
  );
  console.error('If the harness genuinely needs a second script, teach this check about it.');
  process.exit(1);
}

const m = html.match(/<script>([\s\S]*?)<\/script>/);
if (!m) {
  console.error(`${file}: the <script> tag carries attributes; this check expects a bare <script>.`);
  process.exit(1);
}

// Line of the file the <script> body starts on, so node's line numbers can be
// translated back to the file an agent has to edit.
const offset = html.slice(0, m.index + '<script>'.length).split('\n').length - 1;

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'harness-script-'));
const js = path.join(dir, 'harness-script.js');
fs.writeFileSync(js, m[1]);
const { status, stderr } = spawnSync(process.execPath, ['--check', js], { encoding: 'utf8' });
fs.rmSync(dir, { recursive: true, force: true });

if (status !== 0) {
  console.error(`${file}: the inline <script> does not parse.`);
  console.error(`Line numbers below are within the <script> body; add ${offset} for the file.`);
  console.error('');
  console.error(stderr.trimEnd().split('\n').slice(0, 8).join('\n'));
  process.exit(1);
}

console.log(`${file}: inline <script> parses (${m[1].split('\n').length} lines).`);
