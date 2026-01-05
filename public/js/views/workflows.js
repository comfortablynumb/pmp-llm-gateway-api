/**
 * Workflows CRUD view
 */
const Workflows = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listWorkflows();
            $('#content').html(renderList(data.workflows || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(workflows) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${workflows.length} workflow(s)</p>
                <button id="create-workflow-btn" class="btn btn-primary">+ New Workflow</button>
            </div>

            ${workflows.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Steps</th>
                                <th>Status</th>
                                <th>Version</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${workflows.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No workflows configured yet')}
        `;
    }

    function renderRow(workflow) {
        const statusClass = workflow.enabled ? 'badge-success' : 'badge-gray';
        const statusText = workflow.enabled ? 'Enabled' : 'Disabled';
        const stepCount = workflow.steps?.length || 0;

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(workflow.id)}</td>
                <td>
                    <div class="font-medium">${Utils.escapeHtml(workflow.name)}</div>
                    ${workflow.description ? `<div class="text-xs text-gray-500">${Utils.escapeHtml(Utils.truncate(workflow.description, 50))}</div>` : ''}
                </td>
                <td>${stepCount} step(s)</td>
                <td><span class="badge ${statusClass}">${statusText}</span></td>
                <td><span class="badge badge-gray">v${workflow.version || 1}</span></td>
                <td>
                    <button class="edit-btn text-blue-600 hover:text-blue-800 mr-3" data-id="${Utils.escapeHtml(workflow.id)}">Edit</button>
                    <button class="delete-btn text-red-600 hover:text-red-800" data-id="${Utils.escapeHtml(workflow.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function renderForm(workflow = null) {
        const isEdit = !!workflow;
        const title = isEdit ? 'Edit Workflow' : 'Create Workflow';
        const stepsJson = workflow?.steps ? JSON.stringify(workflow.steps, null, 2) : '[]';
        const inputSchemaJson = workflow?.input_schema ? JSON.stringify(workflow.input_schema, null, 2) : '';

        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="workflow-form" class="card">
                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                            <input type="text" name="id" value="${Utils.escapeHtml(workflow?.id || '')}"
                                class="form-input ${isEdit ? 'bg-gray-100' : ''}"
                                placeholder="my-workflow-id" ${isEdit ? 'readonly' : 'required'}>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                            <input type="text" name="name" value="${Utils.escapeHtml(workflow?.name || '')}"
                                class="form-input" placeholder="My Workflow" required>
                        </div>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <input type="text" name="description" value="${Utils.escapeHtml(workflow?.description || '')}"
                            class="form-input" placeholder="Optional description">
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Input Schema (JSON)</label>
                        <textarea name="input_schema" rows="4" class="form-input font-mono text-sm"
                            placeholder='{"type": "object", "properties": {...}}'>${Utils.escapeHtml(inputSchemaJson)}</textarea>
                        <p class="text-xs text-gray-500 mt-1">JSON Schema for workflow input validation (optional)</p>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Steps (JSON)</label>
                        <textarea name="steps" rows="12" class="json-editor w-full"
                            placeholder='[{"name": "step1", "type": "chat_completion", ...}]' required>${Utils.escapeHtml(stepsJson)}</textarea>
                        <p class="text-xs text-gray-500 mt-1">
                            Array of workflow steps. Each step needs: name, type (chat_completion, knowledge_base_search, crag_scoring, conditional)
                        </p>
                    </div>

                    <div class="flex items-center mt-4">
                        <input type="checkbox" name="enabled" id="enabled" ${workflow?.enabled !== false ? 'checked' : ''}>
                        <label for="enabled" class="ml-2 text-sm text-gray-700">Enabled</label>
                    </div>

                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">${isEdit ? 'Update' : 'Create'}</button>
                    </div>
                </form>

                <div class="card mt-6">
                    <h3 class="font-medium mb-4">Step Types Reference</h3>
                    <div class="space-y-4 text-sm">
                        <div>
                            <h4 class="font-medium text-gray-700">chat_completion</h4>
                            <pre class="bg-gray-50 p-2 rounded mt-1 overflow-x-auto">{
  "name": "answer",
  "type": "chat_completion",
  "model_id": "gpt-4",
  "user_message": "\${request:question}"
}</pre>
                        </div>
                        <div>
                            <h4 class="font-medium text-gray-700">knowledge_base_search</h4>
                            <pre class="bg-gray-50 p-2 rounded mt-1 overflow-x-auto">{
  "name": "search",
  "type": "knowledge_base_search",
  "knowledge_base_id": "my-kb",
  "query": "\${request:question}",
  "top_k": 5
}</pre>
                        </div>
                        <div>
                            <h4 class="font-medium text-gray-700">conditional</h4>
                            <pre class="bg-gray-50 p-2 rounded mt-1 overflow-x-auto">{
  "name": "check",
  "type": "conditional",
  "conditions": [
    {"field": "\${step:search:documents}", "operator": "is_empty", "action": {"end_workflow": {"error": "No results"}}}
  ],
  "default_action": "continue"
}</pre>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-workflow-btn').on('click', () => showForm());

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
        let workflow = null;

        if (id) {
            $('#content').html(Utils.renderLoading());

            try {
                workflow = await API.getWorkflow(id);
            } catch (error) {
                Utils.showToast('Failed to load workflow', 'error');
                return render();
            }
        }

        $('#content').html(renderForm(workflow));
        bindFormEvents(id);
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#workflow-form').on('submit', async function(e) {
            e.preventDefault();

            const formData = Utils.getFormData(this);

            // Parse JSON fields
            let steps, inputSchema;

            try {
                steps = JSON.parse($('[name="steps"]').val());
            } catch (e) {
                Utils.showToast('Invalid JSON in steps field', 'error');
                return;
            }

            const inputSchemaStr = $('[name="input_schema"]').val().trim();

            if (inputSchemaStr) {
                try {
                    inputSchema = JSON.parse(inputSchemaStr);
                } catch (e) {
                    Utils.showToast('Invalid JSON in input schema field', 'error');
                    return;
                }
            }

            const data = {
                id: formData.id,
                name: formData.name,
                description: formData.description || null,
                input_schema: inputSchema || null,
                steps: steps,
                enabled: formData.enabled
            };

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                if (editId) {
                    delete data.id;
                    await API.updateWorkflow(editId, data);
                    Utils.showToast('Workflow updated successfully', 'success');
                } else {
                    await API.createWorkflow(data);
                    Utils.showToast('Workflow created successfully', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete workflow "${id}"?`)) {
            return;
        }

        try {
            await API.deleteWorkflow(id);
            Utils.showToast('Workflow deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
