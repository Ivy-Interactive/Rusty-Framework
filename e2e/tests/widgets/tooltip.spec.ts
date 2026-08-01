import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('tooltip');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('tooltip renders with correct type', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const tooltips = page.locator('[data-widget-type="tooltip"]');
  await expect(tooltips).toHaveCount(2); // wrapping a button and a text block
});

test('tooltip carries its content as a title attribute', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const tooltips = page.locator('[data-widget-type="tooltip"]');
  await expect(tooltips.nth(0)).toHaveAttribute('title', 'Buttons do things');
  await expect(tooltips.nth(1)).toHaveAttribute('title', 'Text can be explained too');
});

test('tooltip renders its wrapped child', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const tooltips = page.locator('[data-widget-type="tooltip"]');
  await expect(tooltips.nth(0).locator('button:has-text("Hover me")')).toBeVisible();
  await expect(tooltips.nth(1).locator('[data-widget-type="text_block"]')).toHaveText('Hover this text');
});

test('hovering the tooltip child keeps it visible', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const trigger = page.locator('[data-widget-type="tooltip"]').nth(0).locator('button');
  await trigger.hover();
  await expect(trigger).toBeVisible();
  await expect(page.locator('[data-tooltip="Buttons do things"]')).toBeVisible();
});
