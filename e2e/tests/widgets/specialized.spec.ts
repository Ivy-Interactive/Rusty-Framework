import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

// Events are queued but never dispatched back over the WebSocket, so these
// specs assert rendering only — no interaction round-trips.

test.describe('diff_view', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('diff_view');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders the diff text', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const view = page.locator('[data-widget-type="diff_view"]');
    await expect(view).toBeVisible();
    await expect(view.locator('pre')).toContainText('println!("new");');
  });

  test('exposes view type, language and revisions', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const view = page.locator('[data-widget-type="diff_view"]');
    await expect(view).toHaveAttribute('data-view-type', 'unified');
    await expect(view).toHaveAttribute('data-language', 'rust');
    await expect(view).toHaveAttribute('data-old-revision', 'HEAD~1');
    await expect(view).toHaveAttribute('data-new-revision', 'HEAD');
    await expect(view).toHaveAttribute('data-collapsible', 'true');
  });
});

test.describe('qr_code', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('qr_code');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders with its value and rendering parameters', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    // The harness renders QR codes as an attribute-only container, so it has no
    // size of its own — assert it is attached rather than visible.
    const code = page.locator('[data-widget-type="qr_code"]');
    await expect(code).toBeAttached();
    await expect(code).toHaveAttribute('data-value', 'https://example.com');
    await expect(code).toHaveAttribute('data-pixel-size', '6');
    await expect(code).toHaveAttribute('data-error-correction-level', 'medium');
  });
});

test.describe('activity_heatmap', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('activity_heatmap');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders one cell per activity', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    // Cells are empty spans, so the container has no size of its own — assert it
    // is attached rather than visible.
    const heatmap = page.locator('[data-widget-type="activity_heatmap"]');
    await expect(heatmap).toBeAttached();
    await expect(heatmap).toHaveAttribute('data-day-count', '3');
    await expect(heatmap.locator('[data-day]')).toHaveCount(3);
  });

  test('cells carry their date and count', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const heatmap = page.locator('[data-widget-type="activity_heatmap"]');
    await expect(heatmap.locator('[data-day="2026-01-02"]')).toHaveAttribute('data-count', '7');
  });

  test('exposes interval, label and date range', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const heatmap = page.locator('[data-widget-type="activity_heatmap"]');
    await expect(heatmap).toHaveAttribute('data-interval', 'daily');
    await expect(heatmap).toHaveAttribute('data-value-label', 'commits');
    await expect(heatmap).toHaveAttribute('data-start-date', '2026-01-01');
    await expect(heatmap).toHaveAttribute('data-end-date', '2026-01-31');
  });
});

test.describe('terminal', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('terminal');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders its initial content', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const terminal = page.locator('[data-widget-type="terminal"]');
    await expect(terminal).toBeVisible();
    await expect(terminal.locator('pre')).toContainText('$ echo hello');
  });

  test('exposes dimensions, cursor style and scrollback', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const terminal = page.locator('[data-widget-type="terminal"]');
    await expect(terminal).toHaveAttribute('data-cols', '80');
    await expect(terminal).toHaveAttribute('data-rows', '24');
    await expect(terminal).toHaveAttribute('data-cursor-style', 'bar');
    await expect(terminal).toHaveAttribute('data-scrollback', '1000');
  });
});

test.describe('rich_text_input', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('rich_text_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders an editable surface holding the current value', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="rich_text_input"]');
    await expect(input).toBeVisible();
    const editor = input.locator('[data-editor]');
    await expect(editor).toHaveAttribute('contenteditable', 'true');
    await expect(editor).toContainText('Hello');
  });

  test('renders the toolbar and placeholder', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="rich_text_input"]');
    await expect(input.locator('[data-toolbar]')).toBeAttached();
    await expect(input.locator('[data-editor]')).toHaveAttribute(
      'data-placeholder',
      'Write something…'
    );
  });

  test('is not marked invalid', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('[data-widget-type="rich_text_input"] [data-invalid]')).toHaveCount(0);
  });
});
