import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

// The renderer sends a fixed stub payload rather than driving MediaRecorder or
// getUserMedia, so no device permissions are requested anywhere in this file.
// What is under test is that the serialized props arrive and the capture events
// round-trip, not that the browser can reach a microphone.

test.describe('audio_input', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('audio_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders with its recording parameters', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="audio_input"]');
    await expect(input).toBeVisible();
    await expect(input).toHaveAttribute('data-mime-type', 'audio/webm');
    await expect(input).toHaveAttribute('data-recording-label', 'Stop');
    await expect(input).toHaveAttribute('data-chunk-interval', '500');
    await expect(input).toHaveAttribute('data-sample-rate', '44100');
    await expect(input.locator('[data-waveform]')).toBeAttached();
    await expect(input.locator('[data-record]')).toHaveText('Record');
  });

  test('sends no upload url when the app configures none', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="audio_input"]');
    await expect(input).not.toHaveAttribute('data-upload-url');
  });

  test('a capture reaches the server', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('text=Captured:')).toBeVisible();

    await page.locator('[data-record]').click();

    await expect(
      page.locator('text=Captured: data:audio/webm;base64,c3R1Yg=='),
    ).toBeVisible({ timeout: 5000 });
  });

  test('focusing the control reaches the server', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('text=Focuses: 0')).toBeVisible();

    await page.locator('[data-record]').focus();

    // Counted, not a boolean: the re-render this triggers replaces the button and
    // fires a blur, which would flip a flag straight back.
    await expect(page.locator('text=Focuses: 1')).toBeVisible({ timeout: 5000 });
  });
});

test.describe('camera_input', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('camera_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders with its facing and capture modes', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="camera_input"]');
    await expect(input).toBeVisible();
    // Lowercase on purpose: the real Ivy widget hands this to getUserMedia.
    await expect(input).toHaveAttribute('data-facing-mode', 'environment');
    await expect(input).toHaveAttribute('data-capture-mode', 'image');
    await expect(input.locator('[data-preview]')).toContainText('Point at something');
  });

  test('a capture reaches the server', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('text=Captured:')).toBeVisible();

    await page.locator('[data-capture]').click();

    await expect(
      page.locator('text=Captured: data:image/png;base64,c3R1Yg=='),
    ).toBeVisible({ timeout: 5000 });
  });
});

test.describe('signature_input', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('signature_input');
  });

  test.afterAll(() => {
    stopHarness(harness);
  });

  test('renders a pad carrying the pen settings', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    const input = page.locator('[data-widget-type="signature_input"]');
    await expect(input).toBeVisible();
    const pad = input.locator('canvas[data-signature-pad]');
    await expect(pad).toBeAttached();
    await expect(pad).toHaveAttribute('data-pen', 'primary');
    await expect(pad).toHaveAttribute('data-pen-thickness', '2');
    await expect(pad).toHaveAttribute('data-placeholder', 'Sign here');
  });

  test('signing and clearing both reach the server', async ({ page }) => {
    await navigateToHarness(page, harness.port);
    await expect(page.locator('text=Signature:')).toBeVisible();

    await page.locator('[data-sign]').click();
    await expect(
      page.locator('text=Signature: data:image/png;base64,c3R1Yg=='),
    ).toBeVisible({ timeout: 5000 });

    await page.locator('[data-clear]').click();
    await expect(
      page.locator('text=Signature: data:image/png;base64,c3R1Yg=='),
    ).toBeHidden({ timeout: 5000 });
  });
});
