import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('wireframe');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('wireframe callout renders its title, text and child', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const callout = page.locator('[data-widget-type="wireframe_callout"]');
  await expect(callout).toHaveCount(1);
  await expect(callout.locator('.wireframe-callout-title')).toHaveText('UX note');
  await expect(callout.locator('.wireframe-callout-text')).toHaveText('Move this button up');
  await expect(callout.locator('[data-widget-type="text_block"]')).toHaveText('Save');
});

test('wireframe note renders its text and author', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const note = page.locator('[data-widget-type="wireframe_note"]');
  await expect(note).toHaveCount(1);
  await expect(note.locator('.wireframe-note-text')).toHaveText('Consider dark mode');
  await expect(note.locator('.wireframe-note-author')).toHaveText('Alex');
});
