import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

test.describe('spacer', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('spacer');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders between its siblings', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const spacer = page.locator('[data-widget-type="spacer"]');
    await expect(spacer).toBeVisible();

    // The point of a Spacer is its position, so assert the ordering rather
    // than a computed size: it must sit between Left and Right.
    const row = page.locator('[data-widget-type="layout"][data-direction="horizontal"]');
    const types = await row.locator(':scope > [data-widget-type]').evaluateAll(
      (nodes) => nodes.map((n) => n.getAttribute('data-widget-type'))
    );
    expect(types).toEqual(['text_block', 'spacer', 'text_block']);
  });
});

test.describe('separator', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('separator');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders horizontal and vertical orientations', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(
      page.locator('[data-widget-type="separator"][data-orientation="horizontal"]').first()
    ).toBeAttached();
    await expect(
      page.locator('[data-widget-type="separator"][data-orientation="vertical"]')
    ).toBeAttached();
  });

  test('renders inline text when given', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const labelled = page.locator('[data-widget-type="separator"] [data-separator-text]');
    await expect(labelled).toHaveCount(1);
    await expect(labelled).toHaveText('OR');
  });
});

test.describe('container', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('container');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders its children', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const containers = page.locator('[data-widget-type="container"]');
    await expect(containers).toHaveCount(2);
    await expect(containers.first()).toContainText('Bordered and rounded');
  });

  test('exposes border and rounded flags', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const bordered = page.locator('[data-widget-type="container"]').first();
    await expect(bordered).toHaveAttribute('data-border', 'true');
    await expect(bordered).toHaveAttribute('data-rounded', 'true');
  });

  test('applies width, height and background', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const sized = page.locator('[data-widget-type="container"]').nth(1);
    await expect(sized).toHaveAttribute('data-background', '#eef4ff');

    // Size::to_css renders Px as a CSS length, so the box is really 200x80.
    const box = await sized.boundingBox();
    expect(box?.width).toBeCloseTo(200, 0);
    expect(box?.height).toBeCloseTo(80, 0);
  });
});

test.describe('layout sizing', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('layout_sizing');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('applies a pixel width and height', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const fixed = page.locator('[data-widget-type="layout"][data-direction="horizontal"]').first();
    const box = await fixed.boundingBox();
    expect(box?.width).toBeCloseTo(320, 0);
    expect(box?.height).toBeCloseTo(48, 0);
  });

  test('applies a percentage width', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const half = page.locator('[data-widget-type="layout"][data-direction="horizontal"]').nth(1);
    // Size::Percent must reach the client as `50%`, not a bare `50`, which CSS
    // would reject outright.
    await expect(half).toHaveCSS('width', /px$/);
    const [width, parentWidth] = await half.evaluate((el) => [
      el.getBoundingClientRect().width,
      (el.parentElement as HTMLElement).getBoundingClientRect().width,
    ]);
    expect(width).toBeCloseTo(parentWidth / 2, 0);
  });

  test('sets flex-wrap when wrap is enabled', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const wrapping = page.locator('[data-widget-type="layout"][data-wrap="true"]');
    await expect(wrapping).toHaveCount(1);
    await expect(wrapping).toHaveCSS('flex-wrap', 'wrap');
  });

  test('omits width and height when unset', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    // The outer vertical layout sets no explicit size, so no inline
    // width/height should leak through from the serialized `null`.
    const inline = await page
      .locator('[data-widget-type="layout"][data-direction="vertical"]')
      .first()
      .evaluate((el) => (el as HTMLElement).style.width + '|' + (el as HTMLElement).style.height);
    expect(inline).toBe('|');
  });
});
