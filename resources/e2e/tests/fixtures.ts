import { test as base, expect, Page } from '@playwright/test';

/**
 * Login as admin user
 */
export async function loginAsAdmin(page: Page): Promise<void> {
  await page.goto('/ui/');

  // Check if already logged in
  const isModalHidden = await page.locator('#auth-modal').isHidden();

  if (!isModalHidden) {
    await page.fill('#username-input', 'admin');
    await page.fill('#password-input', 'admin123');
    await page.click('#login-form button[type="submit"]');
    await expect(page.locator('#auth-modal')).toBeHidden();
  }
}

/**
 * Navigate to a specific view
 */
export async function navigateTo(page: Page, view: string): Promise<void> {
  await page.goto(`/ui/#${view}`);
  await page.waitForLoadState('networkidle');
}

/**
 * Wait for table to load
 */
export async function waitForTable(page: Page): Promise<void> {
  // Wait for loading spinner to disappear and content to appear
  await page.waitForSelector('.data-table, .text-gray-500:has-text("No")', { timeout: 10000 });
}

/**
 * Get text content with fallback
 */
export async function getTextContent(page: Page, selector: string): Promise<string> {
  const element = page.locator(selector);
  return (await element.textContent()) || '';
}

/**
 * Extended test with authenticated context
 */
export const test = base.extend<{ authenticatedPage: Page }>({
  authenticatedPage: async ({ page }, use) => {
    await loginAsAdmin(page);
    await use(page);
  },
});

export { expect };
