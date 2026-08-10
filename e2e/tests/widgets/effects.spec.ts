import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('effects');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('confetti renders with its trigger and child', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const confetti = page.locator('[data-widget-type="confetti"]');
  await expect(confetti).toHaveCount(1);
  await expect(confetti).toHaveAttribute('data-trigger', 'click');
  await expect(confetti.locator('button:has-text("Celebrate")')).toBeVisible();
});

test('animation nodes render with their type, easing and visibility', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const animations = page.locator('[data-widget-type="animation"]');
  await expect(animations).toHaveCount(3);

  await expect(animations.nth(0)).toHaveAttribute('data-animation-type', 'bounce');
  await expect(animations.nth(0)).toHaveAttribute('data-easing', 'easeInOut');
  await expect(animations.nth(0)).toHaveAttribute('data-visible', 'true');

  await expect(animations.nth(1)).toHaveAttribute('data-animation-type', 'fadeIn');
  await expect(animations.nth(1)).toHaveAttribute('data-visible', 'false');

  await expect(animations.nth(2)).toHaveAttribute('data-animation-type', 'slideIn');
  await expect(animations.nth(2)).toHaveAttribute('data-direction', 'up');
});

test('stacked progress renders its segments and labels', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const bars = page.locator('[data-widget-type="stacked_progress"]');
  await expect(bars).toHaveCount(2);

  const segments = bars.nth(0).locator('.stacked-progress-segment');
  await expect(segments).toHaveCount(3);
  await expect(segments.nth(0)).toHaveAttribute('data-label', 'Done');
});

test('clicking a stacked progress segment dispatches select and updates state', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('text=Selected:')).toBeVisible();

  const selectable = page.locator('[data-widget-type="stacked_progress"]').nth(1);
  await selectable.locator('.stacked-progress-segment').nth(2).click();

  await expect(page.locator('text=Selected: 2')).toBeVisible({ timeout: 5000 });
});
