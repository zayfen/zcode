import { test, expect } from '@playwright/test';

test.describe('Rendering', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for the 3D canvas to render
    await page.waitForSelector('canvas', { timeout: 15000 });
  });

  test('canvas renders on page load', async ({ page }) => {
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();
  });

  test('GameUI overlay is present', async ({ page }) => {
    // The GameUI renders a div with aria-label="Game status"
    const gameStatus = page.locator('[aria-label="Game status"]');
    await expect(gameStatus).toBeVisible();
  });

  test('page title contains pool', async ({ page }) => {
    const title = await page.title();
    expect(title.toLowerCase()).toContain('pool');
  });
});
