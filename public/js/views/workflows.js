/**
 * Workflows CRUD view with node graph visualization
 */
const Workflows = (function() {
    // Current steps being edited
    let currentSteps = [];
    let editingStepIndex = null;

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
                    <button class="execute-btn btn-sm btn-primary mr-2" data-id="${Utils.escapeHtml(workflow.id)}" ${!workflow.enabled ? 'disabled' : ''}>Execute</button>
                    <button class="test-btn btn-sm btn-success-sm mr-2" data-id="${Utils.escapeHtml(workflow.id)}">Test</button>
                    <button class="clone-btn btn-sm bg-gray-200 text-gray-700 hover:bg-gray-300 mr-2" data-id="${Utils.escapeHtml(workflow.id)}" data-name="${Utils.escapeHtml(workflow.name)}">Clone</button>
                    <button class="edit-btn btn-sm btn-edit mr-2" data-id="${Utils.escapeHtml(workflow.id)}">Edit</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(workflow.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function getStepTypeLabel(type) {
        const labels = {
            'chat_completion': 'Chat Completion',
            'knowledge_base_search': 'KB Search',
            'crag_scoring': 'CRAG Scoring',
            'conditional': 'Conditional',
            'http_request': 'HTTP Request'
        };
        return labels[type] || type;
    }

    function generateExampleFromSchema(schema) {
        if (!schema || typeof schema !== 'object') return {};

        const example = {};
        const properties = schema.properties || {};

        for (const [name, propSchema] of Object.entries(properties)) {
            const type = propSchema.type || 'string';

            if (propSchema.example !== undefined) {
                example[name] = propSchema.example;
            } else if (propSchema.default !== undefined) {
                example[name] = propSchema.default;
            } else if (propSchema.enum && propSchema.enum.length > 0) {
                example[name] = propSchema.enum[0];
            } else if (type === 'string') {
                example[name] = propSchema.description || `example_${name}`;
            } else if (type === 'number' || type === 'integer') {
                example[name] = propSchema.minimum || 0;
            } else if (type === 'boolean') {
                example[name] = false;
            } else if (type === 'array') {
                example[name] = [];
            } else if (type === 'object') {
                example[name] = {};
            }
        }
        return example;
    }

    function generateStepMocksExample(steps) {
        const mocks = {};

        for (const step of steps) {
            if (step.type === 'chat_completion') {
                mocks[step.name] = { content: `Example response from ${step.name}` };
            } else if (step.type === 'knowledge_base_search') {
                mocks[step.name] = {
                    documents: [
                        { content: "Example document content", score: 0.95 }
                    ],
                    count: 1
                };
            } else if (step.type === 'crag_scoring') {
                mocks[step.name] = {
                    scored_documents: [
                        { content: "Relevant document", relevance_score: 0.8 }
                    ],
                    relevant_count: 1
                };
            } else if (step.type === 'conditional') {
                mocks[step.name] = { action: "continue" };
            } else if (step.type === 'http_request') {
                mocks[step.name] = {
                    status_code: 200,
                    success: true,
                    body: { data: "Example response" },
                    extracted: { data: "Example response" }
                };
            }
        }
        return mocks;
    }

    function getStepTypeColor(type) {
        const colors = {
            'chat_completion': 'bg-blue-100 border-blue-300 text-blue-800',
            'knowledge_base_search': 'bg-green-100 border-green-300 text-green-800',
            'crag_scoring': 'bg-purple-100 border-purple-300 text-purple-800',
            'conditional': 'bg-yellow-100 border-yellow-300 text-yellow-800',
            'http_request': 'bg-orange-100 border-orange-300 text-orange-800'
        };
        return colors[type] || 'bg-gray-100 border-gray-300 text-gray-800';
    }

    function renderNodeGraph() {
        if (currentSteps.length === 0) {
            return `
                <div class="text-center text-gray-500 py-8">
                    <p>No steps defined yet.</p>
                    <p class="text-sm mt-2">Use the buttons above to add steps to your workflow.</p>
                </div>
            `;
        }

        return `
            <div class="workflow-graph">
                <div class="flex items-center mb-4">
                    <div class="node-start">
                        <span class="text-xs font-medium">START</span>
                    </div>
                    <div class="node-connector"></div>
                </div>
                ${currentSteps.map((step, idx) => `
                    <div class="flex items-center mb-4" data-step-index="${idx}">
                        <div class="node-step ${getStepTypeColor(step.type)} cursor-pointer hover:shadow-md transition-shadow"
                             onclick="Workflows.editStep(${idx})">
                            <div class="flex justify-between items-start">
                                <div>
                                    <div class="font-medium text-sm">${Utils.escapeHtml(step.name)}</div>
                                    <div class="text-xs opacity-75">${getStepTypeLabel(step.type)}</div>
                                </div>
                                <div class="flex gap-1 ml-2">
                                    <button class="step-edit-btn text-gray-500 hover:text-gray-700 p-1" onclick="event.stopPropagation(); Workflows.editStep(${idx})">
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path></svg>
                                    </button>
                                    <button class="step-delete-btn text-red-500 hover:text-red-700 p-1" onclick="event.stopPropagation(); Workflows.deleteStep(${idx})">
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                                    </button>
                                </div>
                            </div>
                            ${renderStepDetails(step)}
                        </div>
                        ${idx < currentSteps.length - 1 ? '<div class="node-connector"></div>' : ''}
                    </div>
                `).join('')}
                <div class="flex items-center">
                    <div class="node-connector"></div>
                    <div class="node-end">
                        <span class="text-xs font-medium">END</span>
                    </div>
                </div>
            </div>
        `;
    }

    function formatConditionField(field) {
        // Truncate and escape field references
        const maxLen = 25;
        const display = field.length > maxLen ? field.substring(0, maxLen) + '...' : field;
        return Utils.escapeHtml(display);
    }

    function formatConditionOperator(op) {
        const opMap = {
            'equals': '=',
            'not_equals': '≠',
            'contains': '∋',
            'is_empty': 'is empty',
            'is_not_empty': 'not empty',
            'greater_than': '>',
            'less_than': '<'
        };
        return opMap[op] || op;
    }

    function formatConditionAction(action) {
        if (!action) return { text: 'continue', color: 'text-green-600' };

        if (action.continue || action === 'continue') {
            return { text: 'continue', color: 'text-green-600' };
        }

        if (action.go_to_step) {
            return { text: `→ ${action.go_to_step}`, color: 'text-blue-600' };
        }

        if (action.end_workflow) {
            const msg = action.end_workflow.error || action.end_workflow.result || 'end';
            return { text: `✗ ${msg.substring(0, 12)}${msg.length > 12 ? '...' : ''}`, color: 'text-red-600' };
        }

        if (action.skip_step) {
            return { text: 'skip', color: 'text-yellow-600' };
        }

        return { text: JSON.stringify(action).substring(0, 15), color: 'text-gray-600' };
    }

    function renderConditionsList(conditions) {
        if (!conditions || conditions.length === 0) {
            return '<div class="text-xs opacity-75">No conditions</div>';
        }

        const items = conditions.slice(0, 3).map(cond => {
            const field = formatConditionField(cond.field || '');
            const op = formatConditionOperator(cond.operator || '');
            const value = cond.value !== undefined ? Utils.escapeHtml(String(cond.value).substring(0, 10)) : '';
            const action = formatConditionAction(cond.action);

            return `
                <div class="flex items-center text-xs mt-1">
                    <span class="opacity-75 mr-1">├─</span>
                    <span class="font-mono opacity-75">${field} ${op} ${value}</span>
                    <span class="mx-1 opacity-50">→</span>
                    <span class="${action.color} font-medium">${action.text}</span>
                </div>
            `;
        }).join('');

        const remaining = conditions.length - 3;

        return `
            ${items}
            ${remaining > 0 ? `<div class="text-xs opacity-50 mt-1">└─ +${remaining} more condition(s)</div>` : ''}
        `;
    }

    function renderStepDetails(step) {
        let details = '';

        if (step.type === 'chat_completion') {
            details = `<div class="text-xs mt-1 opacity-75">Model: ${Utils.escapeHtml(step.model_id || 'N/A')}</div>`;
        } else if (step.type === 'knowledge_base_search') {
            details = `<div class="text-xs mt-1 opacity-75">KB: ${Utils.escapeHtml(step.knowledge_base_id || 'N/A')}</div>`;
        } else if (step.type === 'crag_scoring') {
            details = `<div class="text-xs mt-1 opacity-75">Threshold: ${step.threshold || 'N/A'}</div>`;
        } else if (step.type === 'conditional') {
            details = renderConditionsList(step.conditions);
        } else if (step.type === 'http_request') {
            const method = step.method || 'GET';
            const pathDisplay = (step.path || '/').substring(0, 25) + ((step.path || '').length > 25 ? '...' : '');
            details = `<div class="text-xs mt-1 opacity-75">${method} ${Utils.escapeHtml(pathDisplay)}</div>`;
        }

        return details;
    }

    function renderForm(workflow = null) {
        const isEdit = !!workflow;
        const title = isEdit ? 'Edit Workflow' : 'Create Workflow';
        currentSteps = workflow?.steps ? JSON.parse(JSON.stringify(workflow.steps)) : [];
        const inputSchemaJson = workflow?.input_schema ? JSON.stringify(workflow.input_schema, null, 2) : '';

        return `
            <div class="max-w-5xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="workflow-form" class="card mb-6">
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
                        <label class="block text-sm font-medium text-gray-700 mb-1">Input Schema (JSON, optional)</label>
                        <textarea name="input_schema" rows="3" class="form-input font-mono text-sm"
                            placeholder='{"type": "object", "properties": {...}}'>${Utils.escapeHtml(inputSchemaJson)}</textarea>
                    </div>

                    <div class="flex items-center mb-4">
                        <input type="checkbox" name="enabled" id="enabled" ${workflow?.enabled !== false ? 'checked' : ''}>
                        <label for="enabled" class="ml-2 text-sm text-gray-700">Enabled</label>
                    </div>
                </form>

                <!-- Steps Section -->
                <div class="card">
                    <div class="flex justify-between items-center mb-4">
                        <h3 class="font-medium">Workflow Steps</h3>
                        <div class="flex gap-2 flex-wrap">
                            <button type="button" class="add-step-btn btn-sm bg-blue-100 text-blue-700 hover:bg-blue-200" data-type="chat_completion">
                                + Chat Completion
                            </button>
                            <button type="button" class="add-step-btn btn-sm bg-green-100 text-green-700 hover:bg-green-200" data-type="knowledge_base_search">
                                + KB Search
                            </button>
                            <button type="button" class="add-step-btn btn-sm bg-purple-100 text-purple-700 hover:bg-purple-200" data-type="crag_scoring">
                                + CRAG Scoring
                            </button>
                            <button type="button" class="add-step-btn btn-sm bg-yellow-100 text-yellow-700 hover:bg-yellow-200" data-type="conditional">
                                + Conditional
                            </button>
                            <button type="button" class="add-step-btn btn-sm bg-orange-100 text-orange-700 hover:bg-orange-200" data-type="http_request">
                                + HTTP Request
                            </button>
                        </div>
                    </div>

                    <!-- Tabs -->
                    <div class="border-b mb-4">
                        <div class="flex gap-4">
                            <button type="button" class="view-tab pb-2 border-b-2 border-blue-500 text-blue-600 font-medium" data-view="graph">
                                Node Graph
                            </button>
                            <button type="button" class="view-tab pb-2 border-b-2 border-transparent text-gray-500 hover:text-gray-700" data-view="json">
                                JSON
                            </button>
                        </div>
                    </div>

                    <!-- Graph View -->
                    <div id="graph-view">
                        ${renderNodeGraph()}
                    </div>

                    <!-- JSON View (hidden by default) -->
                    <div id="json-view" class="hidden">
                        <textarea id="steps-json" rows="15" class="json-editor w-full">${Utils.escapeHtml(JSON.stringify(currentSteps, null, 2))}</textarea>
                        <button type="button" id="apply-json-btn" class="btn btn-secondary mt-2">Apply JSON Changes</button>
                    </div>
                </div>

                <div class="flex justify-end gap-3 mt-6">
                    <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                    <button type="button" id="save-btn" class="btn btn-primary">${isEdit ? 'Update' : 'Create'}</button>
                </div>
            </div>

            <!-- Step Modal -->
            <div id="step-modal" class="modal-backdrop hidden">
                <div class="modal-content max-w-lg">
                    <div id="step-modal-content"></div>
                </div>
            </div>
        `;
    }

    function renderStepModal(stepType, step = null) {
        const isEdit = !!step;
        const title = isEdit ? `Edit ${getStepTypeLabel(stepType)} Step` : `Add ${getStepTypeLabel(stepType)} Step`;

        let fieldsHtml = '';

        // Common name field
        fieldsHtml += `
            <div class="mb-4">
                <label class="block text-sm font-medium text-gray-700 mb-1">Step Name *</label>
                <input type="text" name="step_name" value="${Utils.escapeHtml(step?.name || '')}"
                    class="form-input" placeholder="my-step" required>
            </div>
        `;

        if (stepType === 'chat_completion') {
            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Model ID *</label>
                    <input type="text" name="model_id" value="${Utils.escapeHtml(step?.model_id || '')}"
                        class="form-input" placeholder="gpt-4" required>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">User Message *</label>
                    <textarea name="user_message" rows="3" class="form-input" required
                        placeholder='\${request:question}'>${Utils.escapeHtml(step?.user_message || '')}</textarea>
                    <p class="text-xs text-gray-500 mt-1">Use \${request:field} or \${step:name:field} for variables</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">System Message</label>
                    <textarea name="system_message" rows="2" class="form-input"
                        placeholder="You are a helpful assistant...">${Utils.escapeHtml(step?.system_message || '')}</textarea>
                </div>
                <div class="grid grid-cols-3 gap-4 mb-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Temperature</label>
                        <input type="number" name="temperature" step="0.1" min="0" max="2"
                            value="${step?.temperature ?? ''}" class="form-input" placeholder="0.7">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Max Tokens</label>
                        <input type="number" name="max_tokens" min="1"
                            value="${step?.max_tokens ?? ''}" class="form-input" placeholder="1000">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Top P</label>
                        <input type="number" name="top_p" step="0.1" min="0" max="1"
                            value="${step?.top_p ?? ''}" class="form-input" placeholder="1.0">
                    </div>
                </div>
            `;
        } else if (stepType === 'knowledge_base_search') {
            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Knowledge Base *</label>
                    <select name="knowledge_base_id" class="form-input knowledge-base-select" required>
                        <option value="">Select knowledge base...</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">Select the knowledge base to search</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Query *</label>
                    <textarea name="query" rows="2" class="form-input" required
                        placeholder='\${request:question}'>${Utils.escapeHtml(step?.query || '')}</textarea>
                </div>
                <div class="grid grid-cols-2 gap-4 mb-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Top K</label>
                        <input type="number" name="top_k" min="1" max="100"
                            value="${step?.top_k ?? 5}" class="form-input">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Min Score</label>
                        <input type="number" name="min_score" step="0.01" min="0" max="1"
                            value="${step?.min_score ?? ''}" class="form-input" placeholder="0.5">
                    </div>
                </div>
            `;
        } else if (stepType === 'crag_scoring') {
            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Documents Source *</label>
                    <input type="text" name="documents_source" value="${Utils.escapeHtml(step?.documents_source || '')}"
                        class="form-input" placeholder='\${step:search:documents}' required>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Query *</label>
                    <textarea name="query" rows="2" class="form-input" required
                        placeholder='\${request:question}'>${Utils.escapeHtml(step?.query || '')}</textarea>
                </div>
                <div class="grid grid-cols-2 gap-4 mb-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Threshold</label>
                        <input type="number" name="threshold" step="0.01" min="0" max="1"
                            value="${step?.threshold ?? 0.5}" class="form-input">
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Scoring Model</label>
                        <input type="text" name="scoring_model" value="${Utils.escapeHtml(step?.scoring_model || '')}"
                            class="form-input" placeholder="gpt-4">
                    </div>
                </div>
            `;
        } else if (stepType === 'conditional') {
            const conditionsJson = step?.conditions ? JSON.stringify(step.conditions, null, 2) : '[]';
            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Conditions (JSON) *</label>
                    <textarea name="conditions" rows="6" class="form-input font-mono text-sm" required
                        placeholder='[{"field": "\${step:search:documents}", "operator": "is_empty", "action": {"end_workflow": {"error": "No results"}}}]'>${Utils.escapeHtml(conditionsJson)}</textarea>
                    <p class="text-xs text-gray-500 mt-1">Operators: equals, not_equals, contains, is_empty, is_not_empty, greater_than, less_than</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Default Action</label>
                    <select name="default_action" class="form-input">
                        <option value="continue" ${step?.default_action === 'continue' ? 'selected' : ''}>Continue</option>
                        <option value="skip_step" ${step?.default_action === 'skip_step' ? 'selected' : ''}>Skip Step</option>
                    </select>
                </div>
            `;
        } else if (stepType === 'http_request') {
            const headersJson = step?.headers ? JSON.stringify(step.headers, null, 2) : '{}';
            const bodyJson = step?.body ? JSON.stringify(step.body, null, 2) : '';
            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">External API *</label>
                    <select name="external_api_id" class="form-input external-api-select" required>
                        <option value="">Select external API...</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">Provides base URL and base headers for the request</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Credential (Optional)</label>
                    <select name="credential_id" class="form-input http-credential-select">
                        <option value="">None - No authentication</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">Optional HTTP API Key credential for authentication header</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">URI Path</label>
                    <input type="text" name="path" value="${Utils.escapeHtml(step?.path || '')}"
                        class="form-input" placeholder="/api/users/\${input:user_id}">
                    <p class="text-xs text-gray-500 mt-1">Path to append to the External API's base URL. Supports variable references.</p>
                </div>
                <div class="grid grid-cols-2 gap-4 mb-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Method</label>
                        <select name="method" class="form-input">
                            <option value="GET" ${step?.method === 'GET' ? 'selected' : ''}>GET</option>
                            <option value="POST" ${step?.method === 'POST' ? 'selected' : ''}>POST</option>
                            <option value="PUT" ${step?.method === 'PUT' ? 'selected' : ''}>PUT</option>
                            <option value="DELETE" ${step?.method === 'DELETE' ? 'selected' : ''}>DELETE</option>
                            <option value="PATCH" ${step?.method === 'PATCH' ? 'selected' : ''}>PATCH</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">Timeout (ms)</label>
                        <input type="number" name="timeout_ms" min="1000" max="300000"
                            value="${step?.timeout_ms ?? 30000}" class="form-input">
                    </div>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Additional Headers (JSON)</label>
                    <textarea name="headers" rows="3" class="form-input font-mono text-sm"
                        placeholder='{"X-Custom-Header": "\${request:custom}"}'>${Utils.escapeHtml(headersJson)}</textarea>
                    <p class="text-xs text-gray-500 mt-1">Extra headers beyond those in the credential</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Body (JSON)</label>
                    <textarea name="body" rows="4" class="form-input font-mono text-sm"
                        placeholder='{"query": "\${input:query}"}'>${Utils.escapeHtml(bodyJson)}</textarea>
                    <p class="text-xs text-gray-500 mt-1">Leave empty for GET requests</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Extract Path</label>
                    <input type="text" name="extract_path" value="${Utils.escapeHtml(step?.extract_path || '')}"
                        class="form-input" placeholder="$.data.result">
                    <p class="text-xs text-gray-500 mt-1">JSON path to extract from response (optional)</p>
                </div>
                <div class="mb-4">
                    <label class="flex items-center">
                        <input type="checkbox" name="fail_on_error" ${step?.fail_on_error !== false ? 'checked' : ''}>
                        <span class="ml-2 text-sm text-gray-700">Fail on non-2xx response</span>
                    </label>
                </div>
            `;
        }

        // Common on_error field
        fieldsHtml += `
            <div class="mb-4">
                <label class="block text-sm font-medium text-gray-700 mb-1">On Error</label>
                <select name="on_error" class="form-input">
                    <option value="fail" ${step?.on_error === 'fail' ? 'selected' : ''}>Fail Workflow</option>
                    <option value="continue" ${step?.on_error === 'continue' ? 'selected' : ''}>Continue</option>
                    <option value="skip" ${step?.on_error === 'skip' ? 'selected' : ''}>Skip Step</option>
                </select>
            </div>
        `;

        return `
            <div class="p-6">
                <h3 class="text-lg font-medium mb-4">${title}</h3>
                <form id="step-form" data-type="${stepType}">
                    ${fieldsHtml}
                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="modal-cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">${isEdit ? 'Update' : 'Add'} Step</button>
                    </div>
                </form>
            </div>
        `;
    }

    function renderTestForm(workflow) {
        const inputExample = generateExampleFromSchema(workflow.input_schema);
        const inputJson = JSON.stringify(inputExample, null, 2);
        const stepMocksExample = generateStepMocksExample(workflow.steps);
        const stepsJson = JSON.stringify(stepMocksExample, null, 2);

        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Test Workflow: ${Utils.escapeHtml(workflow.name)}</h2>
                </div>

                <div class="card mb-6">
                    <div class="mb-4">
                        <span class="text-sm text-gray-600">Workflow ID:</span>
                        <span class="font-mono ml-2">${Utils.escapeHtml(workflow.id)}</span>
                    </div>

                    <div class="mb-4">
                        <h3 class="font-medium mb-2">Steps</h3>
                        <div class="space-y-1 text-sm">
                            ${workflow.steps.map((step, idx) => `
                                <div class="flex items-center text-gray-600">
                                    <span class="w-6">${idx + 1}.</span>
                                    <span class="font-mono">${Utils.escapeHtml(step.name)}</span>
                                    <span class="badge badge-gray ml-2">${Utils.escapeHtml(step.type || 'unknown')}</span>
                                </div>
                            `).join('')}
                        </div>
                    </div>

                    <form id="test-form">
                        <div class="mb-4">
                            <div class="flex justify-between items-center mb-1">
                                <label class="text-sm font-medium text-gray-700">Input JSON</label>
                                <button type="button" id="load-input-example" class="text-xs text-blue-600 hover:text-blue-800">Load Example</button>
                            </div>
                            <textarea name="input" rows="4" class="json-editor w-full"
                                placeholder='{"question": "What is...?"}'>{}</textarea>
                            <p class="text-xs text-gray-500 mt-1">Input data for the workflow (accessible via \${request:field})</p>
                        </div>

                        <div class="mb-4">
                            <div class="flex justify-between items-center mb-1">
                                <label class="text-sm font-medium text-gray-700">Step Mocks (JSON)</label>
                                <button type="button" id="load-mocks-example" class="text-xs text-blue-600 hover:text-blue-800">Load Example</button>
                            </div>
                            <textarea name="step_mocks" rows="8" class="json-editor w-full">${Utils.escapeHtml(stepsJson)}</textarea>
                            <p class="text-xs text-gray-500 mt-1">Map of step_name to mock output. Steps without mocks will be skipped.</p>
                        </div>

                        <button type="submit" class="btn btn-primary">Run Mock Test</button>
                    </form>
                </div>

                <div id="test-result" class="hidden">
                    <div class="card">
                        <h3 class="font-medium mb-4">Test Result</h3>
                        <div id="test-result-content"></div>
                    </div>
                </div>
            </div>

            <script>
                (function() {
                    const inputExample = ${inputJson};
                    const mocksExample = ${stepsJson};

                    $('#load-input-example').on('click', function() {
                        $('[name="input"]').val(JSON.stringify(inputExample, null, 2));
                    });

                    $('#load-mocks-example').on('click', function() {
                        $('[name="step_mocks"]').val(JSON.stringify(mocksExample, null, 2));
                    });
                })();
            </script>
        `;
    }

    function bindListEvents() {
        $('#create-workflow-btn').on('click', () => showForm());

        $('.execute-btn').on('click', function() {
            const id = $(this).data('id');
            showExecuteForm(id);
        });

        $('.test-btn').on('click', function() {
            const id = $(this).data('id');
            showTest(id);
        });

        $('.clone-btn').on('click', function() {
            const id = $(this).data('id');
            const name = $(this).data('name');
            showCloneModal(id, name);
        });

        $('.edit-btn').on('click', function() {
            const id = $(this).data('id');
            showForm(id);
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });
    }

    function showCloneModal(workflowId, workflowName) {
        const suggestedId = `${workflowId}-copy`;
        const suggestedName = `Copy of ${workflowName}`;

        const modalHtml = `
            <div id="clone-modal" class="modal-backdrop">
                <div class="modal-content max-w-md">
                    <div class="p-6">
                        <h3 class="text-lg font-medium mb-4">Clone Workflow</h3>
                        <p class="text-sm text-gray-600 mb-4">Create a copy of "${Utils.escapeHtml(workflowName)}"</p>

                        <form id="clone-form">
                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">New ID *</label>
                                <input type="text" name="new_id" value="${Utils.escapeHtml(suggestedId)}"
                                    class="form-input" required>
                            </div>
                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">New Name</label>
                                <input type="text" name="new_name" value="${Utils.escapeHtml(suggestedName)}"
                                    class="form-input" placeholder="Optional - defaults to 'Copy of...'">
                            </div>
                            <div class="flex justify-end gap-3 pt-4 border-t">
                                <button type="button" id="clone-cancel-btn" class="btn btn-secondary">Cancel</button>
                                <button type="submit" class="btn btn-primary">Clone</button>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
        `;

        $('body').append(modalHtml);

        $('#clone-cancel-btn').on('click', () => $('#clone-modal').remove());

        $('#clone-modal').on('click', function(e) {
            if (e.target === this) $(this).remove();
        });

        $('#clone-form').on('submit', async function(e) {
            e.preventDefault();
            const newId = $('[name="new_id"]').val().trim();
            const newName = $('[name="new_name"]').val().trim() || null;

            if (!newId) {
                Utils.showToast('New ID is required', 'error');
                return;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Cloning...');

            try {
                await API.cloneWorkflow(workflowId, { new_id: newId, new_name: newName });
                Utils.showToast('Workflow cloned successfully', 'success');
                $('#clone-modal').remove();
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
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

    async function showTest(workflowId) {
        $('#content').html(Utils.renderLoading());

        try {
            const workflow = await API.getWorkflow(workflowId);
            $('#content').html(renderTestForm(workflow));
            bindTestEvents(workflowId);
        } catch (error) {
            Utils.showToast('Failed to load workflow', 'error');
            render();
        }
    }

    function bindTestEvents(workflowId) {
        $('#back-btn').on('click', () => render());

        $('#test-form').on('submit', async function(e) {
            e.preventDefault();

            let input, stepMocks;

            try {
                input = JSON.parse($('[name="input"]').val() || '{}');
            } catch (err) {
                Utils.showToast('Invalid JSON in input field', 'error');
                return;
            }

            try {
                stepMocks = JSON.parse($('[name="step_mocks"]').val() || '{}');
            } catch (err) {
                Utils.showToast('Invalid JSON in step mocks field', 'error');
                return;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Testing...');

            try {
                const result = await API.testWorkflow(workflowId, {
                    input: input,
                    step_mocks: stepMocks
                });
                displayTestResult(result);
            } catch (error) {
                displayTestResult({
                    success: false,
                    workflow_id: workflowId,
                    input: input,
                    step_results: [],
                    error: error.message,
                    execution_time_ms: 0
                });
            } finally {
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    function displayTestResult(result) {
        const statusClass = result.success ? 'text-green-600' : 'text-red-600';
        const statusIcon = result.success ? '&#10003;' : '&#10007;';

        let html = `
            <div class="flex items-center mb-4">
                <span class="${statusClass} text-2xl mr-2">${statusIcon}</span>
                <span class="${statusClass} font-medium">${result.success ? 'Success' : 'Failed'}</span>
                <span class="text-gray-500 text-sm ml-4">${result.execution_time_ms}ms</span>
            </div>
        `;

        if (result.error) {
            html += `
                <div class="bg-red-50 text-red-700 p-3 rounded mb-4 text-sm">
                    ${Utils.escapeHtml(result.error)}
                </div>
            `;
        }

        if (result.step_results && result.step_results.length > 0) {
            html += `
                <div class="mb-4">
                    <h4 class="text-sm font-medium text-gray-700 mb-2">Step Results</h4>
                    <div class="space-y-2">
                        ${result.step_results.map(step => `
                            <div class="border rounded p-3">
                                <div class="flex items-center mb-2">
                                    <span class="font-mono font-medium">${Utils.escapeHtml(step.step_name)}</span>
                                    <span class="badge badge-gray ml-2">${Utils.escapeHtml(step.step_type)}</span>
                                    ${step.mocked ? '<span class="badge badge-success ml-2">Mocked</span>' : ''}
                                    ${step.skipped ? '<span class="badge badge-warning ml-2">Skipped</span>' : ''}
                                </div>
                                ${step.output ? `
                                    <pre class="bg-gray-50 p-2 rounded text-xs overflow-x-auto">${Utils.escapeHtml(JSON.stringify(step.output, null, 2))}</pre>
                                ` : ''}
                                ${step.reason ? `
                                    <p class="text-xs text-gray-500">${Utils.escapeHtml(step.reason)}</p>
                                ` : ''}
                            </div>
                        `).join('')}
                    </div>
                </div>
            `;
        }

        if (result.final_output) {
            html += `
                <div>
                    <h4 class="text-sm font-medium text-gray-700 mb-2">Final Output</h4>
                    <pre class="bg-gray-50 p-3 rounded text-sm overflow-x-auto">${Utils.escapeHtml(JSON.stringify(result.final_output, null, 2))}</pre>
                </div>
            `;
        }

        $('#test-result').removeClass('hidden');
        $('#test-result-content').html(html);
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        // Tab switching
        $('.view-tab').on('click', function() {
            const view = $(this).data('view');
            $('.view-tab').removeClass('border-blue-500 text-blue-600').addClass('border-transparent text-gray-500');
            $(this).removeClass('border-transparent text-gray-500').addClass('border-blue-500 text-blue-600');

            if (view === 'graph') {
                $('#graph-view').removeClass('hidden');
                $('#json-view').addClass('hidden');
            } else {
                $('#graph-view').addClass('hidden');
                $('#json-view').removeClass('hidden');
                $('#steps-json').val(JSON.stringify(currentSteps, null, 2));
            }
        });

        // Apply JSON changes
        $('#apply-json-btn').on('click', function() {
            try {
                currentSteps = JSON.parse($('#steps-json').val());
                updateGraphView();
                Utils.showToast('JSON changes applied', 'success');
            } catch (e) {
                Utils.showToast('Invalid JSON', 'error');
            }
        });

        // Add step buttons
        $('.add-step-btn').on('click', function() {
            const stepType = $(this).data('type');
            editingStepIndex = null;
            showStepModal(stepType);
        });

        // Save workflow
        $('#save-btn').on('click', async function() {
            const formData = Utils.getFormData($('#workflow-form')[0]);

            if (currentSteps.length === 0) {
                Utils.showToast('Workflow must have at least one step', 'error');
                return;
            }

            let inputSchema = null;
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
                input_schema: inputSchema,
                steps: currentSteps,
                enabled: formData.enabled
            };

            const $btn = $(this);
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

    async function showStepModal(stepType, step = null) {
        $('#step-modal-content').html(renderStepModal(stepType, step));
        $('#step-modal').removeClass('hidden');
        bindStepModalEvents(stepType);

        // Load External APIs and HTTP API Key credentials for HTTP Request steps
        if (stepType === 'http_request') {
            await Promise.all([
                loadExternalApis(step?.external_api_id),
                loadHttpCredentials(step?.credential_id)
            ]);
        }

        // Load Knowledge Bases for KB Search steps
        if (stepType === 'knowledge_base_search') {
            await loadKnowledgeBases(step?.knowledge_base_id);
        }
    }

    async function loadExternalApis(selectedId = null) {
        const $select = $('.external-api-select');
        $select.html('<option value="">Loading external APIs...</option>');

        try {
            const data = await API.listExternalApis();
            const apis = (data.external_apis || []).filter(a => a.enabled);

            let options = '<option value="">Select external API...</option>';

            for (const api of apis) {
                const selected = api.id === selectedId ? 'selected' : '';
                options += `<option value="${Utils.escapeHtml(api.id)}" ${selected}>${Utils.escapeHtml(api.name)} - ${Utils.escapeHtml(api.base_url)}</option>`;
            }

            if (apis.length === 0) {
                options = '<option value="">No external APIs found</option>';
            }

            $select.html(options);
        } catch (error) {
            $select.html('<option value="">Failed to load external APIs</option>');
            console.error('Failed to load external APIs:', error);
        }
    }

    async function loadHttpCredentials(selectedId = null) {
        const $select = $('.http-credential-select');
        $select.html('<option value="">Loading credentials...</option>');

        try {
            const data = await API.listCredentials();
            const httpCredentials = (data.credentials || []).filter(c => c.credential_type === 'http_api_key' && c.enabled);

            let options = '<option value="">None - No authentication</option>';

            for (const cred of httpCredentials) {
                const selected = cred.id === selectedId ? 'selected' : '';
                options += `<option value="${Utils.escapeHtml(cred.id)}" ${selected}>${Utils.escapeHtml(cred.name)}</option>`;
            }

            $select.html(options);
        } catch (error) {
            $select.html('<option value="">Failed to load credentials</option>');
            console.error('Failed to load credentials:', error);
        }
    }

    async function loadKnowledgeBases(selectedId = null) {
        const $select = $('.knowledge-base-select');
        $select.html('<option value="">Loading knowledge bases...</option>');

        try {
            const data = await API.listKnowledgeBases();
            const kbs = (data.knowledge_bases || []).filter(kb => kb.enabled);

            let options = '<option value="">Select knowledge base...</option>';

            for (const kb of kbs) {
                const selected = kb.id === selectedId ? 'selected' : '';
                options += `<option value="${Utils.escapeHtml(kb.id)}" ${selected}>${Utils.escapeHtml(kb.name)} (${Utils.escapeHtml(kb.kb_type)})</option>`;
            }

            if (kbs.length === 0) {
                options = '<option value="">No knowledge bases found</option>';
            }

            $select.html(options);
        } catch (error) {
            $select.html('<option value="">Failed to load knowledge bases</option>');
            console.error('Failed to load knowledge bases:', error);
        }
    }

    function hideStepModal() {
        $('#step-modal').addClass('hidden');
        editingStepIndex = null;
    }

    function bindStepModalEvents(stepType) {
        $('#modal-cancel-btn').on('click', hideStepModal);

        $('#step-modal').on('click', function(e) {
            if (e.target === this) hideStepModal();
        });

        $('#step-form').on('submit', function(e) {
            e.preventDefault();

            const step = buildStepFromForm(stepType);

            if (!step.name) {
                Utils.showToast('Step name is required', 'error');
                return;
            }

            // Check for duplicate names (excluding current editing step)
            const existingIndex = currentSteps.findIndex(s => s.name === step.name);

            if (existingIndex !== -1 && existingIndex !== editingStepIndex) {
                Utils.showToast('Step name must be unique', 'error');
                return;
            }

            if (editingStepIndex !== null) {
                currentSteps[editingStepIndex] = step;
            } else {
                currentSteps.push(step);
            }

            updateGraphView();
            hideStepModal();
        });
    }

    function buildStepFromForm(stepType) {
        const step = {
            name: $('[name="step_name"]').val().trim(),
            type: stepType,
            on_error: $('[name="on_error"]').val()
        };

        if (stepType === 'chat_completion') {
            step.model_id = $('[name="model_id"]').val().trim();
            step.user_message = $('[name="user_message"]').val();

            const systemMessage = $('[name="system_message"]').val().trim();

            if (systemMessage) step.system_message = systemMessage;

            const temp = parseFloat($('[name="temperature"]').val());

            if (!isNaN(temp)) step.temperature = temp;

            const maxTokens = parseInt($('[name="max_tokens"]').val());

            if (!isNaN(maxTokens)) step.max_tokens = maxTokens;

            const topP = parseFloat($('[name="top_p"]').val());

            if (!isNaN(topP)) step.top_p = topP;
        } else if (stepType === 'knowledge_base_search') {
            step.knowledge_base_id = $('[name="knowledge_base_id"]').val().trim();
            step.query = $('[name="query"]').val();

            const topK = parseInt($('[name="top_k"]').val());

            if (!isNaN(topK)) step.top_k = topK;

            const minScore = parseFloat($('[name="min_score"]').val());

            if (!isNaN(minScore)) step.min_score = minScore;
        } else if (stepType === 'crag_scoring') {
            step.documents_source = $('[name="documents_source"]').val().trim();
            step.query = $('[name="query"]').val();

            const threshold = parseFloat($('[name="threshold"]').val());

            if (!isNaN(threshold)) step.threshold = threshold;

            const scoringModel = $('[name="scoring_model"]').val().trim();

            if (scoringModel) step.scoring_model = scoringModel;
        } else if (stepType === 'conditional') {
            try {
                step.conditions = JSON.parse($('[name="conditions"]').val());
            } catch (e) {
                Utils.showToast('Invalid JSON in conditions', 'error');
                return step;
            }
            step.default_action = $('[name="default_action"]').val();
        } else if (stepType === 'http_request') {
            step.external_api_id = $('[name="external_api_id"]').val().trim();

            const credentialId = $('[name="credential_id"]').val().trim();

            if (credentialId) {
                step.credential_id = credentialId;
            }

            step.path = $('[name="path"]').val().trim() || '/';
            step.method = $('[name="method"]').val();

            const timeoutMs = parseInt($('[name="timeout_ms"]').val());

            if (!isNaN(timeoutMs)) step.timeout_ms = timeoutMs;

            try {
                const headersStr = $('[name="headers"]').val().trim();

                if (headersStr && headersStr !== '{}') {
                    step.headers = JSON.parse(headersStr);
                }
            } catch (e) {
                Utils.showToast('Invalid JSON in headers', 'error');
                return step;
            }

            try {
                const bodyStr = $('[name="body"]').val().trim();

                if (bodyStr) {
                    step.body = JSON.parse(bodyStr);
                }
            } catch (e) {
                Utils.showToast('Invalid JSON in body', 'error');
                return step;
            }

            const extractPath = $('[name="extract_path"]').val().trim();

            if (extractPath) step.extract_path = extractPath;

            step.fail_on_error = $('[name="fail_on_error"]').is(':checked');
        }

        return step;
    }

    function updateGraphView() {
        $('#graph-view').html(renderNodeGraph());
        $('#steps-json').val(JSON.stringify(currentSteps, null, 2));
    }

    // Public methods for inline event handlers
    function editStep(index) {
        editingStepIndex = index;
        const step = currentSteps[index];
        showStepModal(step.type, step);
    }

    function deleteStep(index) {
        if (Utils.confirm(`Delete step "${currentSteps[index].name}"?`)) {
            currentSteps.splice(index, 1);
            updateGraphView();
        }
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

    // Execute functionality
    async function showExecuteForm(workflowId) {
        $('#content').html(Utils.renderLoading());

        try {
            const workflow = await API.getWorkflow(workflowId);
            $('#content').html(renderExecuteForm(workflow));
            bindExecuteFormEvents(workflowId, workflow);
        } catch (error) {
            Utils.showToast('Failed to load workflow', 'error');
            render();
        }
    }

    function renderExecuteForm(workflow) {
        const inputSchema = workflow.input_schema || {};
        const properties = inputSchema.properties || {};
        const required = inputSchema.required || [];

        return `
            <div class="max-w-6xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Execute Workflow: ${Utils.escapeHtml(workflow.name)}</h2>
                </div>

                <div class="grid grid-cols-2 gap-6">
                    <div class="card">
                        <h3 class="font-medium mb-4">Input</h3>
                        <form id="execute-form">
                            ${Object.keys(properties).length > 0 ? `
                                <div class="mb-4">
                                    <p class="text-sm text-gray-600 mb-3">Fill in the workflow input fields:</p>
                                    ${Object.entries(properties).map(([name, schema]) => `
                                        <div class="mb-3">
                                            <label class="block text-sm font-medium text-gray-700 mb-1">
                                                ${Utils.escapeHtml(name)}
                                                ${required.includes(name) ? '<span class="text-red-500">*</span>' : ''}
                                            </label>
                                            ${renderInputField(name, schema, required.includes(name))}
                                            ${schema.description ? `<p class="text-xs text-gray-500 mt-1">${Utils.escapeHtml(schema.description)}</p>` : ''}
                                        </div>
                                    `).join('')}
                                </div>
                            ` : `
                                <div class="mb-4">
                                    <div class="flex justify-between items-center mb-2">
                                        <p class="text-sm text-gray-600">Enter input as JSON:</p>
                                        <button type="button" id="load-execute-example" class="text-xs text-blue-600 hover:text-blue-800">Load Example</button>
                                    </div>
                                    <textarea name="raw_input" class="form-input font-mono text-sm h-48" placeholder='{}'>{}</textarea>
                                    <p class="text-xs text-gray-500 mt-1">Tip: Define an input_schema on your workflow to get structured input fields</p>
                                </div>
                            `}
                            <button type="submit" class="btn btn-primary w-full">Execute Workflow</button>
                        </form>
                    </div>

                    <div class="card">
                        <h3 class="font-medium mb-4">Results</h3>
                        <div id="result-area">
                            <p class="text-gray-400 italic">Results will appear here...</p>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    function renderInputField(name, schema, isRequired) {
        const type = schema.type || 'string';
        const defaultValue = schema.default !== undefined ? schema.default : '';

        if (type === 'string' && (schema.maxLength > 200 || !schema.maxLength)) {
            return `<textarea name="input_${Utils.escapeHtml(name)}"
                class="form-input text-sm h-20"
                placeholder="${Utils.escapeHtml(schema.description || '')}"
                ${isRequired ? 'required' : ''}>${Utils.escapeHtml(String(defaultValue))}</textarea>`;
        } else if (type === 'number' || type === 'integer') {
            return `<input type="number" name="input_${Utils.escapeHtml(name)}"
                class="form-input"
                value="${Utils.escapeHtml(String(defaultValue))}"
                ${schema.minimum !== undefined ? `min="${schema.minimum}"` : ''}
                ${schema.maximum !== undefined ? `max="${schema.maximum}"` : ''}
                ${isRequired ? 'required' : ''}>`;
        } else if (type === 'boolean') {
            return `<select name="input_${Utils.escapeHtml(name)}" class="form-input">
                <option value="true" ${defaultValue === true ? 'selected' : ''}>True</option>
                <option value="false" ${defaultValue === false ? 'selected' : ''}>False</option>
            </select>`;
        } else if (schema.enum) {
            return `<select name="input_${Utils.escapeHtml(name)}" class="form-input" ${isRequired ? 'required' : ''}>
                ${schema.enum.map(v => `<option value="${Utils.escapeHtml(v)}" ${v === defaultValue ? 'selected' : ''}>${Utils.escapeHtml(v)}</option>`).join('')}
            </select>`;
        } else {
            return `<input type="text" name="input_${Utils.escapeHtml(name)}"
                class="form-input"
                value="${Utils.escapeHtml(String(defaultValue))}"
                placeholder="${Utils.escapeHtml(schema.description || '')}"
                ${isRequired ? 'required' : ''}>`;
        }
    }

    function bindExecuteFormEvents(workflowId, workflow) {
        $('#back-btn').on('click', () => render());

        // Load example for raw JSON input
        $('#load-execute-example').on('click', function() {
            const example = generateExampleFromSchema(workflow.input_schema);

            // If no schema, create a sample structure
            if (Object.keys(example).length === 0) {
                example.question = "What is the meaning of life?";
            }
            $('[name="raw_input"]').val(JSON.stringify(example, null, 2));
        });

        $('#execute-form').on('submit', async function(e) {
            e.preventDefault();

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Executing...');

            $('#result-area').html('<p class="text-gray-400 italic">Executing workflow...</p>');

            const formData = new FormData(this);
            let input = {};

            // Check if using raw JSON input or structured fields
            const rawInput = formData.get('raw_input');

            if (rawInput !== null) {
                try {
                    input = JSON.parse(rawInput);
                } catch (e) {
                    Utils.showToast('Invalid JSON input', 'error');
                    $btn.prop('disabled', false).text(originalText);
                    return;
                }
            } else {
                // Collect structured input fields
                const inputSchema = workflow.input_schema || {};
                const properties = inputSchema.properties || {};

                for (const [key, value] of formData.entries()) {
                    if (key.startsWith('input_')) {
                        const fieldName = key.substring(6);
                        const schema = properties[fieldName] || {};
                        const type = schema.type || 'string';

                        if (type === 'number' || type === 'integer') {
                            input[fieldName] = value ? Number(value) : null;
                        } else if (type === 'boolean') {
                            input[fieldName] = value === 'true';
                        } else {
                            input[fieldName] = value;
                        }
                    }
                }
            }

            try {
                const result = await API.executeWorkflow(workflowId, { input });
                $('#result-area').html(renderExecuteResult(result));
                Utils.showToast('Workflow executed successfully', 'success');
            } catch (error) {
                $('#result-area').html(`<p class="text-red-500">${Utils.escapeHtml(error.message)}</p>`);
                Utils.showToast(error.message, 'error');
            } finally {
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    function renderExecuteResult(result) {
        const statusClass = result.success ? 'badge-success' : 'badge-error';
        const statusText = result.success ? 'Success' : 'Failed';

        return `
            <div class="space-y-4">
                <div class="flex items-center justify-between">
                    <span class="badge ${statusClass}">${statusText}</span>
                    <span class="text-sm text-gray-500">${result.execution_time_ms}ms</span>
                </div>

                ${result.error ? `
                    <div class="bg-red-50 border border-red-200 rounded p-3">
                        <p class="text-red-700 text-sm">${Utils.escapeHtml(result.error)}</p>
                    </div>
                ` : ''}

                <div>
                    <h4 class="text-sm font-medium text-gray-700 mb-2">Final Output</h4>
                    <pre class="bg-gray-50 p-3 rounded text-xs overflow-auto max-h-48">${Utils.escapeHtml(JSON.stringify(result.output, null, 2))}</pre>
                </div>

                <div>
                    <h4 class="text-sm font-medium text-gray-700 mb-2">Step Results</h4>
                    <div class="space-y-2">
                        ${result.step_results.map((step, index) => `
                            <div class="border rounded p-3 ${step.success ? 'border-green-200 bg-green-50' : 'border-red-200 bg-red-50'}">
                                <div class="flex items-center justify-between mb-2">
                                    <span class="font-medium text-sm">${index + 1}. ${Utils.escapeHtml(step.step_name)}</span>
                                    <div class="flex items-center gap-2">
                                        <span class="text-xs text-gray-500">${step.execution_time_ms}ms</span>
                                        <span class="badge ${step.success ? 'badge-success' : 'badge-error'} text-xs">
                                            ${step.success ? 'OK' : 'Error'}
                                        </span>
                                    </div>
                                </div>
                                ${step.error ? `<p class="text-red-600 text-xs mb-2">${Utils.escapeHtml(step.error)}</p>` : ''}
                                ${step.output ? `
                                    <details class="text-xs">
                                        <summary class="cursor-pointer text-gray-600 hover:text-gray-800">View output</summary>
                                        <pre class="mt-2 bg-white p-2 rounded overflow-auto max-h-32">${Utils.escapeHtml(JSON.stringify(step.output, null, 2))}</pre>
                                    </details>
                                ` : ''}
                            </div>
                        `).join('')}
                    </div>
                </div>
            </div>
        `;
    }

    return { render, editStep, deleteStep };
})();
