/**
 * Models CRUD view
 */
const Models = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listModels();
            $('#content').html(renderList(data.models || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(models) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${models.length} model(s)</p>
                <button id="create-model-btn" class="btn btn-primary">+ New Model</button>
            </div>

            ${models.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Provider</th>
                                <th>Provider Model</th>
                                <th>Status</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${models.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No models configured yet')}
        `;
    }

    function renderRow(model) {
        const statusClass = model.enabled ? 'badge-success' : 'badge-gray';
        const statusText = model.enabled ? 'Enabled' : 'Disabled';

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(model.id)}</td>
                <td>${Utils.escapeHtml(model.name)}</td>
                <td>${Utils.escapeHtml(model.provider)}</td>
                <td class="font-mono text-sm">${Utils.escapeHtml(model.provider_model)}</td>
                <td><span class="badge ${statusClass}">${statusText}</span></td>
                <td>
                    <button class="edit-btn text-blue-600 hover:text-blue-800 mr-3" data-id="${Utils.escapeHtml(model.id)}">Edit</button>
                    <button class="delete-btn text-red-600 hover:text-red-800" data-id="${Utils.escapeHtml(model.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function renderForm(model = null) {
        const isEdit = !!model;
        const title = isEdit ? 'Edit Model' : 'Create Model';

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="model-form" class="card">
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                        <input type="text" name="id" value="${Utils.escapeHtml(model?.id || '')}"
                            class="form-input ${isEdit ? 'bg-gray-100' : ''}"
                            placeholder="my-model-id" ${isEdit ? 'readonly' : 'required'}>
                        <p class="text-xs text-gray-500 mt-1">Alphanumeric and hyphens only, max 50 chars</p>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" value="${Utils.escapeHtml(model?.name || '')}"
                            class="form-input" placeholder="My Model" required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Provider</label>
                        <select name="provider" class="form-input" required>
                            <option value="openai" ${model?.provider === 'openai' ? 'selected' : ''}>OpenAI</option>
                            <option value="anthropic" ${model?.provider === 'anthropic' ? 'selected' : ''}>Anthropic</option>
                            <option value="azure_openai" ${model?.provider === 'azure_openai' ? 'selected' : ''}>Azure OpenAI</option>
                            <option value="aws_bedrock" ${model?.provider === 'aws_bedrock' ? 'selected' : ''}>AWS Bedrock</option>
                        </select>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Provider Model</label>
                        <input type="text" name="provider_model" value="${Utils.escapeHtml(model?.provider_model || '')}"
                            class="form-input" placeholder="gpt-4" required>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Configuration (optional)</h3>
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm text-gray-600 mb-1">Temperature</label>
                                <input type="number" name="config.temperature" step="0.1" min="0" max="2"
                                    value="${model?.config?.temperature ?? ''}"
                                    class="form-input" placeholder="0.7">
                            </div>
                            <div>
                                <label class="block text-sm text-gray-600 mb-1">Max Tokens</label>
                                <input type="number" name="config.max_tokens" min="1"
                                    value="${model?.config?.max_tokens ?? ''}"
                                    class="form-input" placeholder="4096">
                            </div>
                            <div>
                                <label class="block text-sm text-gray-600 mb-1">Top P</label>
                                <input type="number" name="config.top_p" step="0.1" min="0" max="1"
                                    value="${model?.config?.top_p ?? ''}"
                                    class="form-input" placeholder="1.0">
                            </div>
                        </div>
                    </div>

                    <div class="flex items-center mt-4">
                        <input type="checkbox" name="enabled" id="enabled" ${model?.enabled !== false ? 'checked' : ''}>
                        <label for="enabled" class="ml-2 text-sm text-gray-700">Enabled</label>
                    </div>

                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">${isEdit ? 'Update' : 'Create'}</button>
                    </div>
                </form>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-model-btn').on('click', () => showForm());

        $('.edit-btn').on('click', function() {
            const id = $(this).data('id');
            showForm(id);
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });
    }

    async function showForm(id = null) {
        let model = null;

        if (id) {
            $('#content').html(Utils.renderLoading());

            try {
                model = await API.getModel(id);
            } catch (error) {
                Utils.showToast('Failed to load model', 'error');
                return render();
            }
        }

        $('#content').html(renderForm(model));
        bindFormEvents(id);
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#model-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                if (editId) {
                    delete formData.id;
                    await API.updateModel(editId, formData);
                    Utils.showToast('Model updated successfully', 'success');
                } else {
                    await API.createModel(formData);
                    Utils.showToast('Model created successfully', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete model "${id}"?`)) {
            return;
        }

        try {
            await API.deleteModel(id);
            Utils.showToast('Model deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
