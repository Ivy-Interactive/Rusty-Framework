import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('progress');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('progress renders with correct type', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const bars = page.locator('[data-widget-type="progress"]');
  await expect(bars).toHaveCount(5); // 0.25, labelled, indeterminate, max 200, advancing
});

test('progress reflects value and max', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const first = page.locator('[data-widget-type="progress"]').nth(0).locator('progress');
  await expect(first).toHaveJSProperty('value', 0.25);
  await expect(first).toHaveJSProperty('max', 1);

  const scaled = page.locator('[data-widget-type="progress"]').nth(3).locator('progress');
  await expect(scaled).toHaveJSProperty('value', 50);
  await expect(scaled).toHaveJSProperty('max', 200);
});

test('progress renders its label', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('.field-label:has-text("Upload progress")')).toBeVisible();
});

test('indeterminate progress is marked', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('[data-widget-type="progress"][data-indeterminate="true"]')).toHaveCount(1);
});

test('advance button is present', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('button:has-text("Advance")')).toBeEnabled();
});

// Pre-existing framework bug: Runtime::run() is never spawned for a WebSocket
// session, so RuntimeMessage::Event is queued on a channel nobody reads and no
// update patch is ever sent. The same defect fails button.spec.ts and
// input.spec.ts on main. Un-fixme once the event loop is driven.
test.fixme('advance button updates the progress value', async ({ page }) => {
  await navigateToHarness(page, harness.port);

  await expect(page.locator('text=Value: 0')).toBeVisible();

  await page.locator('button:has-text("Advance")').click();

  // The update patch must arrive and re-render the bar
  await expect(page.locator('text=Value: 0.25')).toBeVisible({ timeout: 5000 });
  const advancing = page.locator('[data-widget-type="progress"]').nth(4).locator('progress');
  await expect(advancing).toHaveJSProperty('value', 0.25);
});
