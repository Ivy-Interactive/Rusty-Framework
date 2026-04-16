import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('text');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('text block renders headings', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('h1:has-text("Heading 1")')).toBeVisible();
  await expect(page.locator('h2:has-text("Heading 2")')).toBeVisible();
  await expect(page.locator('h3:has-text("Heading 3")')).toBeVisible();
});

test('text block renders paragraph', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('p:has-text("This is a paragraph.")')).toBeVisible();
});

test('text block renders code', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('code:has-text("let x = 42;")')).toBeVisible();
});

test('text block has correct variant attributes', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const variants = page.locator('[data-widget-type="text_block"]');
  await expect(variants).toHaveCount(6); // h1, h2, h3, paragraph, code, label
});
