import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

// A fresh harness per test: the query cache is server-scoped, so a warm entry
// from a previous navigation would skip the loading state entirely.
let harness: HarnessContext;

test.beforeEach(async () => {
  harness = await startHarness('query');
});

test.afterEach(() => {
  stopHarness(harness);
});

test('use_query renders the loading state, then the pushed value', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('p:has-text("Loading...")')).toBeVisible();
  await expect(page.locator('p:has-text("Hello from the query cache")')).toBeVisible({ timeout: 10_000 });
  await expect(page.locator('p:has-text("Loading...")')).toHaveCount(0);
});

test('the pushed value arrives without a page reload', async ({ page }) => {
  const reloads: string[] = [];
  page.on('framenavigated', (f) => { if (f === page.mainFrame()) reloads.push(f.url()); });
  await navigateToHarness(page, harness.port);
  await expect(page.locator('p:has-text("Hello from the query cache")')).toBeVisible({ timeout: 10_000 });
  expect(reloads).toHaveLength(1);
});
