import { test, expect } from '@playwright/test';

test.describe('Authentication', () => {
  test.beforeEach(async ({ page }) => {
    // Clear session storage before each test
    await page.goto('/ui/');
    await page.evaluate(() => sessionStorage.clear());
    await page.reload();
  });

  test('should show login modal when not authenticated', async ({ page }) => {
    await page.goto('/ui/');

    // Login modal should be visible
    await expect(page.locator('#auth-modal')).toBeVisible();
    await expect(page.locator('#username-input')).toBeVisible();
    await expect(page.locator('#password-input')).toBeVisible();
  });

  test('should show error for invalid credentials', async ({ page }) => {
    await page.goto('/ui/');

    // Fill in invalid credentials
    await page.fill('#username-input', 'invalid-user');
    await page.fill('#password-input', 'invalid-password');
    await page.click('#login-form button[type="submit"]');

    // Error message should be shown
    await expect(page.locator('#login-error')).toBeVisible();
  });

  test('should login successfully with valid credentials', async ({ page }) => {
    await page.goto('/ui/');

    // Fill in valid credentials (admin/admin from ADMIN_DEFAULT_PASSWORD)
    await page.fill('#username-input', 'admin');
    await page.fill('#password-input', 'admin123');
    await page.click('#login-form button[type="submit"]');

    // Login modal should be hidden
    await expect(page.locator('#auth-modal')).toBeHidden();

    // Main app should be visible
    await expect(page.locator('#app')).toBeVisible();

    // Dashboard should be shown
    await expect(page.locator('#content')).toContainText('Dashboard');
  });

  test('should persist session after page reload', async ({ page }) => {
    await page.goto('/ui/');

    // Login first
    await page.fill('#username-input', 'admin');
    await page.fill('#password-input', 'admin123');
    await page.click('#login-form button[type="submit"]');
    await expect(page.locator('#auth-modal')).toBeHidden();

    // Reload the page
    await page.reload();

    // Should still be authenticated
    await expect(page.locator('#auth-modal')).toBeHidden();
    await expect(page.locator('#app')).toBeVisible();
  });

  test('should logout successfully', async ({ page }) => {
    await page.goto('/ui/');

    // Login first
    await page.fill('#username-input', 'admin');
    await page.fill('#password-input', 'admin123');
    await page.click('#login-form button[type="submit"]');
    await expect(page.locator('#auth-modal')).toBeHidden();

    // Click logout button
    await page.click('#logout-btn');

    // Login modal should be shown again
    await expect(page.locator('#auth-modal')).toBeVisible();
  });

  test('should require username', async ({ page }) => {
    await page.goto('/ui/');

    // Submit without username
    await page.fill('#password-input', 'admin123');
    await page.click('#login-form button[type="submit"]');

    // Error should be shown
    await expect(page.locator('#login-error')).toBeVisible();
    await expect(page.locator('#login-error')).toContainText('username');
  });

  test('should require password', async ({ page }) => {
    await page.goto('/ui/');

    // Submit without password
    await page.fill('#username-input', 'admin');
    await page.click('#login-form button[type="submit"]');

    // Error should be shown
    await expect(page.locator('#login-error')).toBeVisible();
    await expect(page.locator('#login-error')).toContainText('password');
  });
});
