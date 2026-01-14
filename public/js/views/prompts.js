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
                    <button class="details-btn btn-sm btn-primary mr-2" data-id="${Utils.escapeHtml(prompt.id)}">Details</button>
                    <button class="edit-btn btn-sm btn-edit mr-2" data-id="${Utils.escapeHtml(prompt.id)}">Edit</button>
                    <button class="preview-btn btn-sm btn-success-sm mr-2" data-id="${Utils.escapeHtml(prompt.id)}">Preview</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(prompt.id)}">Delete</button>
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

                    <!-- Structured Output Schema -->
                    <div class="mb-4">
                        <div class="flex items-center justify-between mb-2">
                            <label class="block text-sm font-medium text-gray-700">Structured Output Schema (Optional)</label>
                            <label class="flex items-center">
                                <input type="checkbox" id="enable-output-schema" class="mr-2"
                                    ${prompt?.output_schema ? 'checked' : ''}>
                                <span class="text-sm text-gray-600">Enable</span>
                            </label>
                        </div>
                        <div id="output-schema-fields" class="${prompt?.output_schema ? '' : 'hidden'}">
                            <div class="mb-2">
                                <label class="block text-xs font-medium text-gray-600 mb-1">Schema Name</label>
                                <input type="text" name="output_schema_name"
                                    value="${Utils.escapeHtml(prompt?.output_schema?.name || '')}"
                                    class="form-input" placeholder="response_schema">
                            </div>
                            <div class="mb-2">
                                <label class="block text-xs font-medium text-gray-600 mb-1">JSON Schema</label>
                                <textarea name="output_schema_json" rows="6" class="form-input font-mono text-sm"
                                    placeholder='{"type":"object","properties":{"field":{"type":"string"}},"required":["field"]}'>${Utils.escapeHtml(prompt?.output_schema?.schema ? JSON.stringify(prompt.output_schema.schema, null, 2) : '')}</textarea>
                            </div>
                            <div class="flex items-center">
                                <input type="checkbox" name="output_schema_strict" id="output_schema_strict" class="mr-2"
                                    ${prompt?.output_schema?.strict !== false ? 'checked' : ''}>
                                <label for="output_schema_strict" class="text-sm text-gray-600">Strict mode (enforce exact schema)</label>
                            </div>
                        </div>
                        <p class="text-xs text-gray-500 mt-1">Define a JSON schema for structured LLM responses</p>
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

    function renderDetails(prompt, versions) {
        const tagsHtml = prompt.tags && prompt.tags.length > 0
            ? prompt.tags.map(t => `<span class="badge badge-primary mr-1">${Utils.escapeHtml(t)}</span>`).join('')
            : '<span class="text-gray-400">No tags</span>';

        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Prompt Details: ${Utils.escapeHtml(prompt.name)}</h2>
                </div>

                <!-- Metadata -->
                <div class="card mb-6">
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <span class="text-sm text-gray-600">ID:</span>
                            <span class="font-mono ml-2">${Utils.escapeHtml(prompt.id)}</span>
                        </div>
                        <div>
                            <span class="text-sm text-gray-600">Current Version:</span>
                            <span class="badge badge-success ml-2">v${versions.current_version}</span>
                        </div>
                        <div>
                            <span class="text-sm text-gray-600">Status:</span>
                            <span class="badge ${prompt.enabled ? 'badge-success' : 'badge-gray'} ml-2">${prompt.enabled ? 'Enabled' : 'Disabled'}</span>
                        </div>
                        <div>
                            <span class="text-sm text-gray-600">Tags:</span>
                            <span class="ml-2">${tagsHtml}</span>
                        </div>
                    </div>
                    ${prompt.description ? `
                        <div class="mt-4 pt-4 border-t">
                            <span class="text-sm text-gray-600">Description:</span>
                            <p class="mt-1">${Utils.escapeHtml(prompt.description)}</p>
                        </div>
                    ` : ''}
                    ${prompt.output_schema ? `
                        <div class="mt-4 pt-4 border-t">
                            <span class="text-sm text-gray-600">Structured Output Schema:</span>
                            <div class="mt-2 bg-gray-50 p-3 rounded-lg">
                                <div class="flex items-center gap-4 mb-2">
                                    <span class="font-medium">${Utils.escapeHtml(prompt.output_schema.name)}</span>
                                    <span class="badge ${prompt.output_schema.strict ? 'badge-success' : 'badge-gray'}">${prompt.output_schema.strict ? 'Strict' : 'Non-strict'}</span>
                                </div>
                                <pre class="text-xs bg-white p-2 rounded border overflow-x-auto">${Utils.escapeHtml(JSON.stringify(prompt.output_schema.schema, null, 2))}</pre>
                            </div>
                        </div>
                    ` : ''}
                </div>

                <!-- Current Content -->
                <div class="card mb-6">
                    <h3 class="font-medium mb-4">Current Content (v${versions.current_version})</h3>
                    <pre class="bg-gray-50 p-4 rounded-lg text-sm whitespace-pre-wrap overflow-x-auto">${Utils.escapeHtml(prompt.content)}</pre>
                </div>

                <!-- Version History -->
                <div class="card">
                    <h3 class="font-medium mb-4">Version History</h3>
                    ${versions.versions.length > 0 ? `
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>Version</th>
                                    <th>Created At</th>
                                    <th>Message</th>
                                    <th>Actions</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${versions.versions.map(v => `
                                    <tr>
                                        <td><span class="badge badge-gray">v${v.version}</span></td>
                                        <td class="text-sm">${Utils.formatDate(v.created_at)}</td>
                                        <td class="text-sm text-gray-600">${Utils.escapeHtml(v.message || '-')}</td>
                                        <td>
                                            <button class="view-version-btn btn-sm btn-edit mr-2"
                                                data-version="${v.version}"
                                                data-content="${Utils.escapeHtml(v.content)}">View</button>
                                            <button class="revert-version-btn btn-sm btn-warning"
                                                data-version="${v.version}"
                                                data-prompt-id="${Utils.escapeHtml(prompt.id)}">Revert</button>
                                        </td>
                                    </tr>
                                `).join('')}
                            </tbody>
                        </table>
                    ` : `
                        <p class="text-gray-500">No previous versions available. Version history is created when the content is updated.</p>
                    `}
                </div>
            </div>

            <!-- Version Content Modal -->
            <div id="version-modal" class="hidden fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                <div class="bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden">
                    <div class="p-4 border-b flex justify-between items-center">
                        <h3 class="font-medium" id="version-modal-title">Version Content</h3>
                        <button id="close-version-modal" class="text-gray-500 hover:text-gray-700">&times;</button>
                    </div>
                    <div class="p-4 overflow-y-auto max-h-[60vh]">
                        <pre id="version-modal-content" class="bg-gray-50 p-4 rounded-lg text-sm whitespace-pre-wrap"></pre>
                    </div>
                </div>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-prompt-btn').on('click', () => showForm());

        $('.details-btn').on('click', function() {
            const id = $(this).data('id');
            showDetails(id);
        });

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

    async function showDetails(id) {
        $('#content').html(Utils.renderLoading());

        try {
            const [prompt, versions] = await Promise.all([
                API.getPrompt(id),
                API.listPromptVersions(id)
            ]);
            $('#content').html(renderDetails(prompt, versions));
            bindDetailsEvents(id);
        } catch (error) {
            Utils.showToast('Failed to load prompt details', 'error');
            render();
        }
    }

    function bindDetailsEvents(promptId) {
        $('#back-btn').on('click', () => render());

        // View version content
        $('.view-version-btn').on('click', function() {
            const version = $(this).data('version');
            const content = $(this).data('content');
            $('#version-modal-title').text(`Version ${version} Content`);
            $('#version-modal-content').text(content);
            $('#version-modal').removeClass('hidden');
        });

        // Close modal (only via close button, not backdrop click)
        $('#close-version-modal').on('click', function() {
            $('#version-modal').addClass('hidden');
        });

        // Revert to version
        $('.revert-version-btn').on('click', async function() {
            const version = $(this).data('version');
            const id = $(this).data('prompt-id');

            if (!Utils.confirm(`Are you sure you want to revert to version ${version}? This will create a new version with the old content.`)) {
                return;
            }

            try {
                await API.revertPromptVersion(id, version);
                Utils.showToast(`Reverted to version ${version}`, 'success');
                showDetails(id);
            } catch (error) {
                Utils.showToast(error.message, 'error');
            }
        });
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        // Toggle output schema fields visibility
        $('#enable-output-schema').on('change', function() {
            if ($(this).is(':checked')) {
                $('#output-schema-fields').removeClass('hidden');
            } else {
                $('#output-schema-fields').addClass('hidden');
            }
        });

        $('#prompt-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            // Convert tags string to array
            if (typeof formData.tags === 'string') {
                formData.tags = formData.tags.split(',').map(t => t.trim()).filter(t => t);
            }

            // Build output_schema if enabled
            if ($('#enable-output-schema').is(':checked')) {
                const schemaName = $('input[name="output_schema_name"]').val().trim();
                const schemaJson = $('textarea[name="output_schema_json"]').val().trim();
                const schemaStrict = $('input[name="output_schema_strict"]').is(':checked');

                if (schemaName && schemaJson) {
                    try {
                        formData.output_schema = {
                            name: schemaName,
                            schema: JSON.parse(schemaJson),
                            strict: schemaStrict
                        };
                    } catch (parseError) {
                        Utils.showToast('Invalid JSON in output schema', 'error');
                        return;
                    }
                }
            }

            // Remove temporary form fields from payload
            delete formData.output_schema_name;
            delete formData.output_schema_json;
            delete formData.output_schema_strict;

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

            // Get variables as strings (don't use getFormData which converts types)
            const variables = {};
            $(this).find('input[type="text"]').each(function() {
                const name = $(this).attr('name');
                const value = $(this).val();

                if (name && value !== '') {
                    variables[name] = value;
                }
            });

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
