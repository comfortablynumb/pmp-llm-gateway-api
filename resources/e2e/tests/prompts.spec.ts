import { test, expect, loginAsAdmin, navigateTo, waitForTable } from './fixtures';

test.describe('Prompts CRUD', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
    await navigateTo(page, 'prompts');
    await waitForTable(page);
  });

  test('should display prompts list', async ({ page }) => {
    // Should show the prompts count
    await expect(page.locator('#content')).toContainText('prompt(s)');

    // Create button should be visible
    await expect(page.locator('#create-prompt-btn')).toBeVisible();
  });

  test('should open create prompt form', async ({ page }) => {
    await page.click('#create-prompt-btn');

    // Form should be visible
    await expect(page.locator('#prompt-id')).toBeVisible();
    await expect(page.locator('#prompt-name')).toBeVisible();
    await expect(page.locator('#prompt-content')).toBeVisible();
  });

  test('should create a new prompt', async ({ page }) => {
    const promptId = `test-prompt-${Date.now()}`;

    await page.click('#create-prompt-btn');

    // Fill in the form
    await page.fill('#prompt-id', promptId);
    await page.fill('#prompt-name', 'Test Prompt');
    await page.fill('#prompt-content', 'You are a helpful assistant. User topic: ${var:topic:general}');

    // Submit the form
    await page.click('#save-prompt-btn');

    // Wait for table to reload
    await waitForTable(page);

    // Verify the prompt appears in the list
    await expect(page.locator(`text=${promptId}`)).toBeVisible();
  });

  test('should show variable preview', async ({ page }) => {
    await page.click('#create-prompt-btn');

    // Fill in content with variables
    await page.fill('#prompt-content', 'Hello ${var:name:World}!');

    // Check for variable detection
    await page.waitForTimeout(500);

    // Variables section should show the variable
    const variablesText = await page.locator('#content').textContent();
    expect(variablesText).toContain('name');
  });

  test('should edit an existing prompt', async ({ page }) => {
    const editButton = page.locator('.edit-btn').first();

    if (await editButton.isVisible()) {
      await editButton.click();

      // Form should be visible with data
      await expect(page.locator('#prompt-id')).toBeVisible();

      // Prompt ID should be readonly in edit mode
      const isDisabled = await page.locator('#prompt-id').isDisabled();
      expect(isDisabled).toBe(true);

      // Cancel the edit
      await page.click('#cancel-prompt-btn');
    }
  });

  test('should render prompt with variables', async ({ page }) => {
    const renderButton = page.locator('.render-btn').first();

    if (await renderButton.isVisible()) {
      await renderButton.click();

      // Render modal should appear
      await expect(page.locator('#render-modal')).toBeVisible();
    }
  });

  test('should delete a prompt', async ({ page }) => {
    // Create a prompt first
    const promptId = `delete-test-${Date.now()}`;

    await page.click('#create-prompt-btn');
    await page.fill('#prompt-id', promptId);
    await page.fill('#prompt-name', 'Delete Test Prompt');
    await page.fill('#prompt-content', 'Test content');
    await page.click('#save-prompt-btn');
    await waitForTable(page);

    // Find and delete the prompt
    const deleteButton = page.locator(`button.delete-btn[data-id="${promptId}"]`);

    if (await deleteButton.isVisible()) {
      page.on('dialog', dialog => dialog.accept());
      await deleteButton.click();
      await waitForTable(page);

      // Prompt should no longer appear
      await expect(page.locator(`text=${promptId}`)).toBeHidden();
    }
  });
});
