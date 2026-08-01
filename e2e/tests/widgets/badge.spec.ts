import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('badge');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('badge renders with correct type', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const badges = page.locator('[data-widget-type="badge"]');
  await expect(badges).toHaveCount(6); // 3 variants + 3 colors
});

test('badge displays correct labels', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('.badge:has-text("Default")')).toBeVisible();
  await expect(page.locator('.badge:has-text("Outline")')).toBeVisible();
  await expect(page.locator('.badge:has-text("Dot")')).toBeVisible();
});

test('badge has variant attribute', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('.badge[data-variant="default"]')).toBeVisible();
  await expect(page.locator('.badge[data-variant="outline"]')).toBeVisible();
  await expect(page.locator('.badge[data-variant="dot"]')).toBeVisible();
});

test('badge has color attribute', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('.badge[data-color="success"]')).toHaveText('Success');
  await expect(page.locator('.badge[data-color="warning"]')).toHaveText('Warning');
  await expect(page.locator('.badge[data-color="danger"]')).toHaveText('Danger');
});
