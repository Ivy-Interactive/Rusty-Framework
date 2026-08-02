import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeEach(async () => {
  harness = await startHarness('downloads');
});

test.afterEach(() => {
  stopHarness(harness);
});

/** The two URLs arrive over the push path, so wait for the first to be non-empty. */
async function downloadUrls(page: import('@playwright/test').Page): Promise<string[]> {
  const codes = page.locator('[data-widget-type="text_block"][data-variant="code"] code');
  await expect(codes.first()).toHaveText(/^\/rusty\/download\//, { timeout: 10_000 });
  return codes.allTextContents();
}

test('the mount effect pushes both download urls', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const urls = await downloadUrls(page);
  expect(urls).toHaveLength(2);
  for (const url of urls) {
    expect(url).toMatch(/^\/rusty\/download\/[0-9a-f-]{36}\/[0-9a-f-]{36}$/);
  }
});

test('the streaming download is a chunked attachment with the right disposition', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const [streamUrl] = await downloadUrls(page);

  const response = await page.request.get(`http://localhost:${harness.port}${streamUrl}`);
  expect(response.status()).toBe(200);
  const headers = response.headers();
  expect(headers['content-disposition']).toBe('attachment; filename="stream-export.csv"');
  expect(headers['content-type']).toBe('text/csv');
  expect(headers['transfer-encoding']).toBe('chunked');
  expect(headers['content-length']).toBeUndefined();
  expect(await response.text()).toBe('chunk-1;chunk-2;chunk-3;');
});

test('the buffered download is content-length delimited', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const [, bytesUrl] = await downloadUrls(page);

  const response = await page.request.get(`http://localhost:${harness.port}${bytesUrl}`);
  expect(response.status()).toBe(200);
  const headers = response.headers();
  expect(headers['content-disposition']).toBe('attachment; filename="buffered.json"');
  expect(headers['content-type']).toBe('application/json');
  expect(headers['content-length']).toBe('13');
  expect(headers['transfer-encoding']).toBeUndefined();
  expect(await response.text()).toBe('buffered-body');
});

test('a download url from another session is a 404', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const [streamUrl] = await downloadUrls(page);
  const foreign = streamUrl.replace(/\/[0-9a-f-]{36}\//, '/00000000-0000-0000-0000-000000000000/');
  const response = await page.request.get(`http://localhost:${harness.port}${foreign}`);
  expect(response.status()).toBe(404);
});
