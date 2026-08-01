import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

test.describe('icon', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('icon');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders each icon by name', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const icons = page.locator('[data-widget-type="icon"]');
    await expect(icons).toHaveCount(3);
    await expect(icons.nth(0)).toHaveAttribute('data-icon-name', 'check');
    await expect(icons.nth(1)).toHaveAttribute('data-icon-name', 'alert');
    await expect(icons.nth(2)).toHaveAttribute('data-icon-name', 'info');
  });

  test('applies size and colour when given', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const icons = page.locator('[data-widget-type="icon"]');
    await expect(icons.nth(1)).toHaveAttribute('data-size', '32');
    await expect(icons.nth(2)).toHaveAttribute('data-color', '#0066cc');
    // Unset optional fields must not surface as attributes.
    await expect(icons.nth(0)).not.toHaveAttribute('data-size', /.*/);
    await expect(icons.nth(0)).not.toHaveAttribute('data-color', /.*/);
  });
});

test.describe('image', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('image');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders an img with src and alt', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const img = page.locator('[data-widget-type="image"] img').first();
    await expect(img).toHaveAttribute('src', /^data:image\/gif;base64,/);
    await expect(img).toHaveAttribute('alt', 'A transparent pixel');
  });

  test('applies width and height', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const sized = page.locator('[data-widget-type="image"] img').nth(1);
    const box = await sized.boundingBox();
    expect(box?.width).toBeCloseTo(64, 0);
    expect(box?.height).toBeCloseTo(64, 0);
  });
});

test.describe('avatar', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('avatar');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders the fallback text when no image is given', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const avatars = page.locator('[data-widget-type="avatar"]');
    await expect(avatars).toHaveCount(3);
    await expect(avatars.first().locator('[data-fallback]')).toHaveText('AB');
  });

  test('exposes the density as a size', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const avatars = page.locator('[data-widget-type="avatar"]');
    // Density serializes camelCase and defaults to `normal`.
    await expect(avatars.nth(0)).toHaveAttribute('data-size', 'normal');
    await expect(avatars.nth(1)).toHaveAttribute('data-size', 'compact');
    await expect(avatars.nth(2)).toHaveAttribute('data-size', 'comfortable');
  });
});

test.describe('callout', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('callout');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders one callout per variant', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const callouts = page.locator('[data-widget-type="callout"]');
    await expect(callouts).toHaveCount(4);
    for (const [index, variant] of ['info', 'success', 'warning', 'error'].entries()) {
      await expect(callouts.nth(index)).toHaveAttribute('data-variant', variant);
    }
  });

  test('renders the title and children', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const info = page.locator('[data-widget-type="callout"]').first();
    await expect(info.locator('.callout-title')).toHaveText('Heads up');
    await expect(info).toContainText('An informational note.');
  });
});

test.describe('skeleton', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('skeleton');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('applies pixel and percentage sizes', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const skeletons = page.locator('[data-widget-type="skeleton"]');
    await expect(skeletons).toHaveCount(2);

    const fixed = await skeletons.first().boundingBox();
    expect(fixed?.width).toBeCloseTo(240, 0);
    expect(fixed?.height).toBeCloseTo(16, 0);

    const [width, parentWidth] = await skeletons.nth(1).evaluate((el) => [
      el.getBoundingClientRect().width,
      (el.parentElement as HTMLElement).getBoundingClientRect().width,
    ]);
    expect(width).toBeCloseTo(parentWidth * 0.6, 0);
  });
});

test.describe('expandable', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('expandable');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('starts collapsed with its body hidden', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const expandable = page.locator('[data-widget-type="expandable"]');
    await expect(expandable).toHaveAttribute('data-expanded', 'false');
    await expect(expandable.locator('[data-expandable-body]')).toBeHidden();
    await expect(expandable.locator('[data-expandable-header]')).toHaveText('Details');
  });

  test('toggling the header round-trips through the server', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('[data-widget-type="layout"]')).toContainText('Expanded: false');

    await page.locator('[data-expandable-header]').click();

    // The handler sets state on the server, which pushes a new tree back.
    await expect(page.locator('[data-widget-type="layout"]')).toContainText('Expanded: true');
    await expect(page.locator('[data-widget-type="expandable"]')).toHaveAttribute(
      'data-expanded',
      'true'
    );
    await expect(page.locator('[data-expandable-body]')).toBeVisible();
    await expect(page.locator('[data-expandable-body]')).toContainText('Hidden body content');
  });

  test('toggling twice collapses again', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await page.locator('[data-expandable-header]').click();
    await expect(page.locator('[data-widget-type="layout"]')).toContainText('Expanded: true');

    await page.locator('[data-expandable-header]').click();
    await expect(page.locator('[data-widget-type="layout"]')).toContainText('Expanded: false');
  });
});

test.describe('list', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('list');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders one list_item per item', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const items = page.locator('[data-widget-type="list"] [data-widget-type="list_item"]');
    await expect(items).toHaveCount(3);
    await expect(items.nth(0).locator('[data-list-item-title]')).toHaveText('Inbox');
    await expect(items.nth(1).locator('[data-list-item-title]')).toHaveText('Drafts');
    await expect(items.nth(2).locator('[data-list-item-title]')).toHaveText('Archive');
  });

  test('renders the subtitle and icon when given', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const inbox = page.locator('[data-widget-type="list_item"]').first();
    await expect(inbox.locator('[data-list-item-subtitle]')).toHaveText('3 unread');
    await expect(inbox.locator('[data-icon-name="mail"]')).toBeAttached();

    // Archive has neither, so nothing should be emitted for them.
    const archive = page.locator('[data-widget-type="list_item"]').nth(2);
    await expect(archive.locator('[data-list-item-subtitle]')).toHaveCount(0);
  });

  test('each item is assigned its own widget id', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const ids = await page
      .locator('[data-widget-type="list_item"]')
      .evaluateAll((nodes) => nodes.map((n) => n.getAttribute('data-widget-id')));
    expect(new Set(ids).size).toBe(3);
    expect(ids.every((id) => id !== null)).toBe(true);
  });

  test('clicking an item round-trips through the server', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const root = page.locator('[data-widget-type="layout"]').first();
    await expect(root).toContainText('Selected:');

    await page.locator('[data-widget-type="list_item"]').first().click();
    await expect(root).toContainText('Selected: Inbox');

    await page.locator('[data-widget-type="list_item"]').nth(1).click();
    await expect(root).toContainText('Selected: Drafts');
  });

  test('an item without a handler is not clickable', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const archive = page.locator('[data-widget-type="list_item"]').nth(2);
    await expect(archive).not.toHaveCSS('cursor', 'pointer');

    // Clicking it must not disturb the selection set by a previous click.
    await page.locator('[data-widget-type="list_item"]').first().click();
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Selected: Inbox'
    );
    await archive.click();
    await expect(page.locator('[data-widget-type="layout"]').first()).toContainText(
      'Selected: Inbox'
    );
  });
});
