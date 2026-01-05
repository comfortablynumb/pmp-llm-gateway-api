/**
 * Prompts CRUD view
 */
const Prompts = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listPrompts();
            $('#content').html(renderList(data.prompts || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(prompts) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${prompts.length} prompt(s)</p>
                <button id="create-prompt-btn" class="btn btn-primary">+ New Prompt</button>
            </div>

            ${prompts.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Content Preview</th>
                                <th>Version</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${prompts.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No prompts configured yet')}
        `;
    }

    function renderRow(prompt) {
        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(prompt.id)}</td>
                <td>${Utils.escapeHtml(prompt.name)}</td>
                <td class="text-gray-500 text-sm">${Utils.escapeHtml(Utils.truncate(prompt.content, 60))}</td>
                <td><span class="badge badge-gray">v${prompt.version || 1}</span></td>
                <td>
                    <button class="edit-btn text-blue-600 hover:text-blue-800 mr-3" data-id="${Utils.escapeHtml(prompt.id)}">Edit</button>
                    <button class="preview-btn text-green-600 hover:text-green-800 mr-3" data-id="${Utils.escapeHtml(prompt.id)}">Preview</button>
                    <button class="delete-btn text-red-600 hover:text-red-800" data-id="${Utils.escapeHtml(prompt.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function renderForm(prompt = null) {
        const isEdit = !!prompt;
        const title = isEdit ? 'Edit Prompt' : 'Create Prompt';

        return `
            <div class="max-w-3xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="prompt-form" class="card">
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                        <input type="text" name="id" value="${Utils.escapeHtml(prompt?.id || '')}"
                            class="form-input ${isEdit ? 'bg-gray-100' : ''}"
                            placeholder="my-prompt-id" ${isEdit ? 'readonly' : 'required'}>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" value="${Utils.escapeHtml(prompt?.name || '')}"
                            class="form-input" placeholder="My Prompt" required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <input type="text" name="description" value="${Utils.escapeHtml(prompt?.description || '')}"
                            class="form-input" placeholder="Optional description">
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Content</label>
                        <textarea name="content" rows="8" class="form-input font-mono text-sm"
                            placeholder="Enter your prompt content..." required>${Utils.escapeHtml(prompt?.content || '')}</textarea>
                        <p class="text-xs text-gray-500 mt-1">
                            Use <code class="bg-gray-100 px-1 rounded">\${var:name}</code> or
                            <code class="bg-gray-100 px-1 rounded">\${var:name:default}</code> for variables
                        </p>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Tags</label>
                        <input type="text" name="tags" value="${Utils.escapeHtml((prompt?.tags || []).join(', '))}"
                            class="form-input" placeholder="tag1, tag2, tag3">
                        <p class="text-xs text-gray-500 mt-1">Comma-separated list of tags</p>
                    </div>

                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">${isEdit ? 'Update' : 'Create'}</button>
                    </div>
                </form>
            </div>
        `;
    }

    function renderPreview(prompt) {
        // Extract variables from content
        const varRegex = /\$\{var:([a-zA-Z0-9_-]+)(?::([^}]*))?\}/g;
        const variables = [];
        let match;

        while ((match = varRegex.exec(prompt.content)) !== null) {
            variables.push({
                name: match[1],
                defaultValue: match[2] || ''
            });
        }

        return `
            <div class="max-w-3xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Preview: ${Utils.escapeHtml(prompt.name)}</h2>
                </div>

                <div class="card mb-6">
                    <h3 class="font-medium mb-4">Variables</h3>
                    ${variables.length > 0 ? `
                        <form id="preview-form" class="space-y-4">
                            ${variables.map(v => `
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">${Utils.escapeHtml(v.name)}</label>
                                    <input type="text" name="${Utils.escapeHtml(v.name)}"
                                        value="${Utils.escapeHtml(v.defaultValue)}"
                                        class="form-input" placeholder="Enter value...">
                                </div>
                            `).join('')}
                            <button type="submit" class="btn btn-primary">Render Preview</button>
                        </form>
                    ` : `
                        <p class="text-gray-500">No variables found in this prompt</p>
                    `}
                </div>

                <div class="card">
                    <h3 class="font-medium mb-4">Rendered Output</h3>
                    <pre id="rendered-output" class="bg-gray-50 p-4 rounded-lg text-sm whitespace-pre-wrap">${Utils.escapeHtml(prompt.content)}</pre>
                </div>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-prompt-btn').on('click', () => showForm());

        $('.edit-btn').on('click', function() {
            const id = $(this).data('id');
            showForm(id);
        });

        $('.preview-btn').on('click', function() {
            const id = $(this).data('id');
            showPreview(id);
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });
    }

    async function showForm(id = null) {
        let prompt = null;

        if (id) {
            $('#content').html(Utils.renderLoading());

            try {
                prompt = await API.getPrompt(id);
            } catch (error) {
                Utils.showToast('Failed to load prompt', 'error');
                return render();
            }
        }

        $('#content').html(renderForm(prompt));
        bindFormEvents(id);
    }

    async function showPreview(id) {
        $('#content').html(Utils.renderLoading());

        try {
            const prompt = await API.getPrompt(id);
            $('#content').html(renderPreview(prompt));
            bindPreviewEvents(id);
        } catch (error) {
            Utils.showToast('Failed to load prompt', 'error');
            render();
        }
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#prompt-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            // Convert tags string to array
            if (typeof formData.tags === 'string') {
                formData.tags = formData.tags.split(',').map(t => t.trim()).filter(t => t);
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                if (editId) {
                    delete formData.id;
                    await API.updatePrompt(editId, formData);
                    Utils.showToast('Prompt updated successfully', 'success');
                } else {
                    await API.createPrompt(formData);
                    Utils.showToast('Prompt created successfully', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    function bindPreviewEvents(promptId) {
        $('#back-btn').on('click', () => render());

        $('#preview-form').on('submit', async function(e) {
            e.preventDefault();
            const variables = Utils.getFormData(this);

            try {
                const result = await API.renderPrompt(promptId, variables);
                $('#rendered-output').text(result.rendered);
            } catch (error) {
                Utils.showToast(error.message, 'error');
            }
        });
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete prompt "${id}"?`)) {
            return;
        }

        try {
            await API.deletePrompt(id);
            Utils.showToast('Prompt deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
