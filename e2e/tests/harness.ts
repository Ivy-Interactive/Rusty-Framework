import { type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'child_process';
import * as path from 'path';

const REPO_ROOT = path.resolve(__dirname, '../..');
const HARNESS_BIN = path.join(REPO_ROOT, 'target', 'debug', 'widget_harness');
const STATIC_DIR = path.join(REPO_ROOT, 'e2e', 'app');

export interface HarnessContext {
  port: number;
  process: ChildProcess;
}

/**
 * Start the widget_harness binary for a given widget.
 * Returns the port it's listening on.
 */
export async function startHarness(widget: string): Promise<HarnessContext> {
  return new Promise((resolve, reject) => {
    const proc = spawn(HARNESS_BIN, [widget, '--port', '0', '--static-dir', STATIC_DIR], {
      env: { ...process.env, RUST_LOG: 'info' },
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    const timeout = setTimeout(() => {
      proc.kill();
      reject(new Error(`Harness startup timed out for widget "${widget}". stdout: ${stdout}, stderr: ${stderr}`));
    }, 15_000);

    proc.stdout!.on('data', (data: Buffer) => {
      stdout += data.toString();
      // Look for RUSTY_PORT=<number> in output
      const match = stdout.match(/RUSTY_PORT=(\d+)/);
      if (match) {
        clearTimeout(timeout);
        resolve({ port: parseInt(match[1], 10), process: proc });
      }
    });

    proc.stderr!.on('data', (data: Buffer) => {
      stderr += data.toString();
      // Also check stderr since tracing may write there
      const match = stderr.match(/RUSTY_PORT=(\d+)/);
      if (match) {
        clearTimeout(timeout);
        resolve({ port: parseInt(match[1], 10), process: proc });
      }
    });

    proc.on('error', (err) => {
      clearTimeout(timeout);
      reject(new Error(`Failed to start harness: ${err.message}`));
    });

    proc.on('exit', (code) => {
      clearTimeout(timeout);
      if (code !== null && code !== 0) {
        reject(new Error(`Harness exited with code ${code}. stderr: ${stderr}`));
      }
    });
  });
}

/**
 * Navigate the page to the harness and wait for the widget tree to render.
 */
export async function navigateToHarness(page: Page, port: number): Promise<void> {
  await page.goto(`http://localhost:${port}/`);
  // Wait for the WebSocket to connect and the first widget to render
  await page.waitForSelector('[data-widget-type]', { timeout: 10_000 });
}

/**
 * Stop the harness process.
 */
export function stopHarness(ctx: HarnessContext): void {
  if (ctx.process && !ctx.process.killed) {
    ctx.process.kill('SIGTERM');
  }
}
