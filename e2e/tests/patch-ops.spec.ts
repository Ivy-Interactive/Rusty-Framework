import { test, expect } from '@playwright/test';
import { startHarness, navigateToHarness, type HarnessContext } from './harness';

test.describe('applyPatch operations', () => {
  let harness: HarnessContext;

  test.beforeAll(async () => {
    harness = await startHarness('button');
  });

  test.afterAll(() => {
    harness.process.kill();
  });

  test.beforeEach(async ({ page }) => {
    await navigateToHarness(page, harness.port);
  });

  test('replace on a scalar field', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { label: 'old', count: 5 };
      const patch = { op: 'replace', path: '/label', value: 'new' };
      (window as any).applyPatch(tree, patch);
      return tree;
    });
    expect(result).toEqual({ label: 'new', count: 5 });
  });

  test('add a previously-absent optional key', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { label: 'Click me' };
      const patch = { op: 'add', path: '/variant', value: 'primary' };
      (window as any).applyPatch(tree, patch);
      return tree;
    });
    expect(result).toEqual({ label: 'Click me', variant: 'primary' });
  });

  test('remove an optional key', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { label: 'Click me', variant: 'primary' };
      const patch = { op: 'remove', path: '/variant' };
      (window as any).applyPatch(tree, patch);
      return tree;
    });
    expect(result).toEqual({ label: 'Click me' });
  });

  test('add nested through an array index', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { children: [{ name: 'field1', value: 'test' }] };
      const patch = { op: 'add', path: '/children/0/invalid', value: 'Required' };
      (window as any).applyPatch(tree, patch);
      return tree;
    });
    expect(result).toEqual({ children: [{ name: 'field1', value: 'test', invalid: 'Required' }] });
  });

  test('remove an array element splices', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { items: ['a', 'b', 'c'] };
      const patch = { op: 'remove', path: '/items/1' };
      (window as any).applyPatch(tree, patch);
      return tree;
    });
    expect(result).toEqual({ items: ['a', 'c'] });
  });

  test('unknown op lands in rustyPatchErrors', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { label: 'unchanged' };
      (window as any).rustyPatchErrors = [];
      const patch = { op: 'move', path: '/label', from: '/foo' };
      (window as any).applyPatch(tree, patch);
      return {
        tree,
        errors: (window as any).rustyPatchErrors,
      };
    });
    expect(result.tree).toEqual({ label: 'unchanged' });
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]).toEqual({ op: 'move', path: '/label', from: '/foo' });
  });

  test('missing intermediate node is a no-op', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { a: 1 };
      const patch = { op: 'add', path: '/x/y/z', value: 'deep' };
      (window as any).applyPatch(tree, patch);
      return tree;
    });
    expect(result).toEqual({ a: 1 });
  });

  test('empty path is a no-op', async ({ page }) => {
    const result = await page.evaluate(() => {
      const tree = { label: 'unchanged' };
      const patch = { op: 'replace', path: '', value: 'new' };
      (window as any).applyPatch(tree, patch);
      return tree;
    });
    expect(result).toEqual({ label: 'unchanged' });
  });
});
