import { test, expect, loginAsAdmin } from './fixtures';

test.describe('Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  test('should navigate to dashboard', async ({ page }) => {
    await page.goto('/ui/#dashboard');
    await expect(page.locator('#content')).toContainText('Dashboard');
  });

  test('should navigate to models', async ({ page }) => {
    await page.click('a[href="#models"]');
    await expect(page.locator('#content')).toContainText('model');
  });

  test('should navigate to prompts', async ({ page }) => {
    await page.click('a[href="#prompts"]');
    await expect(page.locator('#content')).toContainText('prompt');
  });

  test('should navigate to api-keys', async ({ page }) => {
    await page.click('a[href="#api-keys"]');
    await expect(page.locator('#content')).toContainText('API key');
  });

  test('should navigate to workflows', async ({ page }) => {
    await page.click('a[href="#workflows"]');
    await expect(page.locator('#content')).toContainText('workflow');
  });

  test('should navigate to credentials', async ({ page }) => {
    await page.click('a[href="#credentials"]');
    await expect(page.locator('#content')).toContainText('Credential');
  });

  test('should navigate to experiments', async ({ page }) => {
    await page.click('a[href="#experiments"]');
    await expect(page.locator('#content')).toContainText('experiment');
  });

  test('should navigate to knowledge-bases', async ({ page }) => {
    await page.click('a[href="#knowledge-bases"]');
    await expect(page.locator('#content')).toContainText('Knowledge');
  });

  test('should navigate to teams', async ({ page }) => {
    await page.click('a[href="#teams"]');
    await expect(page.locator('#content')).toContainText('team');
  });

  test('should navigate to budgets', async ({ page }) => {
    await page.click('a[href="#budgets"]');
    await expect(page.locator('#content')).toContainText('budget');
  });

  test('should navigate to webhooks', async ({ page }) => {
    await page.click('a[href="#webhooks"]');
    await expect(page.locator('#content')).toContainText('webhook');
  });

  test('should navigate to test-cases', async ({ page }) => {
    await page.click('a[href="#test-cases"]');
    await expect(page.locator('#content')).toContainText('Test');
  });

  test('should navigate to configuration', async ({ page }) => {
    await page.click('a[href="#configuration"]');
    await expect(page.locator('#content')).toContainText('Configuration');
  });

  test('should navigate to execution-logs', async ({ page }) => {
    await page.click('a[href="#execution-logs"]');
    await expect(page.locator('#content')).toContainText('Execution');
  });

  test('should highlight active navigation item', async ({ page }) => {
    await page.goto('/ui/#models');

    // The models link should have active class
    const modelsLink = page.locator('a[href="#models"]');
    await expect(modelsLink).toHaveClass(/bg-blue-50|active/);
  });

  test('should show correct page title', async ({ page }) => {
    await page.goto('/ui/#models');

    // Page should have models in the heading
    await expect(page.locator('h1, h2')).toContainText(/Models/i);
  });
});
