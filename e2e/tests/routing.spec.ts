import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, navigateToApp, stopHarness, type HarnessContext } from './harness';

declare global {
  interface Window {
    rustyNavigate: (appId: string) => void;
    rustyRefreshCount: number;
    rustyUpdateCount: number;
  }
}

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('button');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('the client script parses and renders widgets', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));

  await navigateToHarness(page, harness.port);

  // The script should parse without errors
  expect(errors).toEqual([]);

  // There should be exactly one script tag
  const scriptCount = await page.locator('script').count();
  expect(scriptCount).toBe(1);

  // At least one widget should be rendered
  const widgetCount = await page.locator('[data-widget-type="button"]').count();
  expect(widgetCount).toBeGreaterThan(0);

  // No "Unknown widget" errors
  const unknownCount = await page.getByText('[Unknown widget:').count();
  expect(unknownCount).toBe(0);
});

test('refresh and update counters track server messages', async ({ page }) => {
  await navigateToHarness(page, harness.port);

  // After connection, we should have received one refresh
  let refreshCount = await page.evaluate(() => window.rustyRefreshCount);
  let updateCount = await page.evaluate(() => window.rustyUpdateCount);
  expect(refreshCount).toBe(1);
  expect(updateCount).toBe(0);

  // Click the button to trigger an update
  await page.getByText('Click me').click();

  // Wait for the update counter to increment
  await expect.poll(async () => {
    return await page.evaluate(() => window.rustyUpdateCount);
  }, { timeout: 5000 }).toBe(1);

  // Refresh count should remain unchanged
  refreshCount = await page.evaluate(() => window.rustyRefreshCount);
  expect(refreshCount).toBe(1);
});

test('appId from the page query is forwarded onto the socket URL', async ({ page }) => {
  const socketUrls: string[] = [];
  page.on('websocket', (ws) => {
    socketUrls.push(ws.url());
  });

  await navigateToApp(page, harness.port, 'beta');

  // Exactly one WebSocket connection should have been made
  expect(socketUrls.length).toBe(1);

  // The URL should contain the appId parameter
  expect(socketUrls[0]).toContain('/ws?appId=beta');
});

test('omitting appId leaves the socket URL unchanged', async ({ page }) => {
  const socketUrls: string[] = [];
  page.on('websocket', (ws) => {
    socketUrls.push(ws.url());
  });

  await navigateToHarness(page, harness.port);

  // Exactly one WebSocket connection should have been made
  expect(socketUrls.length).toBe(1);

  // The URL should end with /ws (no query string)
  expect(socketUrls[0]).toMatch(/\/ws$/);
});

test('rustyNavigate is exposed and sends without throwing', async ({ page }) => {
  await navigateToHarness(page, harness.port);

  // The function should be exposed on window
  const navigateType = await page.evaluate(() => typeof window.rustyNavigate);
  expect(navigateType).toBe('function');

  // Calling it should not throw
  await page.evaluate(() => window.rustyNavigate('beta'));

  // Wait briefly for any message processing
  await page.waitForTimeout(500);

  // The refresh count should still be 1, because the server's Navigate arm is a stub
  // and does not send a new refresh message. This asserts today's server behaviour.
  // When server routing is restored, this will need to change to expect 2 refreshes.
  const refreshCount = await page.evaluate(() => window.rustyRefreshCount);
  expect(refreshCount).toBe(1);
});
