/**
 * Workflows CRUD view with node graph visualization
 */
const Workflows = (function() {
    // Current steps being edited
    let currentSteps = [];
    let editingStepIndex = null;
    // Current workflow input schema for variable picker
    let currentInputSchema = null;

    // Filter operators for metadata filtering
    const FILTER_OPERATORS = [
        { value: 'eq', label: 'equals' },
        { value: 'ne', label: 'not equals' },
        { value: 'gt', label: '>' },
        { value: 'gte', label: '>=' },
        { value: 'lt', label: '<' },
        { value: 'lte', label: '<=' },
        { value: 'contains', label: 'contains' },
        { value: 'starts_with', label: 'starts with' },
        { value: 'ends_with', label: 'ends with' },
        { value: 'in', label: 'in list' },
        { value: 'not_in', label: 'not in list' },
        { value: 'exists', label: 'exists' },
        { value: 'not_exists', label: 'not exists' }
    ];

    // Operators that don't require a value
    const NO_VALUE_OPERATORS = ['exists', 'not_exists'];

    /**
     * Render filter operator options
     */
    function renderFilterOperatorOptions(selected = 'eq') {
        return FILTER_OPERATORS.map(op =>
            `<option value="${op.value}" ${op.value === selected ? 'selected' : ''}>${Utils.escapeHtml(op.label)}</option>`
        ).join('');
    }

    /**
     * Render a single filter row
     */
    function renderFilterRow(index, condition = {}) {
        const key = condition.key || '';
        const operator = condition.operator || 'eq';
        const value = condition.value !== undefined ? String(condition.value) : '';
        const hideValue = NO_VALUE_OPERATORS.includes(operator);

        return `
            <div class="filter-row flex items-center gap-2" data-index="${index}">
                <input type="text" class="filter-key form-input text-sm w-28"
                    placeholder="field" value="${Utils.escapeHtml(key)}">
                <select class="filter-operator form-input text-sm w-28">
                    ${renderFilterOperatorOptions(operator)}
                </select>
                <input type="text" class="filter-value form-input text-sm flex-1"
                    placeholder="value" value="${Utils.escapeHtml(value)}"
                    ${hideValue ? 'style="display:none"' : ''}>
                <button type="button" class="remove-filter-row text-red-500 hover:text-red-700 p-1" title="Remove filter">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>
        `;
    }

    /**
     * Render the filter builder container
     */
    function renderFilterBuilder(filter) {
        const connector = filter?.connector || 'and';
        const filters = filter?.filters || [];

        let rowsHtml = '';

        if (filters.length > 0) {
            filters.forEach((f, i) => {
                // Only handle simple conditions for now (not nested groups)
                if (f.key !== undefined) {
                    rowsHtml += renderFilterRow(i, f);
                }
            });
        }

        return `
            <div class="flex items-center gap-2 mb-2">
                <span class="text-xs text-gray-500">Match</span>
                <select class="filter-connector form-input text-sm w-20">
                    <option value="and" ${connector === 'and' ? 'selected' : ''}>ALL</option>
                    <option value="or" ${connector === 'or' ? 'selected' : ''}>ANY</option>
                </select>
                <span class="text-xs text-gray-500">of the following:</span>
            </div>
            <div id="filter-rows-container" class="space-y-2">
                ${rowsHtml || '<p class="text-xs text-gray-400 italic">No filters. Click "+ Add Filter" to add one.</p>'}
            </div>
        `;
    }

    /**
     * Parse filter value with type detection
     */
    function parseFilterValue(valueStr, operator) {
        if (NO_VALUE_OPERATORS.includes(operator)) {
            return undefined;
        }

        const trimmed = valueStr.trim();

        // Handle 'in' and 'not_in' operators - expect comma-separated list
        if (operator === 'in' || operator === 'not_in') {
            // Try JSON array first
            if (trimmed.startsWith('[')) {
                try {
                    return JSON.parse(trimmed);
                } catch (e) {
                    // Fall through to comma-separated
                }
            }
            // Split by comma
            return trimmed.split(',').map(v => parseSimpleValue(v.trim()));
        }

        return parseSimpleValue(trimmed);
    }

    /**
     * Parse a simple value with type detection
     */
    function parseSimpleValue(valueStr) {
        if (valueStr === 'true') return true;
        if (valueStr === 'false') return false;
        if (valueStr === 'null') return null;

        // Check if it's a number
        if (valueStr !== '' && !isNaN(valueStr)) {
            return Number(valueStr);
        }

        return valueStr;
    }

    /**
     * Build filter JSON from UI state
     */
    function buildFilterFromUI() {
        const connector = $('.filter-connector').val() || 'and';
        const filters = [];

        $('.filter-row').each(function() {
            const key = $(this).find('.filter-key').val().trim();
            const operator = $(this).find('.filter-operator').val();
            const valueStr = $(this).find('.filter-value').val();

            if (key) {
                const condition = { key, operator };
                const value = parseFilterValue(valueStr, operator);

                if (value !== undefined) {
                    condition.value = value;
                }

                filters.push(condition);
            }
        });

        if (filters.length === 0) {
            return null;
        }

        return { connector, filters };
    }

    /**
     * Add a new filter row
     */
    function addFilterRow() {
        const container = $('#filter-rows-container');

        // Remove "no filters" message if present
        container.find('p.italic').remove();

        const index = container.find('.filter-row').length;
        container.append(renderFilterRow(index));
        bindFilterRowEvents();
    }

    /**
     * Bind events for filter rows
     */
    function bindFilterRowEvents() {
        // Remove row
        $('.remove-filter-row').off('click').on('click', function() {
            $(this).closest('.filter-row').remove();

            // Show "no filters" message if empty
            if ($('.filter-row').length === 0) {
                $('#filter-rows-container').html('<p class="text-xs text-gray-400 italic">No filters. Click "+ Add Filter" to add one.</p>');
            }
        });

        // Toggle value field visibility based on operator
        $('.filter-operator').off('change').on('change', function() {
            const operator = $(this).val();
            const $valueInput = $(this).closest('.filter-row').find('.filter-value');

            if (NO_VALUE_OPERATORS.includes(operator)) {
                $valueInput.hide().val('');
            } else {
                $valueInput.show();
            }
        });
    }

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
            details = `
                <div class="text-xs mt-1 opacity-75">Model: ${Utils.escapeHtml(step.model_id || 'N/A')}</div>
                <div class="text-xs opacity-75">Prompt: ${Utils.escapeHtml(step.prompt_id || 'N/A')}</div>
            `;
        } else if (step.type === 'knowledge_base_search') {
            const filterCount = step.filter?.filters?.length || 0;
            const filterInfo = filterCount > 0 ? ` (${filterCount} filter${filterCount > 1 ? 's' : ''})` : '';
            details = `<div class="text-xs mt-1 opacity-75">KB: ${Utils.escapeHtml(step.knowledge_base_id || 'N/A')}${filterInfo}</div>`;
        } else if (step.type === 'crag_scoring') {
            details = `
                <div class="text-xs mt-1 opacity-75">Model: ${Utils.escapeHtml(step.model_id || 'N/A')}</div>
                <div class="text-xs opacity-75">Prompt: ${Utils.escapeHtml(step.prompt_id || 'N/A')}</div>
                <div class="text-xs opacity-75">Threshold: ${step.threshold ?? 'N/A'}</div>
            `;
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
        currentInputSchema = workflow?.input_schema || null;
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

    /**
     * Get available variables for a step based on input schema and previous steps
     */
    function getAvailableVariables() {
        const variables = {
            request: [],
            steps: []
        };

        // Add request variables from input schema
        if (currentInputSchema && currentInputSchema.properties) {
            for (const [name, schema] of Object.entries(currentInputSchema.properties)) {
                variables.request.push({
                    name: name,
                    syntax: `\${request:${name}}`,
                    type: schema.type || 'string',
                    description: schema.description || ''
                });
            }
        }

        // Add outputs from previous steps
        const maxIndex = editingStepIndex !== null ? editingStepIndex : currentSteps.length;

        for (let i = 0; i < maxIndex; i++) {
            const prevStep = currentSteps[i];
            const stepVars = getStepOutputVariables(prevStep);
            variables.steps.push({
                stepName: prevStep.name,
                stepType: prevStep.type,
                outputs: stepVars
            });
        }

        return variables;
    }

    /**
     * Get output variables for a step based on its type
     */
    function getStepOutputVariables(step) {
        const outputs = [];

        if (step.type === 'chat_completion') {
            outputs.push(
                { name: 'content', syntax: `\${step:${step.name}:content}`, description: 'LLM response text' },
                { name: 'model', syntax: `\${step:${step.name}:model}`, description: 'Model used' },
                { name: 'finish_reason', syntax: `\${step:${step.name}:finish_reason}`, description: 'Completion finish reason' }
            );
        } else if (step.type === 'knowledge_base_search') {
            outputs.push(
                { name: 'documents', syntax: `\${step:${step.name}:documents}`, description: 'Array of matching documents' },
                { name: 'documents_xml', syntax: `\${step:${step.name}:documents_xml}`, description: 'Documents as XML string' },
                { name: 'total', syntax: `\${step:${step.name}:total}`, description: 'Total number of results' }
            );
        } else if (step.type === 'crag_scoring') {
            outputs.push(
                { name: 'scored_documents', syntax: `\${step:${step.name}:scored_documents}`, description: 'Documents with relevance scores' },
                { name: 'relevant_count', syntax: `\${step:${step.name}:relevant_count}`, description: 'Number of relevant documents' }
            );
        } else if (step.type === 'http_request') {
            outputs.push(
                { name: 'body', syntax: `\${step:${step.name}:body}`, description: 'Response body' },
                { name: 'extracted', syntax: `\${step:${step.name}:extracted}`, description: 'Extracted data (if extract_path set)' },
                { name: 'status_code', syntax: `\${step:${step.name}:status_code}`, description: 'HTTP status code' }
            );
        } else if (step.type === 'conditional') {
            outputs.push(
                { name: 'action', syntax: `\${step:${step.name}:action}`, description: 'Action taken (continue/skip/end)' }
            );
        }

        return outputs;
    }

    /**
     * Render variable picker button with dropdown
     */
    function renderVariablePicker(targetFieldId) {
        return `
            <button type="button" class="variable-picker-btn ml-2 px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 rounded border"
                data-target="${targetFieldId}" title="Insert variable">
                <span class="font-mono">\${...}</span>
            </button>
        `;
    }

    /**
     * Render variable picker dropdown content
     */
    function renderVariablePickerDropdown(targetFieldId) {
        const variables = getAvailableVariables();
        let html = `
            <div class="variable-picker-dropdown absolute z-50 mt-1 w-80 bg-white border border-gray-300 rounded-lg shadow-lg max-h-64 overflow-y-auto"
                 data-target="${targetFieldId}">
                <div class="p-2 border-b bg-gray-50">
                    <span class="text-xs font-medium text-gray-600">Insert Variable</span>
                </div>
        `;

        // Request variables
        if (variables.request.length > 0) {
            html += `
                <div class="p-2 border-b">
                    <div class="text-xs font-semibold text-blue-600 mb-2">📥 Request Input</div>
                    <div class="space-y-1">
            `;

            for (const v of variables.request) {
                html += `
                    <button type="button" class="var-insert-btn w-full text-left px-2 py-1 text-xs hover:bg-blue-50 rounded flex items-center justify-between group"
                        data-syntax="${Utils.escapeHtml(v.syntax)}" data-target="${targetFieldId}">
                        <span class="font-mono text-blue-700">${Utils.escapeHtml(v.syntax)}</span>
                        <span class="text-gray-400 text-xs hidden group-hover:inline">${Utils.escapeHtml(v.type)}</span>
                    </button>
                `;
            }
            html += `</div></div>`;
        }

        // Step output variables
        if (variables.steps.length > 0) {
            html += `<div class="p-2">
                <div class="text-xs font-semibold text-green-600 mb-2">📤 Step Outputs</div>
            `;

            for (const stepGroup of variables.steps) {
                html += `
                    <div class="mb-2">
                        <div class="text-xs text-gray-500 mb-1">
                            <span class="font-medium">${Utils.escapeHtml(stepGroup.stepName)}</span>
                            <span class="text-gray-400">(${getStepTypeLabel(stepGroup.stepType)})</span>
                        </div>
                        <div class="space-y-1 pl-2 border-l-2 border-green-200">
                `;

                for (const output of stepGroup.outputs) {
                    html += `
                        <button type="button" class="var-insert-btn w-full text-left px-2 py-1 text-xs hover:bg-green-50 rounded flex items-center justify-between group"
                            data-syntax="${Utils.escapeHtml(output.syntax)}" data-target="${targetFieldId}">
                            <span class="font-mono text-green-700">${Utils.escapeHtml(output.syntax)}</span>
                            <span class="text-gray-400 text-xs hidden group-hover:inline truncate max-w-24">${Utils.escapeHtml(output.description)}</span>
                        </button>
                    `;
                }
                html += `</div></div>`;
            }
            html += `</div>`;
        }

        // No variables available
        if (variables.request.length === 0 && variables.steps.length === 0) {
            html += `
                <div class="p-4 text-center text-gray-500 text-xs">
                    <p>No variables available yet.</p>
                    <p class="mt-1">Define an input schema or add steps before this one.</p>
                </div>
            `;
        }

        html += `</div>`;
        return html;
    }

    /**
     * Insert variable at cursor position or append to field
     */
    function insertVariableIntoField(targetId, variableSyntax) {
        const $field = $(`#${targetId}, [name="${targetId}"]`);

        if ($field.length === 0) return;

        const field = $field[0];
        const currentValue = $field.val();

        // For textareas and inputs, try to insert at cursor position
        if (field.selectionStart !== undefined) {
            const start = field.selectionStart;
            const end = field.selectionEnd;
            const newValue = currentValue.substring(0, start) + variableSyntax + currentValue.substring(end);
            $field.val(newValue);

            // Move cursor to end of inserted text
            const newCursorPos = start + variableSyntax.length;
            field.setSelectionRange(newCursorPos, newCursorPos);
            $field.focus();
        } else {
            // Fallback: append to end
            $field.val(currentValue + variableSyntax);
        }

        // Trigger change event for any listeners
        $field.trigger('input').trigger('change');
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
            // Build existing variables JSON for display
            const existingVarsJson = step?.prompt_variables ? JSON.stringify(step.prompt_variables, null, 2) : '{}';

            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Model *</label>
                    <select name="model_id" class="form-input model-select" required>
                        <option value="">Select model...</option>
                    </select>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Prompt *</label>
                    <select name="prompt_id" class="form-input prompt-select" required>
                        <option value="">Select prompt...</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">Select a prompt template. Variables will be shown below.</p>
                </div>
                <div id="prompt-variables-section" class="mb-4 hidden">
                    <div class="flex items-center justify-between mb-2">
                        <label class="text-sm font-medium text-gray-700">Prompt Variables</label>
                        ${renderVariablePicker('prompt_var_target')}
                    </div>
                    <div id="prompt-variables-container" class="space-y-3 p-3 bg-gray-50 rounded border">
                        <!-- Variable inputs will be dynamically inserted here -->
                    </div>
                    <p class="text-xs text-gray-500 mt-1">Click a variable input, then use the picker above to insert variables</p>
                </div>
                <div class="mb-4">
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">User Message *</label>
                        ${renderVariablePicker('user_message')}
                    </div>
                    <textarea name="user_message" rows="3" class="form-input" required
                        placeholder='\${request:question}'>${Utils.escapeHtml(step?.user_message || '')}</textarea>
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
                <input type="hidden" name="prompt_variables_json" value="${Utils.escapeHtml(existingVarsJson)}">
            `;
        } else if (stepType === 'knowledge_base_search') {
            const existingFilter = step?.filter || null;

            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Knowledge Base *</label>
                    <select name="knowledge_base_id" class="form-input knowledge-base-select" required>
                        <option value="">Select knowledge base...</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">Select the knowledge base to search</p>
                </div>
                <div class="mb-4">
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">Query *</label>
                        ${renderVariablePicker('query')}
                    </div>
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
                <div class="mb-4">
                    <div class="flex items-center justify-between mb-2">
                        <label class="text-sm font-medium text-gray-700">Metadata Filters</label>
                        <button type="button" class="add-filter-row-btn text-xs text-blue-600 hover:text-blue-800">+ Add Filter</button>
                    </div>
                    <div id="filter-builder-container" class="p-3 bg-gray-50 rounded border">
                        ${renderFilterBuilder(existingFilter)}
                    </div>
                    <p class="text-xs text-gray-500 mt-1">Filter documents by metadata fields. For "in list", use comma-separated values.</p>
                </div>
            `;
        } else if (stepType === 'crag_scoring') {
            // Build existing variables JSON for display
            const existingCragVarsJson = step?.prompt_variables ? JSON.stringify(step.prompt_variables, null, 2) : '{}';

            fieldsHtml += `
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Model *</label>
                    <select name="model_id" class="form-input crag-model-select" required>
                        <option value="">Select model...</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">Model used for scoring document relevance</p>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Prompt *</label>
                    <select name="prompt_id" class="form-input crag-prompt-select" required>
                        <option value="">Select prompt...</option>
                    </select>
                    <p class="text-xs text-gray-500 mt-1">Prompt template for relevance scoring</p>
                </div>
                <div id="crag-prompt-variables-section" class="mb-4 hidden">
                    <div class="flex items-center justify-between mb-2">
                        <label class="text-sm font-medium text-gray-700">Prompt Variables</label>
                        ${renderVariablePicker('crag_prompt_var_target')}
                    </div>
                    <div id="crag-prompt-variables-container" class="space-y-3 p-3 bg-gray-50 rounded border">
                        <!-- Variable inputs will be dynamically inserted here -->
                    </div>
                    <p class="text-xs text-gray-500 mt-1">Click a variable input, then use the picker above to insert variables</p>
                </div>
                <div class="mb-4">
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">Documents Source *</label>
                        ${renderVariablePicker('documents_source')}
                    </div>
                    <input type="text" name="documents_source" value="${Utils.escapeHtml(step?.documents_source || '')}"
                        class="form-input" placeholder='\${step:search:documents}' required>
                    <p class="text-xs text-gray-500 mt-1">Reference to documents from a previous KB Search step</p>
                </div>
                <div class="mb-4">
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">Query *</label>
                        ${renderVariablePicker('crag_query')}
                    </div>
                    <textarea name="query" id="crag_query" rows="2" class="form-input" required
                        placeholder='\${request:question}'>${Utils.escapeHtml(step?.query || '')}</textarea>
                </div>
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-700 mb-1">Threshold</label>
                    <input type="number" name="threshold" step="0.01" min="0" max="1"
                        value="${step?.threshold ?? 0.5}" class="form-input">
                    <p class="text-xs text-gray-500 mt-1">Minimum relevance score (0-1) for a document to be considered relevant</p>
                </div>
                <input type="hidden" name="crag_prompt_variables_json" value="${Utils.escapeHtml(existingCragVarsJson)}">
            `;
        } else if (stepType === 'conditional') {
            const conditionsJson = step?.conditions ? JSON.stringify(step.conditions, null, 2) : '[]';
            fieldsHtml += `
                <div class="mb-4">
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">Conditions (JSON) *</label>
                        ${renderVariablePicker('conditions')}
                    </div>
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
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">URI Path</label>
                        ${renderVariablePicker('path')}
                    </div>
                    <input type="text" name="path" value="${Utils.escapeHtml(step?.path || '')}"
                        class="form-input" placeholder="/api/users/\${request:user_id}">
                    <p class="text-xs text-gray-500 mt-1">Path to append to the External API's base URL</p>
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
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">Additional Headers (JSON)</label>
                        ${renderVariablePicker('headers')}
                    </div>
                    <textarea name="headers" rows="3" class="form-input font-mono text-sm"
                        placeholder='{"X-Custom-Header": "\${request:custom}"}'>${Utils.escapeHtml(headersJson)}</textarea>
                    <p class="text-xs text-gray-500 mt-1">Extra headers beyond those in the credential</p>
                </div>
                <div class="mb-4">
                    <div class="flex items-center justify-between mb-1">
                        <label class="text-sm font-medium text-gray-700">Body (JSON)</label>
                        ${renderVariablePicker('http_body')}
                    </div>
                    <textarea name="body" id="http_body" rows="4" class="form-input font-mono text-sm"
                        placeholder='{"query": "\${request:query}"}'>${Utils.escapeHtml(bodyJson)}</textarea>
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
                    <option value="fail_workflow" ${!step?.on_error || step?.on_error === 'fail_workflow' ? 'selected' : ''}>Fail Workflow</option>
                    <option value="skip_step" ${step?.on_error === 'skip_step' ? 'selected' : ''}>Skip Step</option>
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

        // Track input schema changes for variable picker
        $('[name="input_schema"]').on('change blur', function() {
            const schemaStr = $(this).val().trim();

            if (schemaStr) {
                try {
                    currentInputSchema = JSON.parse(schemaStr);
                } catch (e) {
                    // Invalid JSON, keep previous schema
                }
            } else {
                currentInputSchema = null;
            }
        });

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

        // Load Models and Prompts for Chat Completion steps
        if (stepType === 'chat_completion') {
            await Promise.all([
                loadModels(step?.model_id),
                loadPrompts(step?.prompt_id, step?.prompt_variables)
            ]);
        }

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

        // Load Models and Prompts for CRAG Scoring steps
        if (stepType === 'crag_scoring') {
            await Promise.all([
                loadCragModels(step?.model_id),
                loadCragPrompts(step?.prompt_id, step?.prompt_variables)
            ]);
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

    async function loadModels(selectedId = null) {
        const $select = $('.model-select');
        $select.html('<option value="">Loading models...</option>');

        try {
            const data = await API.listModels();
            const models = (data.models || []).filter(m => m.enabled !== false);

            let options = '<option value="">Select model...</option>';

            for (const model of models) {
                const selected = model.id === selectedId ? 'selected' : '';
                const provider = model.config?.provider || 'unknown';
                options += `<option value="${Utils.escapeHtml(model.id)}" ${selected}>${Utils.escapeHtml(model.name)} (${Utils.escapeHtml(provider)})</option>`;
            }

            if (models.length === 0) {
                options = '<option value="">No models found</option>';
            }

            $select.html(options);
        } catch (error) {
            $select.html('<option value="">Failed to load models</option>');
            console.error('Failed to load models:', error);
        }
    }

    // Store prompts data for variable extraction
    let cachedPrompts = [];

    async function loadPrompts(selectedId = null, existingVariables = null) {
        const $select = $('.prompt-select');
        $select.html('<option value="">Loading prompts...</option>');

        try {
            const data = await API.listPrompts();
            cachedPrompts = (data.prompts || []).filter(p => p.enabled !== false);

            let options = '<option value="">Select prompt...</option>';

            for (const prompt of cachedPrompts) {
                const selected = prompt.id === selectedId ? 'selected' : '';
                options += `<option value="${Utils.escapeHtml(prompt.id)}" ${selected}>${Utils.escapeHtml(prompt.name)}</option>`;
            }

            if (cachedPrompts.length === 0) {
                options = '<option value="">No prompts found</option>';
            }

            $select.html(options);

            // If a prompt was pre-selected, load its variables
            if (selectedId) {
                await handlePromptSelection(selectedId, existingVariables);
            }
        } catch (error) {
            $select.html('<option value="">Failed to load prompts</option>');
            console.error('Failed to load prompts:', error);
        }
    }

    async function loadCragModels(selectedId = null) {
        const $select = $('.crag-model-select');
        $select.html('<option value="">Loading models...</option>');

        try {
            const data = await API.listModels();
            const models = (data.models || []).filter(m => m.enabled !== false);

            let options = '<option value="">Select model...</option>';

            for (const model of models) {
                const selected = model.id === selectedId ? 'selected' : '';
                const provider = model.config?.provider || 'unknown';
                options += `<option value="${Utils.escapeHtml(model.id)}" ${selected}>${Utils.escapeHtml(model.name)} (${Utils.escapeHtml(provider)})</option>`;
            }

            if (models.length === 0) {
                options = '<option value="">No models found</option>';
            }

            $select.html(options);
        } catch (error) {
            $select.html('<option value="">Failed to load models</option>');
            console.error('Failed to load models:', error);
        }
    }

    async function loadCragPrompts(selectedId = null, existingVariables = null) {
        const $select = $('.crag-prompt-select');
        $select.html('<option value="">Loading prompts...</option>');

        try {
            const data = await API.listPrompts();
            cachedPrompts = (data.prompts || []).filter(p => p.enabled !== false);

            let options = '<option value="">Select prompt...</option>';

            for (const prompt of cachedPrompts) {
                const selected = prompt.id === selectedId ? 'selected' : '';
                options += `<option value="${Utils.escapeHtml(prompt.id)}" ${selected}>${Utils.escapeHtml(prompt.name)}</option>`;
            }

            if (cachedPrompts.length === 0) {
                options = '<option value="">No prompts found</option>';
            }

            $select.html(options);

            // If a prompt was pre-selected, load its variables
            if (selectedId) {
                await handleCragPromptSelection(selectedId, existingVariables);
            }
        } catch (error) {
            $select.html('<option value="">Failed to load prompts</option>');
            console.error('Failed to load prompts:', error);
        }
    }

    async function handleCragPromptSelection(promptId, existingVariables = null) {
        const $section = $('#crag-prompt-variables-section');
        const $container = $('#crag-prompt-variables-container');

        if (!promptId) {
            $section.addClass('hidden');
            $container.html('');
            return;
        }

        // Try to find prompt in cache, otherwise fetch it
        let prompt = cachedPrompts.find(p => p.id === promptId);

        if (!prompt) {
            try {
                prompt = await API.getPrompt(promptId);
            } catch (error) {
                console.error('Failed to load prompt:', error);
                $section.addClass('hidden');
                return;
            }
        }

        const variables = extractPromptVariables(prompt.content || '');

        if (variables.length === 0) {
            $section.addClass('hidden');
            $container.html('<p class="text-sm text-gray-500 italic">This prompt has no variables.</p>');
            return;
        }

        // Parse existing variables if provided
        let existingVarsMap = {};

        if (existingVariables && typeof existingVariables === 'object') {
            existingVarsMap = existingVariables;
        }

        // Render variable inputs
        let html = '';

        for (const variable of variables) {
            const existingValue = existingVarsMap[variable.name] || variable.defaultValue || '';
            html += `
                <div class="flex items-center gap-2">
                    <label class="w-32 text-sm font-medium text-gray-600 shrink-0">\${var:${Utils.escapeHtml(variable.name)}}</label>
                    <input type="text" name="crag_prompt_var_${Utils.escapeHtml(variable.name)}"
                        value="${Utils.escapeHtml(existingValue)}"
                        class="form-input flex-1 text-sm"
                        placeholder="\${request:field} or \${step:name:field}">
                </div>
            `;
        }

        $container.html(html);
        $section.removeClass('hidden');

        // Update hidden field with variable names for form processing
        updateCragPromptVariablesJson();
    }

    function updateCragPromptVariablesJson() {
        const variables = {};
        $('[name^="crag_prompt_var_"]').each(function() {
            const name = $(this).attr('name').replace('crag_prompt_var_', '');
            const value = $(this).val().trim();

            if (value) {
                variables[name] = value;
            }
        });
        $('[name="crag_prompt_variables_json"]').val(JSON.stringify(variables));
    }

    function extractPromptVariables(content) {
        // Match ${var:variable-name} or ${var:variable-name:default-value}
        const regex = /\$\{var:([a-zA-Z_][a-zA-Z0-9_-]*?)(?::([^}]*))?\}/g;
        const variables = [];
        let match;

        while ((match = regex.exec(content)) !== null) {
            const varName = match[1];
            const defaultValue = match[2] || '';

            // Avoid duplicates
            if (!variables.find(v => v.name === varName)) {
                variables.push({ name: varName, defaultValue: defaultValue });
            }
        }

        return variables;
    }

    async function handlePromptSelection(promptId, existingVariables = null) {
        const $section = $('#prompt-variables-section');
        const $container = $('#prompt-variables-container');

        if (!promptId) {
            $section.addClass('hidden');
            $container.html('');
            return;
        }

        // Try to find prompt in cache, otherwise fetch it
        let prompt = cachedPrompts.find(p => p.id === promptId);

        if (!prompt) {
            try {
                prompt = await API.getPrompt(promptId);
            } catch (error) {
                console.error('Failed to load prompt:', error);
                $section.addClass('hidden');
                return;
            }
        }

        const variables = extractPromptVariables(prompt.content || '');

        if (variables.length === 0) {
            $section.addClass('hidden');
            $container.html('<p class="text-sm text-gray-500 italic">This prompt has no variables.</p>');
            return;
        }

        // Parse existing variables if provided
        let existingVarsMap = {};

        if (existingVariables && typeof existingVariables === 'object') {
            existingVarsMap = existingVariables;
        }

        // Render variable inputs
        let html = '';

        for (const variable of variables) {
            const existingValue = existingVarsMap[variable.name] || variable.defaultValue || '';
            html += `
                <div class="flex items-center gap-2">
                    <label class="w-32 text-sm font-medium text-gray-600 shrink-0">\${var:${Utils.escapeHtml(variable.name)}}</label>
                    <input type="text" name="prompt_var_${Utils.escapeHtml(variable.name)}"
                        value="${Utils.escapeHtml(existingValue)}"
                        class="form-input flex-1 text-sm"
                        placeholder="\${request:field} or \${step:name:field}">
                </div>
            `;
        }

        $container.html(html);
        $section.removeClass('hidden');

        // Update hidden field with variable names for form processing
        updatePromptVariablesJson();
    }

    function updatePromptVariablesJson() {
        const variables = {};
        $('[name^="prompt_var_"]').each(function() {
            const name = $(this).attr('name').replace('prompt_var_', '');
            const value = $(this).val().trim();

            if (value) {
                variables[name] = value;
            }
        });
        $('[name="prompt_variables_json"]').val(JSON.stringify(variables));
    }

    function hideStepModal() {
        $('#step-modal').addClass('hidden');
        editingStepIndex = null;
        lastFocusedPromptVar = null;
        // Clean up event listeners
        $(document).off('click.varPicker');
        $(document).off('focus', '[name^="prompt_var_"], [name^="crag_prompt_var_"]');
        $(document).off('input', '[name^="prompt_var_"]');
        $(document).off('input', '[name^="crag_prompt_var_"]');
        $('.variable-picker-dropdown').remove();
    }

    // Track currently focused field for prompt variable picker
    let lastFocusedPromptVar = null;

    function bindStepModalEvents(stepType) {
        $('#modal-cancel-btn').on('click', hideStepModal);

        $('#step-modal').on('click', function(e) {
            if (e.target === this) hideStepModal();
        });

        // Handle variable picker button clicks
        $('.variable-picker-btn').on('click', function(e) {
            e.preventDefault();
            e.stopPropagation();

            const targetId = $(this).data('target');
            const $btn = $(this);

            // Close any existing dropdowns
            $('.variable-picker-dropdown').remove();

            // For prompt_var_target or crag_prompt_var_target, use the last focused prompt variable input
            let actualTargetId = targetId;

            if ((targetId === 'prompt_var_target' || targetId === 'crag_prompt_var_target') && lastFocusedPromptVar) {
                actualTargetId = lastFocusedPromptVar;
            }

            // Create and position the dropdown
            const dropdown = renderVariablePickerDropdown(actualTargetId);
            $btn.parent().css('position', 'relative').append(dropdown);

            // Bind insert handlers
            $('.var-insert-btn').on('click', function(e) {
                e.preventDefault();
                e.stopPropagation();

                const syntax = $(this).data('syntax');
                const targetField = $(this).data('target');
                insertVariableIntoField(targetField, syntax);
                $('.variable-picker-dropdown').remove();
            });
        });

        // Close dropdown when clicking outside
        $(document).on('click.varPicker', function(e) {
            if (!$(e.target).closest('.variable-picker-dropdown, .variable-picker-btn').length) {
                $('.variable-picker-dropdown').remove();
            }
        });

        // Track focus on prompt variable inputs for the dynamic picker (both chat_completion and crag_scoring)
        $(document).on('focus', '[name^="prompt_var_"], [name^="crag_prompt_var_"]', function() {
            lastFocusedPromptVar = $(this).attr('name');
        });

        // Handle prompt selection change for chat_completion
        if (stepType === 'chat_completion') {
            $('.prompt-select').on('change', async function() {
                const promptId = $(this).val();
                await handlePromptSelection(promptId);
            });

            // Update hidden JSON field when variable inputs change
            $(document).on('input', '[name^="prompt_var_"]', function() {
                updatePromptVariablesJson();
            });
        }

        // Handle prompt selection change for crag_scoring
        if (stepType === 'crag_scoring') {
            $('.crag-prompt-select').on('change', async function() {
                const promptId = $(this).val();
                await handleCragPromptSelection(promptId);
            });

            // Update hidden JSON field when CRAG variable inputs change
            $(document).on('input', '[name^="crag_prompt_var_"]', function() {
                updateCragPromptVariablesJson();
            });
        }

        // Handle filter builder for knowledge_base_search
        if (stepType === 'knowledge_base_search') {
            // Add filter row button
            $('.add-filter-row-btn').on('click', function() {
                addFilterRow();
            });

            // Bind events for existing filter rows
            bindFilterRowEvents();
        }

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
            step.prompt_id = $('[name="prompt_id"]').val().trim();
            step.user_message = $('[name="user_message"]').val();

            // Parse prompt variables from the hidden JSON field
            try {
                const varsJson = $('[name="prompt_variables_json"]').val();
                const vars = JSON.parse(varsJson || '{}');

                if (Object.keys(vars).length > 0) {
                    step.prompt_variables = vars;
                }
            } catch (e) {
                console.error('Failed to parse prompt variables:', e);
            }

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

            // Build metadata filter from UI
            const filter = buildFilterFromUI();

            if (filter) {
                step.filter = filter;
            }
        } else if (stepType === 'crag_scoring') {
            step.model_id = $('[name="model_id"]').val().trim();
            step.prompt_id = $('[name="prompt_id"]').val().trim();
            step.documents_source = $('[name="documents_source"]').val().trim();
            step.query = $('[name="query"]').val();

            // Parse prompt variables from the hidden JSON field
            try {
                const varsJson = $('[name="crag_prompt_variables_json"]').val();
                const vars = JSON.parse(varsJson || '{}');

                if (Object.keys(vars).length > 0) {
                    step.prompt_variables = vars;
                }
            } catch (e) {
                console.error('Failed to parse CRAG prompt variables:', e);
            }

            const threshold = parseFloat($('[name="threshold"]').val());

            if (!isNaN(threshold)) step.threshold = threshold;
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
