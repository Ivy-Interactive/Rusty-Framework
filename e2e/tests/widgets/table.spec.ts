import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('table');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('table renders with correct type', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const tables = page.locator('[data-widget-type="table"]');
  await expect(tables).toHaveCount(2); // unsorted + sorted by name
});

test('table renders its column headers', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const headers = page.locator('[data-widget-type="table"]').nth(0).locator('th');
  await expect(headers).toHaveCount(3);
  await expect(headers.nth(0)).toHaveText('Name');
  await expect(headers.nth(1)).toHaveText('Role');
  await expect(headers.nth(2)).toHaveText('Age');
});

test('table marks sortable columns', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const firstTable = page.locator('[data-widget-type="table"]').nth(0);
  await expect(firstTable.locator('th[data-sortable="true"]')).toHaveCount(2); // name, age
  await expect(firstTable.locator('th[data-key="role"]')).not.toHaveAttribute('data-sortable', 'true');
});

test('table renders its rows', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const rows = page.locator('[data-widget-type="table"]').nth(0).locator('tbody tr');
  await expect(rows).toHaveCount(3);
  await expect(rows.nth(0).locator('td').nth(0)).toHaveText('Ada');
  await expect(rows.nth(0).locator('td').nth(1)).toHaveText('Engineer');
  await expect(rows.nth(0).locator('td').nth(2)).toHaveText('36');
});

test('sorted table carries its sort state', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const sorted = page.locator('[data-widget-type="table"]').nth(1);
  await expect(sorted).toHaveAttribute('data-sort-by', 'name');
  await expect(sorted).toHaveAttribute('data-sort-ascending', 'true');
});
