import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

// Events are queued but never dispatched back over the WebSocket, so these
// specs assert rendering only — no click round-trips.

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('data_table');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('data_table renders a table', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('[data-widget-type="data_table"] table')).toBeVisible();
});

test('data_table renders one header per column', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const headers = page.locator('[data-widget-type="data_table"] th');
  await expect(headers).toHaveCount(3);
  await expect(headers.nth(0)).toHaveText('Name');
  await expect(headers.nth(1)).toHaveText('Age');
  await expect(headers.nth(2)).toHaveText('Active');
});

test('data_table headers carry the column name', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const table = page.locator('[data-widget-type="data_table"]');
  await expect(table.locator('th[data-column-name="name"]')).toBeVisible();
  await expect(table.locator('th[data-column-name="age"]')).toBeVisible();
  await expect(table.locator('th[data-column-name="active"]')).toBeVisible();
});

test('data_table headers carry the column type', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const table = page.locator('[data-widget-type="data_table"]');
  await expect(table.locator('th[data-column-name="name"]')).toHaveAttribute(
    'data-column-type',
    'text'
  );
  await expect(table.locator('th[data-column-name="age"]')).toHaveAttribute(
    'data-column-type',
    'number'
  );
  await expect(table.locator('th[data-column-name="active"]')).toHaveAttribute(
    'data-column-type',
    'boolean'
  );
});

test('data_table renders one row per data entry', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const rows = page.locator('[data-widget-type="data_table"] tbody tr');
  await expect(rows).toHaveCount(2);
});

test('data_table cells render the row values', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const firstRow = page.locator('[data-widget-type="data_table"] tbody tr').first();
  const cells = firstRow.locator('td');
  await expect(cells.nth(0)).toHaveText('Alice');
  await expect(cells.nth(1)).toHaveText('30');
  await expect(cells.nth(2)).toHaveText('true');
});

test('data_table exposes its config on the container', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const table = page.locator('[data-widget-type="data_table"]');
  await expect(table).toHaveAttribute('data-selection-mode', 'cells');
  await expect(table).toHaveAttribute('data-show-search', 'true');
});
