import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('dialog');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('dialog renders with correct type', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const dialogs = page.locator('[data-widget-type="dialog"]');
  await expect(dialogs).toHaveCount(1);
});

test('dialog starts closed', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const dialog = page.locator('[data-widget-type="dialog"]');
  await expect(dialog).toHaveAttribute('data-open', 'false');
  await expect(dialog).toBeHidden();
  await expect(page.locator('text=Open: false')).toBeVisible();
});

test('dialog renders its title, body and footer in the tree', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  // The dialog is display:none while closed, so assert on the tree rather than
  // on visibility — this covers title/children/footer without needing a click.
  const dialog = page.locator('[data-widget-type="dialog"]');
  await expect(dialog.locator('.dialog-title')).toHaveText('Confirm action');
  await expect(dialog.locator('.dialog-footer button')).toHaveText('Close');
  await expect(dialog.locator('[data-widget-type="text_block"]')).toHaveText(
    'Are you sure about this?'
  );
});

test('dialog trigger button is present and clickable', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('button:has-text("Open dialog")')).toBeEnabled();
});

test('clicking the trigger opens the dialog', async ({ page }) => {
  await navigateToHarness(page, harness.port);

  await page.locator('button:has-text("Open dialog")').click();

  const dialog = page.locator('[data-widget-type="dialog"]');
  await expect(dialog).toHaveAttribute('data-open', 'true', { timeout: 5000 });
  await expect(dialog).toBeVisible();
  await expect(page.locator('.dialog-title:has-text("Confirm action")')).toBeVisible();
  await expect(page.locator('text=Are you sure about this?')).toBeVisible();
});

test('dialog footer holds the close button', async ({ page }) => {
  await navigateToHarness(page, harness.port);

  await page.locator('button:has-text("Open dialog")').click();
  const footer = page.locator('.dialog-footer');
  await expect(footer.locator('button:has-text("Close")')).toBeVisible({ timeout: 5000 });
});

test('close button closes the dialog', async ({ page }) => {
  await navigateToHarness(page, harness.port);

  await page.locator('button:has-text("Open dialog")').click();
  const dialog = page.locator('[data-widget-type="dialog"]');
  await expect(dialog).toHaveAttribute('data-open', 'true', { timeout: 5000 });

  await page.locator('.dialog-footer button:has-text("Close")').click();

  await expect(dialog).toHaveAttribute('data-open', 'false', { timeout: 5000 });
  await expect(page.locator('text=Open: false')).toBeVisible();
});
