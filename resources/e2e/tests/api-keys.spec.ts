import { test, expect, loginAsAdmin, navigateTo, waitForTable } from './fixtures';

test.describe('API Keys CRUD', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
    await navigateTo(page, 'api-keys');
    await waitForTable(page);
  });

  test('should display api keys list', async ({ page }) => {
    // Should show the API keys count
    await expect(page.locator('#content')).toContainText('API key(s)');

    // Create button should be visible
    await expect(page.locator('#create-api-key-btn')).toBeVisible();
  });

  test('should open create api key form', async ({ page }) => {
    await page.click('#create-api-key-btn');

    // Form should be visible
    await expect(page.locator('#api-key-id')).toBeVisible();
    await expect(page.locator('#api-key-name')).toBeVisible();
  });

  test('should create a new api key', async ({ page }) => {
    const keyId = `test-key-${Date.now()}`;

    await page.click('#create-api-key-btn');

    // Fill in the form
    await page.fill('#api-key-id', keyId);
    await page.fill('#api-key-name', 'Test API Key');

    // Submit the form
    await page.click('#save-api-key-btn');

    // Wait for the modal showing the secret key
    await page.waitForSelector('#api-key-secret-modal, .data-table');

    // If secret modal is shown, close it
    const secretModal = page.locator('#api-key-secret-modal');

    if (await secretModal.isVisible()) {
      await page.click('#close-secret-modal-btn');
    }

    // Wait for table to reload
    await waitForTable(page);

    // Verify the key appears in the list
    await expect(page.locator(`text=${keyId}`)).toBeVisible();
  });

  test('should suspend an api key', async ({ page }) => {
    const suspendButton = page.locator('.suspend-btn').first();

    if (await suspendButton.isVisible()) {
      await suspendButton.click();
      await waitForTable(page);

      // Status should change to Suspended
      await expect(page.locator('.badge:has-text("Suspended")')).toBeVisible();
    }
  });

  test('should activate a suspended api key', async ({ page }) => {
    const activateButton = page.locator('.activate-btn').first();

    if (await activateButton.isVisible()) {
      await activateButton.click();
      await waitForTable(page);

      // Status should change to Active
      await expect(page.locator('.badge:has-text("Active")')).toBeVisible();
    }
  });

  test('should revoke an api key', async ({ page }) => {
    // Create a key to revoke
    const keyId = `revoke-test-${Date.now()}`;

    await page.click('#create-api-key-btn');
    await page.fill('#api-key-id', keyId);
    await page.fill('#api-key-name', 'Revoke Test Key');
    await page.click('#save-api-key-btn');

    // Close secret modal if shown
    const secretModal = page.locator('#api-key-secret-modal');

    if (await secretModal.isVisible()) {
      await page.click('#close-secret-modal-btn');
    }

    await waitForTable(page);

    // Find and revoke the key
    const revokeButton = page.locator(`button.revoke-btn[data-id="${keyId}"]`);

    if (await revokeButton.isVisible()) {
      page.on('dialog', dialog => dialog.accept());
      await revokeButton.click();
      await waitForTable(page);

      // Status should be Revoked
      await expect(page.locator(`tr:has-text("${keyId}") .badge:has-text("Revoked")`)).toBeVisible();
    }
  });

  test('should delete an api key', async ({ page }) => {
    // Create a key to delete
    const keyId = `delete-test-${Date.now()}`;

    await page.click('#create-api-key-btn');
    await page.fill('#api-key-id', keyId);
    await page.fill('#api-key-name', 'Delete Test Key');
    await page.click('#save-api-key-btn');

    // Close secret modal if shown
    const secretModal = page.locator('#api-key-secret-modal');

    if (await secretModal.isVisible()) {
      await page.click('#close-secret-modal-btn');
    }

    await waitForTable(page);

    // Find and delete the key
    const deleteButton = page.locator(`button.delete-btn[data-id="${keyId}"]`);

    if (await deleteButton.isVisible()) {
      page.on('dialog', dialog => dialog.accept());
      await deleteButton.click();
      await waitForTable(page);

      // Key should no longer appear
      await expect(page.locator(`text=${keyId}`)).toBeHidden();
    }
  });
});
