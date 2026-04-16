import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

test.describe('TextInput', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('text_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders with label', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('[data-widget-type="text_input"]')).toBeVisible();
    await expect(page.locator('.field-label:has-text("Name")')).toBeVisible();
  });

  test('displays initial value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="text_input"] input');
    await expect(input).toHaveValue('hello');
  });

  test('typing updates displayed value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="text_input"] input');
    await input.fill('world');
    await expect(page.locator('text=Value: world')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('NumberInput', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('number_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders with label', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('[data-widget-type="number_input"]')).toBeVisible();
    await expect(page.locator('.field-label:has-text("Amount")')).toBeVisible();
  });

  test('displays initial value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="number_input"] input');
    await expect(input).toHaveValue('42');
  });
});

test.describe('Select', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('select');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders with options', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('[data-widget-type="select"]')).toBeVisible();
    const options = page.locator('[data-widget-type="select"] select option:not([disabled])');
    await expect(options).toHaveCount(3); // Apple, Banana, Cherry
  });

  test('displays initial selection', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const select = page.locator('[data-widget-type="select"] select');
    await expect(select).toHaveValue('apple');
  });
});

test.describe('Checkbox', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('checkbox');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders with label', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('[data-widget-type="checkbox"]')).toBeVisible();
    await expect(page.locator('text=Accept terms')).toBeVisible();
  });

  test('initially unchecked', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const checkbox = page.locator('[data-widget-type="checkbox"] input[type="checkbox"]');
    await expect(checkbox).not.toBeChecked();
  });

  test('clicking toggles checked state', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const checkbox = page.locator('[data-widget-type="checkbox"] input[type="checkbox"]');
    await checkbox.check();
    await expect(page.locator('text=Checked: true')).toBeVisible({ timeout: 5000 });
  });
});
