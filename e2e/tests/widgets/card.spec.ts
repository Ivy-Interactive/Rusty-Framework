import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('card');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('card renders with title', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('.card-title:has-text("My Card")')).toBeVisible();
});

test('card renders with subtitle', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('.card-subtitle:has-text("A subtitle")')).toBeVisible();
});

test('card contains body content', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('text=Card body content')).toBeVisible();
});

test('two cards are rendered', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const cards = page.locator('[data-widget-type="card"]');
  await expect(cards).toHaveCount(2);
});

test('second card has a button', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const secondCard = page.locator('[data-widget-type="card"]').nth(1);
  await expect(secondCard.locator('button:has-text("Card Action")')).toBeVisible();
});
