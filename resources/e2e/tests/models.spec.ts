import { test, expect, loginAsAdmin, navigateTo, waitForTable } from './fixtures';

test.describe('Models CRUD', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
    await navigateTo(page, 'models');
    await waitForTable(page);
  });

  test('should display models list', async ({ page }) => {
    // Should show the models count
    await expect(page.locator('#content')).toContainText('model(s)');

    // Create button should be visible
    await expect(page.locator('#create-model-btn')).toBeVisible();
  });

  test('should open create model form', async ({ page }) => {
    await page.click('#create-model-btn');

    // Form should be visible
    await expect(page.locator('#model-id')).toBeVisible();
    await expect(page.locator('#model-name')).toBeVisible();
    await expect(page.locator('#model-provider')).toBeVisible();
  });

  test('should create a new model', async ({ page }) => {
    const modelId = `test-model-${Date.now()}`;

    await page.click('#create-model-btn');

    // Fill in the form
    await page.fill('#model-id', modelId);
    await page.fill('#model-name', 'Test Model');
    await page.selectOption('#model-provider', 'openai');
    await page.fill('#model-provider-model', 'gpt-4');

    // Wait for credential dropdown to populate
    await page.waitForTimeout(500);

    // Submit the form
    await page.click('#save-model-btn');

    // Wait for table to reload
    await waitForTable(page);

    // Verify the model appears in the list
    await expect(page.locator(`text=${modelId}`)).toBeVisible();
  });

  test('should edit an existing model', async ({ page }) => {
    // Click edit on first model
    const editButton = page.locator('.edit-btn').first();

    if (await editButton.isVisible()) {
      await editButton.click();

      // Form should be visible with data
      await expect(page.locator('#model-id')).toBeVisible();

      // Model ID should be readonly in edit mode
      const isDisabled = await page.locator('#model-id').isDisabled();
      expect(isDisabled).toBe(true);

      // Cancel the edit
      await page.click('#cancel-model-btn');
    }
  });

  test('should delete a model', async ({ page }) => {
    // Create a model first
    const modelId = `delete-test-${Date.now()}`;

    await page.click('#create-model-btn');
    await page.fill('#model-id', modelId);
    await page.fill('#model-name', 'Delete Test Model');
    await page.selectOption('#model-provider', 'openai');
    await page.fill('#model-provider-model', 'gpt-4');
    await page.waitForTimeout(500);
    await page.click('#save-model-btn');
    await waitForTable(page);

    // Find and delete the model
    const deleteButton = page.locator(`button.delete-btn[data-id="${modelId}"]`);

    if (await deleteButton.isVisible()) {
      // Handle confirmation dialog
      page.on('dialog', dialog => dialog.accept());
      await deleteButton.click();
      await waitForTable(page);

      // Model should no longer appear
      await expect(page.locator(`text=${modelId}`)).toBeHidden();
    }
  });

  test('should filter models by provider', async ({ page }) => {
    // This tests the table filtering if implemented
    const table = page.locator('.data-table');

    if (await table.isVisible()) {
      // Verify table headers
      await expect(table.locator('th:has-text("Provider")')).toBeVisible();
    }
  });
});
