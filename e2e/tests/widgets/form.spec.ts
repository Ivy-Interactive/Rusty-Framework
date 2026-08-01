import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

// Events are queued but never dispatched back over the WebSocket, so these
// specs assert rendering only — no submit or change round-trips.

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('form');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('form renders', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('[data-widget-type="form"]')).toBeVisible();
});

test('form renders one field per registration', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const fields = page.locator('[data-widget-type="field"]');
  await expect(fields).toHaveCount(2);
});

test('fields render their labels in registration order', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const labels = page.locator('[data-widget-type="field"] .field-label');
  await expect(labels.nth(0)).toHaveText('Name');
  await expect(labels.nth(1)).toHaveText('Email');
});

test('required field is marked required', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const fields = page.locator('[data-widget-type="field"]');
  await expect(fields.nth(0)).toHaveAttribute('data-required', 'true');
  await expect(fields.nth(1)).not.toHaveAttribute('data-required', 'true');
});

test('field description is rendered', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const description = page
    .locator('[data-widget-type="field"]')
    .nth(1)
    .locator('[data-field-description]');
  await expect(description).toHaveText('We never share it');
});

test('each field wraps a text input', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const inputs = page.locator('[data-widget-type="field"] [data-widget-type="text_input"] input');
  await expect(inputs).toHaveCount(2);
  await expect(inputs.nth(0)).toHaveAttribute('placeholder', 'Your name');
  await expect(inputs.nth(1)).toHaveAttribute('placeholder', 'you@example.com');
});

test('form renders a submit button with the configured title', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const submit = page.locator('[data-widget-type="form"] [data-form-submit]');
  await expect(submit).toBeVisible();
  await expect(submit).toHaveText('Sign up');
});

test('no field starts out invalid', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('[data-field-invalid]')).toHaveCount(0);
});
