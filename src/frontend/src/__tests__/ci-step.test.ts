import { describe, test, expect } from 'vitest';
import { execFileSync } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const ciStepScript = join(__dirname, '../../../../scripts/ci-step.js');

function runCiStep(job: string, step: string, fixtureYaml?: string): { stdout: string; exitCode: number } {
  let scriptPath = ciStepScript;

  if (fixtureYaml) {
    // Create temp dir with fixture workflow
    const tempDir = join(tmpdir(), `ci-step-test-${Date.now()}-${Math.random().toString(36).slice(2)}`);
    mkdirSync(join(tempDir, '.github', 'workflows'), { recursive: true });
    mkdirSync(join(tempDir, 'scripts'), { recursive: true });

    writeFileSync(join(tempDir, '.github', 'workflows', 'ci.yml'), fixtureYaml);
    writeFileSync(join(tempDir, 'scripts', 'ci-step.js'),
      require('fs').readFileSync(ciStepScript, 'utf8'));

    scriptPath = join(tempDir, 'scripts', 'ci-step.js');
  }

  try {
    const stdout = execFileSync('node', [scriptPath, job, step], {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe']
    });
    return { stdout: stdout.trim(), exitCode: 0 };
  } catch (err: any) {
    return { stdout: err.stderr?.trim() || '', exitCode: err.status || 1 };
  }
}

describe('ci-step.js', () => {
  test('extracts all seven gate commands from real ci.yml', () => {
    // Test against the actual .github/workflows/ci.yml
    expect(runCiStep('build', 'Build').stdout).toBe('cargo build --workspace');
    expect(runCiStep('build', 'Test').stdout).toBe('cargo test --workspace');
    expect(runCiStep('build', 'Clippy').stdout).toBe('cargo clippy --workspace --all-targets -- -D warnings');
    expect(runCiStep('build', 'Format check').stdout).toBe('cargo fmt --all -- --check');
    expect(runCiStep('frontend', 'Lint').stdout).toBe('pnpm lint');
    expect(runCiStep('frontend', 'Typecheck').stdout).toBe('pnpm exec tsc -b');
    expect(runCiStep('frontend', 'Test').stdout).toBe('pnpm test');
  });

  test('exits 3 for nonexistent step', () => {
    const result = runCiStep('build', 'NonExistent');
    expect(result.exitCode).toBe(3);
    expect(result.stdout).toContain('0 steps named "NonExistent"');
  });

  test('exits 3 for nonexistent job', () => {
    const result = runCiStep('nonexistent', 'Test');
    expect(result.exitCode).toBe(3);
    expect(result.stdout).toContain('0 steps named "Test"');
  });

  test('exits 3 for ambiguous step name', () => {
    const fixture = `
jobs:
  job1:
    steps:
      - name: Test
        run: echo one
  job2:
    steps:
      - name: Test
        run: echo two
`;
    // This should fail because we're only looking in one job, so it won't be ambiguous
    // Let me create a fixture where the same job has duplicate step names
    const ambiguousFixture = `
jobs:
  build:
    steps:
      - name: Test
        run: echo first
      - uses: some-action
      - name: Test
        run: echo second
`;
    const result = runCiStep('build', 'Test', ambiguousFixture);
    expect(result.exitCode).toBe(3);
    expect(result.stdout).toContain('2 steps named "Test"');
  });

  test('handles multi-line block scalar correctly', () => {
    const fixture = `
jobs:
  test:
    steps:
      - name: Multi
        run: |
          line one
          line two
          line three
`;
    const result = runCiStep('test', 'Multi', fixture);
    expect(result.stdout).toBe('line one\nline two\nline three');
    expect(result.exitCode).toBe(0);
  });

  test('never returns literal pipe character for block scalar', () => {
    const fixture = `
jobs:
  test:
    steps:
      - name: Block
        run: |
          echo "test"
`;
    const result = runCiStep('test', 'Block', fixture);
    expect(result.stdout).not.toBe('|');
    expect(result.stdout).toBe('echo "test"');
  });

  test('handles inline comments in step names', () => {
    const fixture = `
jobs:
  test:
    steps:
      - name: Lint  # this is a comment
        run: pnpm lint
`;
    const result = runCiStep('test', 'Lint', fixture);
    expect(result.stdout).toBe('pnpm lint');
    expect(result.exitCode).toBe(0);
  });
});
