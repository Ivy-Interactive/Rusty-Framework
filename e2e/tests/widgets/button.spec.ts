import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('button');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('button renders with correct type', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const buttons = page.locator('[data-widget-type="button"]');
  await expect(buttons).toHaveCount(3); // Click me, Secondary, Disabled
});

test('button displays correct labels', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('button:has-text("Click me")')).toBeVisible();
  await expect(page.locator('button:has-text("Secondary")')).toBeVisible();
  await expect(page.locator('button:has-text("Disabled")')).toBeVisible();
});

test('button has variant attribute', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('button[data-variant="primary"]')).toBeVisible();
  await expect(page.locator('button[data-variant="secondary"]')).toBeVisible();
});

test('disabled button is not clickable', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const disabled = page.locator('button:has-text("Disabled")');
  await expect(disabled).toBeDisabled();
});

test('click button increments counter', async ({ page }) => {
  await navigateToHarness(page, harness.port);

  // Initial count
  await expect(page.locator('text=Count: 0')).toBeVisible();

  // Click the primary button
  await page.locator('button:has-text("Click me")').click();

  // Wait for the count to update
  await expect(page.locator('text=Count: 1')).toBeVisible({ timeout: 5000 });
});
