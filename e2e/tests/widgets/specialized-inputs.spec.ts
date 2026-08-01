import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

test.describe('text_area', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('text_area');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders a textarea with its label, placeholder and rows', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const widget = page.locator('[data-widget-type="text_area"]');
    await expect(widget.locator('.field-label')).toHaveText('Message');
    const area = widget.locator('textarea');
    await expect(area).toHaveAttribute('placeholder', 'Say something');
    await expect(area).toHaveAttribute('rows', '4');
  });

  test('typing round-trips through the server', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await page.locator('[data-widget-type="text_area"] textarea').fill('multi\nline');
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Value: multi'
    );
  });
});

test.describe('slider', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('slider');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders a range input with its bounds', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="slider"] input[type="range"]');
    await expect(input).toHaveAttribute('min', '0');
    await expect(input).toHaveAttribute('max', '100');
    await expect(input).toHaveAttribute('step', '5');
    await expect(input).toHaveValue('25');
  });

  test('dragging round-trips a numeric value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="slider"] input[type="range"]');

    // `fill` on a range input sets the value and fires `input`, which is what
    // the renderer listens for.
    await input.fill('60');
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText('Value: 60');
  });
});

test.describe('date_input', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('date_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders a date input with its min and max', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="date_input"] input[type="date"]');
    await expect(input).toHaveAttribute('min', '2026-01-01');
    await expect(input).toHaveAttribute('max', '2026-12-31');
  });

  test('picking a date round-trips an ISO-8601 string', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await page.locator('[data-widget-type="date_input"] input[type="date"]').fill('2026-08-01');
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Value: 2026-08-01'
    );
  });
});

test.describe('color_input', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('color_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders a colour input with its initial value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="color_input"] input[type="color"]');
    await expect(input).toHaveValue('#000000');
  });

  test('choosing a colour round-trips a hex string', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    // A native colour picker cannot be driven by clicks, so set the value and
    // dispatch the `input` event the renderer subscribes to.
    await page.locator('[data-widget-type="color_input"] input[type="color"]').evaluate((el) => {
      const input = el as HTMLInputElement;
      input.value = '#ff8800';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Value: #ff8800'
    );
  });
});

test.describe('radio_group', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('radio_group');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders one radio per option, defaulting to vertical', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const group = page.locator('[data-widget-type="radio_group"]');
    await expect(group).toHaveAttribute('data-orientation', 'vertical');
    await expect(group.locator('input[type="radio"]')).toHaveCount(3);
    await expect(group.locator('.field-label')).toHaveText('Size');
  });

  test('the options share a name so they are mutually exclusive', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const names = await page
      .locator('[data-widget-type="radio_group"] input[type="radio"]')
      .evaluateAll((nodes) => nodes.map((n) => (n as HTMLInputElement).name));
    expect(new Set(names).size).toBe(1);
  });

  test('selecting an option round-trips its value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const radios = page.locator('[data-widget-type="radio_group"] input[type="radio"]');
    await radios.nth(2).check();
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText('Value: l');

    await radios.nth(0).check();
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText('Value: s');
    // The re-rendered tree must reflect the selection the server holds.
    await expect(radios.nth(0)).toBeChecked();
    await expect(radios.nth(2)).not.toBeChecked();
  });
});

test.describe('multi_select', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('multi_select');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders a multiple select with one option per entry', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const select = page.locator('[data-widget-type="multi_select"] select');
    await expect(select).toHaveAttribute('multiple', '');
    await expect(select.locator('option')).toHaveCount(3);
  });

  test('selecting several options round-trips every value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await page
      .locator('[data-widget-type="multi_select"] select')
      .selectOption(['s', 'l']);

    // MultiSelect decodes a JSON array, so both values must arrive.
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Values: s,l'
    );
  });

  test('deselecting back to nothing round-trips an empty list', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const select = page.locator('[data-widget-type="multi_select"] select');
    await select.selectOption(['m']);
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText('Values: m');

    await select.selectOption([]);
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText('Values: ');
  });

  test('the re-rendered tree marks the server-held selection', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const select = page.locator('[data-widget-type="multi_select"] select');
    await select.selectOption(['s', 'm']);
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Values: s,m'
    );

    const selected = await select.evaluate((el) =>
      Array.from((el as HTMLSelectElement).selectedOptions).map((o) => o.value)
    );
    expect(selected).toEqual(['s', 'm']);
  });
});

test.describe('text_input read_only', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('text_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('is writable when read_only is not set', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    // The new `read_only` builder defaults to false, so the existing harness
    // input must stay editable.
    const input = page.locator('[data-widget-type="text_input"] input');
    await expect(input).not.toHaveAttribute('readonly', /.*/);
    await input.fill('edited');
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Value: edited'
    );
  });
});
