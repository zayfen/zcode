import { test, expect } from '@playwright/test';

test.describe('Game Logic', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('canvas', { timeout: 15000 });
    // Give the React tree time to render the overlay
    await page.waitForTimeout(2000);
  });

  test('game starts with Player 1', async ({ page }) => {
    // The current player indicator should show "Player 1"
    const gameStatus = page.locator('[aria-label="Game status"]');
    await expect(gameStatus).toBeVisible();
    await expect(gameStatus).toContainText('Player 1');
  });

  test('keyboard R resets game', async ({ page }) => {
    // Press R to reset
    await page.keyboard.press('r');

    // After reset the game should still show Player 1 and IDLE phase
    await page.waitForTimeout(500);

    const gameStatus = page.locator('[aria-label="Game status"]');
    await expect(gameStatus).toBeVisible();
    await expect(gameStatus).toContainText('Player 1');

    // Phase indicator should show IDLE
    const phaseIndicator = page.locator('[aria-label="Current player"]');
    await expect(phaseIndicator).toContainText('IDLE');
  });

  test('keyboard T toggles camera', async ({ page }) => {
    // Press T to toggle camera mode
    await page.keyboard.press('t');
    await page.waitForTimeout(500);

    // Press T again to toggle back
    await page.keyboard.press('t');
    await page.waitForTimeout(500);

    // No assertion needed beyond "no crash" — canvas should still be visible
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();
  });

  test('keyboard U undo shows no errors', async ({ page }) => {
    // Collect console errors during this test
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    // Press U to attempt undo (no prior shot, so nothing should happen)
    await page.keyboard.press('u');
    await page.waitForTimeout(500);

    // Verify no console errors occurred
    expect(consoleErrors).toEqual([]);
  });
});
