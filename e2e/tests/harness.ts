import { type Page } from '@playwright/test';
import { spawn, type ChildProcess } from 'child_process';
import * as path from 'path';

const REPO_ROOT = path.resolve(__dirname, '../..');
const HARNESS_BIN = path.join(REPO_ROOT, 'target', 'debug', 'widget_harness');
const ROUTING_BIN = path.join(REPO_ROOT, 'target', 'debug', 'routing_harness');
const STATIC_DIR = path.join(REPO_ROOT, 'e2e', 'app');

export interface HarnessContext {
  port: number;
  process: ChildProcess;
}

/**
 * Internal helper: spawn a harness binary and wait for its RUSTY_PORT= line.
 */
function spawnHarness(bin: string, args: string[], label: string): Promise<HarnessContext> {
  return new Promise((resolve, reject) => {
    const proc = spawn(bin, args, {
      env: { ...process.env, RUST_LOG: 'info' },
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    const timeout = setTimeout(() => {
      proc.kill();
      reject(new Error(`${label} startup timed out. stdout: ${stdout}, stderr: ${stderr}`));
    }, 15_000);

    const scan = (data: Buffer) => {
      const text = data.toString();
      const match = text.match(/RUSTY_PORT=(\d+)/);
      if (match) {
        clearTimeout(timeout);
        resolve({ port: parseInt(match[1], 10), process: proc });
      }
    };

    proc.stdout!.on('data', (data: Buffer) => {
      stdout += data.toString();
      scan(data);
    });

    proc.stderr!.on('data', (data: Buffer) => {
      stderr += data.toString();
      scan(data);
    });

    proc.on('error', (err) => {
      clearTimeout(timeout);
      reject(new Error(`Failed to start ${label}: ${err.message}`));
    });

    proc.on('exit', (code) => {
      clearTimeout(timeout);
      if (code !== null && code !== 0) {
        reject(new Error(`${label} exited with code ${code}. stderr: ${stderr}`));
      }
    });
  });
}

/**
 * Start the widget_harness binary for a given widget.
 * Returns the port it's listening on.
 */
export async function startHarness(widget: string): Promise<HarnessContext> {
  return spawnHarness(HARNESS_BIN, [widget, '--port', '0', '--static-dir', STATIC_DIR], `widget_harness ${widget}`);
}

/**
 * Start the routing_harness binary (multi-app server).
 * Returns the port it's listening on.
 */
export async function startRoutingHarness(): Promise<HarnessContext> {
  return spawnHarness(ROUTING_BIN, ['--port', '0', '--static-dir', STATIC_DIR], 'routing_harness');
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
 * Navigate the page to a specific app and wait for the widget tree to render.
 */
export async function navigateToApp(page: Page, port: number, appId: string): Promise<void> {
  const encodedAppId = encodeURIComponent(appId);
  await page.goto(`http://localhost:${port}/?appId=${encodedAppId}`);
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
