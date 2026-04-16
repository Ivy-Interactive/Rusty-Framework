import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('layout');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('renders vertical layout', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const vertical = page.locator('[data-widget-type="layout"][data-direction="vertical"]');
  await expect(vertical.first()).toBeVisible();
});

test('renders horizontal layout', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const horizontal = page.locator('[data-widget-type="layout"][data-direction="horizontal"]');
  await expect(horizontal).toBeVisible();
});

test('horizontal layout contains buttons', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const horizontal = page.locator('[data-widget-type="layout"][data-direction="horizontal"]');
  const buttons = horizontal.locator('button');
  await expect(buttons).toHaveCount(3); // Left, Center, Right
});

test('renders grid layout', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const grid = page.locator('[data-widget-type="layout"][data-direction="grid"]');
  await expect(grid).toBeVisible();
});

test('grid layout has 6 cells', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const grid = page.locator('[data-widget-type="layout"][data-direction="grid"]');
  const cells = grid.locator('[data-widget-type="text_block"]');
  await expect(cells).toHaveCount(6);
});
