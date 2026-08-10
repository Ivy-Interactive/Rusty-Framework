import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

test.describe('blade', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('blade');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders a container of indexed blades', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const container = page.locator('[data-widget-type="blade_container"]');
    await expect(container).toHaveCount(1);

    const blades = page.locator('[data-widget-type="blade"]');
    await expect(blades).toHaveCount(2);
    await expect(blades.nth(0)).toHaveAttribute('data-blade-index', '0');
    await expect(blades.nth(1)).toHaveAttribute('data-blade-index', '1');
    await expect(blades.nth(0).locator('[data-blade-title]')).toHaveText('Root');
    await expect(blades.nth(1).locator('[data-blade-title]')).toHaveText('Details');

    // Width travels as a CSS length string.
    const box = await blades.nth(0).boundingBox();
    expect(box?.width).toBeCloseTo(240, 0);
  });

  test('hides Close on the root blade only', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const blades = page.locator('[data-widget-type="blade"]');
    await expect(blades.nth(0).locator('[data-blade-close]')).toHaveCount(0);
    await expect(blades.nth(0).locator('[data-blade-refresh]')).toHaveCount(1);
    await expect(blades.nth(1).locator('[data-blade-close]')).toHaveCount(1);
  });

  test('close and refresh reach the Rust handlers', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const root = page.locator('[data-widget-type="layout"]').first();
    await expect(root).toContainText('Closes: 0 Refreshes: 0');

    await page.locator('[data-blade-refresh]').first().click();
    await expect(root).toContainText('Closes: 0 Refreshes: 1');

    await page.locator('[data-blade-close]').first().click();
    await expect(root).toContainText('Closes: 1 Refreshes: 1');
  });
});

test.describe('breadcrumbs', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('breadcrumbs');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders the trail with separators between crumbs', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const crumbs = page.locator('[data-breadcrumb-label]');
    await expect(crumbs).toHaveCount(4);
    await expect(crumbs.nth(0)).toHaveAttribute('data-breadcrumb-label', 'Home');
    await expect(crumbs.nth(3)).toHaveAttribute('data-breadcrumb-label', 'Widgets');

    // Three separators for four crumbs, never a trailing one.
    const separators = page.locator('[data-breadcrumb-separator]');
    await expect(separators).toHaveCount(3);
    await expect(separators.first()).toHaveText('>');
  });

  test('renders only clickable non-final crumbs as buttons', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const crumbs = page.locator('[data-breadcrumb-label]');
    await expect(crumbs.nth(0).locator('button')).toHaveCount(1);
    await expect(crumbs.nth(1).locator('button')).toHaveCount(1);
    // "Rusty" opted out of clickability.
    await expect(crumbs.nth(2).locator('button')).toHaveCount(0);
    // The last crumb is the current location.
    await expect(crumbs.nth(3).locator('button')).toHaveCount(0);
  });

  test('a crumb click reports its index to Rust', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const root = page.locator('[data-widget-type="layout"]').first();
    await expect(root).toContainText('Clicked: -1');

    await page.locator('[data-breadcrumb-label] button').nth(1).click();
    await expect(root).toContainText('Clicked: 1');

    await page.locator('[data-breadcrumb-label] button').nth(0).click();
    await expect(root).toContainText('Clicked: 0');
  });
});

test.describe('pagination', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('pagination');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders one button per page with the current one active', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('[data-page]')).toHaveCount(10);
    await expect(page.locator('[data-page="1"]')).toHaveAttribute('data-active', 'true');
    await expect(page.locator('[data-page="2"]')).not.toHaveAttribute('data-active', /.*/);
    // Page 1 has nowhere to go back to.
    await expect(page.locator('[data-page-prev]')).toBeDisabled();
    await expect(page.locator('[data-page-next]')).toBeEnabled();
  });

  test('selecting a page round-trips through Rust state', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const root = page.locator('[data-widget-type="layout"]').first();
    await expect(root).toContainText('Page: 1');

    await page.locator('[data-page="4"]').click();
    await expect(root).toContainText('Page: 4');
    await expect(page.locator('[data-page="4"]')).toHaveAttribute('data-active', 'true');
    await expect(page.locator('[data-page-prev]')).toBeEnabled();

    await page.locator('[data-page-next]').click();
    await expect(root).toContainText('Page: 5');

    await page.locator('[data-page="10"]').click();
    await expect(root).toContainText('Page: 10');
    await expect(page.locator('[data-page-next]')).toBeDisabled();
  });
});

test.describe('toolbar', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('toolbar');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders buttons, a separator and a nested group', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const toolbar = page.locator('[data-widget-type="toolbar"]');
    await expect(toolbar).toHaveAttribute('role', 'toolbar');
    await expect(toolbar.locator('[role="separator"]')).toHaveCount(1);

    const group = toolbar.locator('[role="group"]');
    await expect(group).toHaveAttribute('data-toolbar-group', 'Format');
    await expect(group.locator('[data-toolbar-tag]')).toHaveCount(2);

    // Items at any depth carry their tag; groups and separators carry none.
    await expect(toolbar.locator('[data-toolbar-tag]')).toHaveCount(4);
    await expect(toolbar.locator('[data-toolbar-tag="bold"]')).toHaveAttribute(
      'data-checked',
      'true'
    );
    await expect(toolbar.locator('[data-toolbar-tag="delete"]')).toBeDisabled();
    await expect(toolbar.locator('[data-toolbar-tag="save"]')).toHaveAttribute(
      'data-icon-name',
      'save'
    );
  });

  test('selecting an item sends its tag to Rust', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const root = page.locator('[data-widget-type="layout"]').first();
    await expect(root).toContainText('Selected:');

    await page.locator('[data-toolbar-tag="save"]').click();
    await expect(root).toContainText('Selected: save');

    // A nested group member reports through the same widget-level handler.
    await page.locator('[data-toolbar-tag="italic"]').click();
    await expect(root).toContainText('Selected: italic');
  });
});
