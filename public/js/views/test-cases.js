/**
 * Test Cases CRUD view
 */
const TestCases = (function() {
    let models = [];
    let prompts = [];
    let workflows = [];

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listTestCases();
            $('#content').html(renderList(data.test_cases || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(testCases) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${testCases.length} test case(s)</p>
                <button id="create-test-case-btn" class="btn btn-primary">+ New Test Case</button>
            </div>

            ${testCases.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Type</th>
                                <th>Status</th>
                                <th>Last Result</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${testCases.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No test cases configured yet')}
        `;
    }

    function renderRow(testCase) {
        const typeBadge = getTypeBadge(testCase.test_type);
        const statusBadge = testCase.enabled
            ? { class: 'badge-success', text: 'Enabled' }
            : { class: 'badge-gray', text: 'Disabled' };
        const lastResult = testCase.last_result;
        const lastResultBadge = getResultBadge(lastResult);

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(testCase.id)}</td>
                <td>${Utils.escapeHtml(testCase.name)}</td>
                <td><span class="badge ${typeBadge.class}">${typeBadge.text}</span></td>
                <td><span class="badge ${statusBadge.class}">${statusBadge.text}</span></td>
                <td>${lastResultBadge}</td>
                <td>
                    <button class="edit-btn btn-sm btn-edit mr-1" data-id="${Utils.escapeHtml(testCase.id)}">Edit</button>
                    <button class="execute-btn btn-sm btn-success mr-1" data-id="${Utils.escapeHtml(testCase.id)}">Run</button>
                    <button class="results-btn btn-sm btn-info mr-1" data-id="${Utils.escapeHtml(testCase.id)}">Results</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(testCase.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function getTypeBadge(type) {
        switch (type) {
            case 'model_prompt': return { class: 'badge-info', text: 'Model+Prompt' };
            case 'workflow': return { class: 'badge-purple', text: 'Workflow' };
            default: return { class: 'badge-gray', text: type };
        }
    }

    function getResultBadge(result) {
        if (!result) return '<span class="text-gray-400">-</span>';

        if (result.passed) {
            return `<span class="badge badge-success">Passed</span>`;
        }

        return `<span class="badge badge-error">Failed</span>`;
    }

    function renderForm(testCase = null) {
        const isEdit = !!testCase;
        const title = isEdit ? 'Edit Test Case' : 'Create Test Case';
        const testType = testCase?.test_type || 'model_prompt';
        const input = testCase?.input || {};

        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="test-case-form" class="card">
                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                            <input type="text" name="id" value="${Utils.escapeHtml(testCase?.id || '')}"
                                class="form-input ${isEdit ? 'bg-gray-100' : ''}"
                                placeholder="my-test-case" ${isEdit ? 'readonly' : 'required'}>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                            <input type="text" name="name" value="${Utils.escapeHtml(testCase?.name || '')}"
                                class="form-input" placeholder="My Test Case" required>
                        </div>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <textarea name="description" class="form-input" rows="2"
                            placeholder="Optional description">${Utils.escapeHtml(testCase?.description || '')}</textarea>
                    </div>

                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Test Type</label>
                            <select name="test_type" class="form-input" ${isEdit ? 'disabled' : ''}>
                                <option value="model_prompt" ${testType === 'model_prompt' ? 'selected' : ''}>Model + Prompt</option>
                                <option value="workflow" ${testType === 'workflow' ? 'selected' : ''}>Workflow</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Tags</label>
                            <input type="text" name="tags" value="${Utils.escapeHtml((testCase?.tags || []).join(', '))}"
                                class="form-input" placeholder="tag1, tag2, tag3">
                        </div>
                    </div>

                    <div class="flex items-center mb-4">
                        <input type="checkbox" name="enabled" id="enabled" ${testCase?.enabled !== false ? 'checked' : ''}>
                        <label for="enabled" class="ml-2 text-sm text-gray-700">Enabled</label>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Input Configuration</h3>

                        <div id="model-prompt-input" class="${testType !== 'model_prompt' ? 'hidden' : ''}">
                            ${renderModelPromptInput(input)}
                        </div>

                        <div id="workflow-input" class="${testType !== 'workflow' ? 'hidden' : ''}">
                            ${renderWorkflowInput(input)}
                        </div>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <div class="flex justify-between items-center mb-3">
                            <h3 class="font-medium">Assertions</h3>
                            <button type="button" id="add-assertion-btn" class="btn-sm btn-primary">+ Add Assertion</button>
                        </div>
                        <div id="assertions-container">
                            ${(testCase?.assertions || []).map((a, i) => renderAssertionForm(a, i)).join('')}
                        </div>
                        <p class="text-xs text-gray-500 mt-2">Define criteria to validate the response</p>
                    </div>

                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">${isEdit ? 'Update' : 'Create'}</button>
                    </div>
                </form>
            </div>
        `;
    }

    function renderModelPromptInput(input) {
        const modelId = input?.model_id || '';
        const promptId = input?.prompt_id || '';

        return `
            <div class="grid grid-cols-2 gap-4 mb-4">
                <div>
                    <label class="block text-sm text-gray-600 mb-1">Model</label>
                    <select name="model_id" class="form-input" required>
                        <option value="">Select a model</option>
                        ${models.sort((a, b) => a.name.localeCompare(b.name)).map(m => `<option value="${Utils.escapeHtml(m.id)}" ${m.id === modelId ? 'selected' : ''}>${Utils.escapeHtml(m.name)} (${Utils.escapeHtml(m.id)})</option>`).join('')}
                    </select>
                </div>
                <div>
                    <label class="block text-sm text-gray-600 mb-1">Prompt (optional)</label>
                    <select name="prompt_id" class="form-input">
                        <option value="">None</option>
                        ${prompts.sort((a, b) => a.name.localeCompare(b.name)).map(p => `<option value="${Utils.escapeHtml(p.id)}" ${p.id === promptId ? 'selected' : ''}>${Utils.escapeHtml(p.name)} (${Utils.escapeHtml(p.id)})</option>`).join('')}
                    </select>
                </div>
            </div>

            <div class="mb-4">
                <label class="block text-sm text-gray-600 mb-1">User Message</label>
                <textarea name="user_message" class="form-input" rows="3"
                    placeholder="Enter the test message">${Utils.escapeHtml(input?.user_message || '')}</textarea>
            </div>

            <div class="grid grid-cols-2 gap-4 mb-4">
                <div>
                    <label class="block text-sm text-gray-600 mb-1">Temperature</label>
                    <input type="number" name="temperature" step="0.1" min="0" max="2"
                        class="form-input" value="${input?.temperature ?? ''}" placeholder="0.7">
                </div>
                <div>
                    <label class="block text-sm text-gray-600 mb-1">Max Tokens</label>
                    <input type="number" name="max_tokens" min="1"
                        class="form-input" value="${input?.max_tokens ?? ''}" placeholder="4096">
                </div>
            </div>

            <div class="mb-4">
                <label class="block text-sm text-gray-600 mb-1">Variables (JSON)</label>
                <textarea name="variables" class="form-input font-mono text-sm" rows="3"
                    placeholder='{"key": "value"}'>${Utils.escapeHtml(JSON.stringify(input?.variables || {}, null, 2))}</textarea>
            </div>
        `;
    }

    function renderWorkflowInput(input) {
        const workflowId = input?.workflow_id || '';

        return `
            <div class="mb-4">
                <label class="block text-sm text-gray-600 mb-1">Workflow</label>
                <select name="workflow_id" class="form-input">
                    <option value="">Select a workflow</option>
                    ${workflows.sort((a, b) => a.name.localeCompare(b.name)).map(w => `<option value="${Utils.escapeHtml(w.id)}" ${w.id === workflowId ? 'selected' : ''}>${Utils.escapeHtml(w.name)} (${Utils.escapeHtml(w.id)})</option>`).join('')}
                </select>
            </div>

            <div class="mb-4">
                <label class="block text-sm text-gray-600 mb-1">Input Variables (JSON)</label>
                <textarea name="workflow_variables" class="form-input font-mono text-sm" rows="4"
                    placeholder='{"question": "...", "knowledge_base_id": "..."}'>${Utils.escapeHtml(JSON.stringify(input?.variables || {}, null, 2))}</textarea>
            </div>
        `;
    }

    function renderAssertionForm(assertion = null, index = 0) {
        const field = assertion?.field || 'content';
        const operator = assertion?.operator || 'contains';
        const value = assertion?.value || '';

        return `
            <div class="assertion-item border rounded p-4 mb-3" data-index="${index}">
                <div class="flex justify-between items-start mb-3">
                    <h4 class="font-medium">Assertion ${index + 1}</h4>
                    <button type="button" class="remove-assertion-btn text-red-500 hover:text-red-700">&times;</button>
                </div>

                <div class="grid grid-cols-3 gap-4">
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">Field</label>
                        <select class="form-input assertion-field">
                            <option value="content" ${field === 'content' ? 'selected' : ''}>Content (Response Text)</option>
                            <option value="model" ${field === 'model' ? 'selected' : ''}>Model</option>
                            <option value="tokens" ${field === 'tokens' ? 'selected' : ''}>Token Count</option>
                            <option value="latency" ${field === 'latency' ? 'selected' : ''}>Latency (ms)</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">Operator</label>
                        <select class="form-input assertion-operator">
                            <option value="contains" ${operator === 'contains' ? 'selected' : ''}>Contains</option>
                            <option value="not_contains" ${operator === 'not_contains' ? 'selected' : ''}>Does Not Contain</option>
                            <option value="equals" ${operator === 'equals' ? 'selected' : ''}>Equals</option>
                            <option value="not_equals" ${operator === 'not_equals' ? 'selected' : ''}>Not Equals</option>
                            <option value="regex" ${operator === 'regex' ? 'selected' : ''}>Matches Regex</option>
                            <option value="json_path" ${operator === 'json_path' ? 'selected' : ''}>JSON Path</option>
                            <option value="greater_than" ${operator === 'greater_than' ? 'selected' : ''}>Greater Than</option>
                            <option value="less_than" ${operator === 'less_than' ? 'selected' : ''}>Less Than</option>
                            <option value="length_min" ${operator === 'length_min' ? 'selected' : ''}>Min Length</option>
                            <option value="length_max" ${operator === 'length_max' ? 'selected' : ''}>Max Length</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">Value</label>
                        <input type="text" class="form-input assertion-value"
                            value="${Utils.escapeHtml(String(value))}" placeholder="Expected value">
                    </div>
                </div>
            </div>
        `;
    }

    function renderResults(testCaseId, results) {
        return `
            <div class="max-w-6xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Test Results: ${Utils.escapeHtml(testCaseId)}</h2>
                </div>

                ${results.length > 0 ? `
                    <div class="card p-0 overflow-hidden">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>ID</th>
                                    <th>Result</th>
                                    <th>Assertions</th>
                                    <th>Latency</th>
                                    <th>Tokens</th>
                                    <th>Executed</th>
                                    <th>Actions</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${results.map(r => renderResultRow(r)).join('')}
                            </tbody>
                        </table>
                    </div>
                ` : Utils.renderEmpty('No test results yet. Click "Run" to execute the test.')}
            </div>
        `;
    }

    function renderResultRow(result) {
        const passedBadge = result.passed
            ? { class: 'badge-success', text: 'Passed' }
            : { class: 'badge-error', text: 'Failed' };
        const assertions = result.assertion_results || [];
        const passed = assertions.filter(a => a.passed).length;
        const failed = assertions.filter(a => !a.passed).length;
        const executedDate = new Date(result.executed_at).toLocaleString();

        return `
            <tr>
                <td class="font-mono text-xs">${Utils.escapeHtml(result.id.substring(0, 12))}...</td>
                <td><span class="badge ${passedBadge.class}">${passedBadge.text}</span></td>
                <td>
                    <span class="text-green-600">${passed} passed</span>
                    ${failed > 0 ? `<span class="text-red-600 ml-2">${failed} failed</span>` : ''}
                </td>
                <td>${result.latency_ms ? result.latency_ms.toFixed(0) + 'ms' : '-'}</td>
                <td>${result.token_usage ? result.token_usage.total_tokens : '-'}</td>
                <td class="text-sm text-gray-500">${executedDate}</td>
                <td>
                    <button class="view-result-btn btn-sm btn-info" data-result='${Utils.escapeHtml(JSON.stringify(result))}'>Details</button>
                </td>
            </tr>
        `;
    }

    function renderResultDetail(result) {
        const assertions = result.assertion_results || [];

        return `
            <div class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
                <div class="bg-white rounded-lg shadow-xl p-6 w-full max-w-4xl max-h-[90vh] overflow-auto">
                    <div class="flex justify-between items-center mb-4">
                        <h3 class="text-lg font-semibold">Result Details</h3>
                        <button id="close-detail-btn" class="text-gray-500 hover:text-gray-700 text-xl">&times;</button>
                    </div>

                    <div class="grid grid-cols-4 gap-4 mb-4">
                        <div class="card">
                            <p class="text-xs text-gray-500">Result</p>
                            <p class="text-lg font-bold ${result.passed ? 'text-green-600' : 'text-red-600'}">
                                ${result.passed ? 'PASSED' : 'FAILED'}
                            </p>
                        </div>
                        <div class="card">
                            <p class="text-xs text-gray-500">Latency</p>
                            <p class="text-lg font-bold">${result.latency_ms ? result.latency_ms.toFixed(0) + 'ms' : '-'}</p>
                        </div>
                        <div class="card">
                            <p class="text-xs text-gray-500">Input Tokens</p>
                            <p class="text-lg font-bold">${result.token_usage?.input_tokens || '-'}</p>
                        </div>
                        <div class="card">
                            <p class="text-xs text-gray-500">Output Tokens</p>
                            <p class="text-lg font-bold">${result.token_usage?.output_tokens || '-'}</p>
                        </div>
                    </div>

                    ${assertions.length > 0 ? `
                        <div class="mb-4">
                            <h4 class="font-medium mb-2">Assertion Results</h4>
                            <div class="space-y-2">
                                ${assertions.map(a => `
                                    <div class="border rounded p-3 ${a.passed ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'}">
                                        <div class="flex items-center">
                                            <span class="${a.passed ? 'text-green-600' : 'text-red-600'} mr-2">
                                                ${a.passed ? '&#10003;' : '&#10007;'}
                                            </span>
                                            <span class="font-mono text-sm">${Utils.escapeHtml(a.field)} ${Utils.escapeHtml(a.operator)} "${Utils.escapeHtml(String(a.expected))}"</span>
                                        </div>
                                        ${!a.passed && a.actual ? `
                                            <p class="text-sm text-gray-600 mt-1">Actual: ${Utils.escapeHtml(String(a.actual).substring(0, 200))}${String(a.actual).length > 200 ? '...' : ''}</p>
                                        ` : ''}
                                        ${a.message ? `<p class="text-sm text-gray-500 mt-1">${Utils.escapeHtml(a.message)}</p>` : ''}
                                    </div>
                                `).join('')}
                            </div>
                        </div>
                    ` : ''}

                    <div class="mb-4">
                        <h4 class="font-medium mb-2">Response</h4>
                        <pre class="bg-gray-100 p-4 rounded overflow-auto max-h-64 text-sm">${Utils.escapeHtml(result.output || '')}</pre>
                    </div>

                    ${result.error ? `
                        <div class="mb-4">
                            <h4 class="font-medium mb-2 text-red-600">Error</h4>
                            <pre class="bg-red-50 text-red-700 p-4 rounded overflow-auto max-h-32 text-sm">${Utils.escapeHtml(result.error)}</pre>
                        </div>
                    ` : ''}
                </div>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-test-case-btn').on('click', () => showForm());

        $('.edit-btn').on('click', function() {
            const id = $(this).data('id');
            showForm(id);
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });

        $('.execute-btn').on('click', function() {
            const id = $(this).data('id');
            executeTestCase(id);
        });

        $('.results-btn').on('click', function() {
            const id = $(this).data('id');
            showResults(id);
        });
    }

    async function showForm(id = null) {
        let testCase = null;
        $('#content').html(Utils.renderLoading());

        try {
            const [modelData, promptData, workflowData] = await Promise.all([
                API.listModels(),
                API.listPrompts(),
                API.listWorkflows()
            ]);
            models = modelData.models || [];
            prompts = promptData.prompts || [];
            workflows = workflowData.workflows || [];

            if (id) {
                testCase = await API.getTestCase(id);
            }
        } catch (error) {
            Utils.showToast('Failed to load data', 'error');
            return render();
        }

        $('#content').html(renderForm(testCase));
        bindFormEvents(id);
    }

    function bindFormEvents(editId) {
        let assertionIndex = $('.assertion-item').length;

        $('#back-btn, #cancel-btn').on('click', () => render());

        $('select[name="test_type"]').on('change', function() {
            const type = $(this).val();
            $('#model-prompt-input').toggleClass('hidden', type !== 'model_prompt');
            $('#workflow-input').toggleClass('hidden', type !== 'workflow');
        });

        $('#add-assertion-btn').on('click', function() {
            $('#assertions-container').append(renderAssertionForm(null, assertionIndex++));
            bindAssertionEvents();
        });

        bindAssertionEvents();

        $('#test-case-form').on('submit', async function(e) {
            e.preventDefault();

            const testType = $('select[name="test_type"]').val();
            const formData = {
                id: $('input[name="id"]').val(),
                name: $('input[name="name"]').val(),
                description: $('textarea[name="description"]').val() || null,
                test_type: testType,
                enabled: $('#enabled').is(':checked'),
                tags: parseTags($('input[name="tags"]').val()),
                assertions: collectAssertions()
            };

            if (testType === 'model_prompt') {
                formData.input = {
                    model_id: $('select[name="model_id"]').val(),
                    prompt_id: $('select[name="prompt_id"]').val() || null,
                    user_message: $('textarea[name="user_message"]').val(),
                    temperature: parseFloat($('input[name="temperature"]').val()) || null,
                    max_tokens: parseInt($('input[name="max_tokens"]').val()) || null,
                    variables: parseJson($('textarea[name="variables"]').val())
                };
            } else {
                formData.input = {
                    workflow_id: $('select[name="workflow_id"]').val(),
                    variables: parseJson($('textarea[name="workflow_variables"]').val())
                };
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                if (editId) {
                    delete formData.id;
                    delete formData.test_type;
                    await API.updateTestCase(editId, formData);
                    Utils.showToast('Test case updated successfully', 'success');
                } else {
                    await API.createTestCase(formData);
                    Utils.showToast('Test case created successfully', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    function bindAssertionEvents() {
        $('.remove-assertion-btn').off('click').on('click', function() {
            $(this).closest('.assertion-item').remove();
        });
    }

    function parseTags(value) {
        if (!value) return [];

        return value.split(',').map(t => t.trim()).filter(t => t);
    }

    function parseJson(value) {
        if (!value || value.trim() === '') return {};

        try {
            return JSON.parse(value);
        } catch (e) {
            return {};
        }
    }

    function collectAssertions() {
        const assertions = [];

        $('.assertion-item').each(function() {
            const $item = $(this);
            const field = $item.find('.assertion-field').val();
            const operator = $item.find('.assertion-operator').val();
            let value = $item.find('.assertion-value').val();

            if (['greater_than', 'less_than', 'length_min', 'length_max'].includes(operator)) {
                value = parseInt(value) || 0;
            }

            assertions.push({ field, operator, value });
        });

        return assertions;
    }

    async function showResults(id) {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.getTestCaseResults(id);
            $('#content').html(renderResults(id, data.results || []));
            $('#back-btn').on('click', () => render());

            $('.view-result-btn').on('click', function() {
                const result = JSON.parse($(this).data('result'));
                showResultDetail(result);
            });
        } catch (error) {
            Utils.showToast('Failed to load results', 'error');
            render();
        }
    }

    function showResultDetail(result) {
        $('body').append(renderResultDetail(result));

        $('#close-detail-btn').on('click', function() {
            $(this).closest('.fixed').remove();
        });
    }

    async function executeTestCase(id) {
        Utils.showToast('Executing test case...', 'info');

        try {
            const result = await API.executeTestCase(id);

            if (result.passed) {
                Utils.showToast('Test passed!', 'success');
            } else {
                Utils.showToast('Test failed', 'error');
            }
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete test case "${id}"?`)) {
            return;
        }

        try {
            await API.deleteTestCase(id);
            Utils.showToast('Test case deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
