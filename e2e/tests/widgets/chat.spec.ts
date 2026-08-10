import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, stopHarness, type HarnessContext } from '../harness';

let harness: HarnessContext;

test.beforeAll(async () => {
  harness = await startHarness('chat');
});

test.afterAll(() => {
  stopHarness(harness);
});

test('chat renders its composer and streaming state', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const chat = page.locator('[data-widget-type="chat"]');
  await expect(chat).toBeVisible();
  await expect(chat).toHaveAttribute('data-streaming', 'true');
  await expect(chat.locator('[data-chat-input]')).toHaveAttribute(
    'placeholder',
    'Ask something…',
  );
});

test('chat renders one message per child with its sender', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  // Seed message, plus the status and loading bubbles the app always appends.
  const messages = page.locator('[data-widget-type="chat_message"]');
  await expect(messages).toHaveCount(3);
  await expect(messages.nth(0)).toHaveAttribute('data-sender', 'user');
  await expect(messages.nth(0)).toContainText('Hello');
  await expect(messages.nth(1)).toHaveAttribute('data-sender', 'assistant');
});

test('chat renders the loading and status indicators', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('[data-widget-type="chat_loading"]')).toHaveAttribute(
    'data-chat-loading',
    'true',
  );
  await expect(page.locator('[data-widget-type="chat_status"]')).toContainText('Searching…');
});

test('chat renders a button per quick reply', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  const chat = page.locator('[data-widget-type="chat"]');
  await expect(chat.locator('[data-quick-reply]')).toHaveCount(2);
  await expect(chat.locator('[data-quick-reply="Yes"]')).toBeVisible();
  await expect(chat.locator('[data-quick-reply="No"]')).toBeVisible();
});

test('sending a message appends it to the conversation', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('text=Messages: 1')).toBeVisible();

  const chat = page.locator('[data-widget-type="chat"]');
  await chat.locator('[data-chat-input]').fill('How are you?');
  await chat.locator('[data-chat-send]').click();

  // The server owns the message list, so a new bubble is proof the send arrived.
  await expect(page.locator('text=Messages: 2')).toBeVisible({ timeout: 5000 });
  await expect(page.locator('text=Last: How are you?')).toBeVisible();
  await expect(page.locator('[data-widget-type="chat_message"]')).toHaveCount(4);
});

test('picking a quick reply sends it as a message', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('text=Messages: 1')).toBeVisible();

  await page.locator('[data-quick-reply="Yes"]').click();

  await expect(page.locator('text=Last: Yes')).toBeVisible({ timeout: 5000 });
});

test('cancelling a streaming response reaches the server', async ({ page }) => {
  await navigateToHarness(page, harness.port);
  await expect(page.locator('text=Cancelled: 0')).toBeVisible();

  await page.locator('[data-chat-cancel]').click();

  await expect(page.locator('text=Cancelled: 1')).toBeVisible({ timeout: 5000 });
});
