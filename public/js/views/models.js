/**
 * Models CRUD view with execution support
 */
const Models = (function() {
    let credentials = [];
    let prompts = [];
    let models = [];

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
                                <th>Credential</th>
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
                <td class="font-mono text-sm">${Utils.escapeHtml(model.credential_id || '-')}</td>
                <td class="font-mono text-sm">${Utils.escapeHtml(model.provider_model)}</td>
                <td><span class="badge ${statusClass}">${statusText}</span></td>
                <td>
                    <button class="execute-btn btn-sm btn-primary mr-2" data-id="${Utils.escapeHtml(model.id)}" ${!model.enabled ? 'disabled' : ''}>Execute</button>
                    <button class="edit-btn btn-sm btn-edit mr-2" data-id="${Utils.escapeHtml(model.id)}">Edit</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(model.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function renderCredentialOptions(selectedProvider, selectedCredentialId) {
        const filtered = credentials.filter(c => c.credential_type === selectedProvider && c.enabled);

        if (filtered.length === 0) {
            return '<option value="">No credentials available for this provider</option>';
        }

        return filtered.map(c => `
            <option value="${Utils.escapeHtml(c.id)}" ${c.id === selectedCredentialId ? 'selected' : ''}>
                ${Utils.escapeHtml(c.name)} (${Utils.escapeHtml(c.id)})
            </option>
        `).join('');
    }

    function renderFallbackModelOptions(currentModelId, selectedFallbackId) {
        // Exclude current model and disabled models
        const filtered = models.filter(m => m.id !== currentModelId && m.enabled);

        return filtered.map(m => `
            <option value="${Utils.escapeHtml(m.id)}" ${m.id === selectedFallbackId ? 'selected' : ''}>
                ${Utils.escapeHtml(m.name)} (${Utils.escapeHtml(m.id)})
            </option>
        `).join('');
    }

    function renderForm(model = null) {
        const isEdit = !!model;
        const title = isEdit ? 'Edit Model' : 'Create Model';
        const selectedProvider = model?.provider || 'openai';

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
                        <select name="provider" id="provider-select" class="form-input" ${isEdit ? 'disabled' : 'required'}>
                            <option value="openai" ${model?.provider === 'openai' ? 'selected' : ''}>OpenAI</option>
                            <option value="anthropic" ${model?.provider === 'anthropic' ? 'selected' : ''}>Anthropic</option>
                            <option value="azure_openai" ${model?.provider === 'azure_openai' ? 'selected' : ''}>Azure OpenAI</option>
                            <option value="aws_bedrock" ${model?.provider === 'aws_bedrock' ? 'selected' : ''}>AWS Bedrock</option>
                        </select>
                        ${isEdit ? `<input type="hidden" name="provider" value="${Utils.escapeHtml(model?.provider || '')}">` : ''}
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Credential</label>
                        <select name="credential_id" id="credential-select" class="form-input" required>
                            ${renderCredentialOptions(selectedProvider, model?.credential_id)}
                        </select>
                        <p class="text-xs text-gray-500 mt-1">Select a credential matching the provider</p>
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

                    <!-- Advanced Configuration (collapsible) -->
                    <div class="border-t pt-4 mt-4">
                        <button type="button" id="toggle-advanced" class="flex items-center text-sm font-medium text-gray-700 hover:text-gray-900">
                            <span id="advanced-arrow" class="mr-2 transition-transform">&rarr;</span>
                            Advanced Configuration
                        </button>
                        <div id="advanced-config" class="hidden mt-3">
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-sm text-gray-600 mb-1">Timeout (ms)</label>
                                    <input type="number" name="config.timeout_ms" min="1000" max="300000"
                                        value="${model?.config?.timeout_ms ?? ''}"
                                        class="form-input" placeholder="30000">
                                    <p class="text-xs text-gray-400 mt-1">Request timeout in milliseconds</p>
                                </div>
                                <div>
                                    <label class="block text-sm text-gray-600 mb-1">Max Retries</label>
                                    <input type="number" name="config.max_retries" min="0" max="10"
                                        value="${model?.config?.max_retries ?? ''}"
                                        class="form-input" placeholder="3">
                                    <p class="text-xs text-gray-400 mt-1">Maximum retry attempts (0-10)</p>
                                </div>
                                <div>
                                    <label class="block text-sm text-gray-600 mb-1">Retry Delay (ms)</label>
                                    <input type="number" name="config.retry_delay_ms" min="100" max="60000"
                                        value="${model?.config?.retry_delay_ms ?? ''}"
                                        class="form-input" placeholder="1000">
                                    <p class="text-xs text-gray-400 mt-1">Delay between retries</p>
                                </div>
                                <div>
                                    <label class="block text-sm text-gray-600 mb-1">Fallback Model</label>
                                    <select name="config.fallback_model_id" class="form-input">
                                        <option value="">-- No fallback --</option>
                                        ${renderFallbackModelOptions(model?.id, model?.config?.fallback_model_id)}
                                    </select>
                                    <p class="text-xs text-gray-400 mt-1">Model to use if this one fails</p>
                                </div>
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

        $('.execute-btn').on('click', function() {
            const id = $(this).data('id');
            showExecuteForm(id);
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

    async function showForm(id = null) {
        let model = null;
        $('#content').html(Utils.renderLoading());

        try {
            // Load credentials and models for fallback dropdown
            const [credData, modelsData] = await Promise.all([
                API.listCredentials(),
                API.listModels()
            ]);
            credentials = credData.credentials || [];
            models = modelsData.models || [];

            if (id) {
                model = await API.getModel(id);
            }
        } catch (error) {
            Utils.showToast('Failed to load data', 'error');
            return render();
        }

        $('#content').html(renderForm(model));
        bindFormEvents(id);
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        // Toggle advanced configuration section
        $('#toggle-advanced').on('click', function() {
            const $config = $('#advanced-config');
            const $arrow = $('#advanced-arrow');

            if ($config.hasClass('hidden')) {
                $config.removeClass('hidden');
                $arrow.css('transform', 'rotate(90deg)');
            } else {
                $config.addClass('hidden');
                $arrow.css('transform', 'rotate(0deg)');
            }
        });

        // Update credential dropdown when provider changes
        $('#provider-select').on('change', function() {
            const selectedProvider = $(this).val();
            const currentCredential = $('#credential-select').val();
            $('#credential-select').html(renderCredentialOptions(selectedProvider, currentCredential));
        });

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

    // Execute functionality
    async function showExecuteForm(modelId) {
        $('#content').html(Utils.renderLoading());

        try {
            // Load model and prompts
            const [model, promptsData] = await Promise.all([
                API.getModel(modelId),
                API.listPrompts()
            ]);
            prompts = promptsData.prompts || [];

            $('#content').html(renderExecuteForm(model));
            bindExecuteFormEvents(modelId);
        } catch (error) {
            Utils.showToast('Failed to load data', 'error');
            render();
        }
    }

    function renderExecuteForm(model) {
        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Execute Model: ${Utils.escapeHtml(model.name)}</h2>
                </div>

                <div class="grid grid-cols-2 gap-6">
                    <div class="card">
                        <h3 class="font-medium mb-4">Execution Parameters</h3>
                        <form id="execute-form">
                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">System Prompt (optional)</label>
                                <select name="prompt_id" id="prompt-select" class="form-input">
                                    <option value="">-- No system prompt --</option>
                                    ${prompts.filter(p => p.enabled).sort((a, b) => a.name.localeCompare(b.name)).map(p => `
                                        <option value="${Utils.escapeHtml(p.id)}">${Utils.escapeHtml(p.name)}</option>
                                    `).join('')}
                                </select>
                            </div>

                            <div id="variables-container" class="mb-4 hidden">
                                <label class="block text-sm font-medium text-gray-700 mb-2">Prompt Variables</label>
                                <div id="variables-fields"></div>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">User Message</label>
                                <textarea name="user_message" class="form-input h-32" placeholder="Enter your message..." required></textarea>
                            </div>

                            <div class="grid grid-cols-2 gap-4 mb-4">
                                <div>
                                    <label class="block text-sm text-gray-600 mb-1">Temperature (optional)</label>
                                    <input type="number" name="temperature" step="0.1" min="0" max="2"
                                        value="${model.config?.temperature ?? ''}"
                                        class="form-input" placeholder="0.7">
                                </div>
                                <div>
                                    <label class="block text-sm text-gray-600 mb-1">Max Tokens (optional)</label>
                                    <input type="number" name="max_tokens" min="1"
                                        value="${model.config?.max_tokens ?? ''}"
                                        class="form-input" placeholder="4096">
                                </div>
                            </div>

                            <!-- Structured Output Section -->
                            <div class="mb-4 border-t pt-4">
                                <label class="flex items-center mb-2">
                                    <input type="checkbox" name="structured_output_enabled" id="structured-output-toggle">
                                    <span class="ml-2 text-sm font-medium text-gray-700">Enable Structured Output</span>
                                </label>
                                <div id="structured-output-fields" class="hidden">
                                    <div class="mb-2">
                                        <label class="block text-sm text-gray-600 mb-1">Schema Name</label>
                                        <input type="text" name="schema_name" class="form-input" placeholder="my_response" value="response">
                                    </div>
                                    <div class="mb-2">
                                        <div class="flex justify-between items-center mb-1">
                                            <label class="block text-sm text-gray-600">JSON Schema</label>
                                            <button type="button" id="example-schema-btn" class="text-xs text-blue-600 hover:text-blue-800">Load Example</button>
                                        </div>
                                        <textarea name="json_schema" id="json-schema-input" class="form-input font-mono text-sm h-32"
                                            placeholder='{"type":"object","properties":{"answer":{"type":"string"},"confidence":{"type":"number"}},"required":["answer"]}'></textarea>
                                        <p class="text-xs text-gray-500 mt-1">Define the expected response format using JSON Schema</p>
                                    </div>
                                    <div>
                                        <label class="flex items-center">
                                            <input type="checkbox" name="strict_schema" checked>
                                            <span class="ml-2 text-sm text-gray-600">Strict mode (requires additionalProperties: false)</span>
                                        </label>
                                    </div>
                                </div>
                            </div>

                            <button type="submit" class="btn btn-primary w-full">Execute</button>
                        </form>
                    </div>

                    <div class="card">
                        <h3 class="font-medium mb-4">Response</h3>
                        <div id="response-area" class="bg-gray-50 p-4 rounded min-h-[300px] text-sm">
                            <p class="text-gray-400 italic">Response will appear here...</p>
                        </div>
                        <div id="usage-info" class="hidden mt-4 text-sm text-gray-600 border-t pt-4">
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    function extractVariablesFromContent(content) {
        const regex = /\$\{var:([a-zA-Z0-9][-a-zA-Z0-9]*)(?::([^}]*))?\}/g;
        const variables = [];
        const seen = new Set();
        let match;

        while ((match = regex.exec(content)) !== null) {
            const name = match[1];

            if (!seen.has(name)) {
                seen.add(name);
                variables.push({
                    name: name,
                    defaultValue: match[2] || null
                });
            }
        }
        return variables;
    }

    function renderVariableFields(variables) {
        if (variables.length === 0) return '';

        return variables.map(v => `
            <div class="mb-2">
                <label class="block text-xs text-gray-600 mb-1">
                    ${Utils.escapeHtml(v.name)}
                    ${v.defaultValue !== null ? `<span class="text-gray-400">(default: ${Utils.escapeHtml(v.defaultValue)})</span>` : '<span class="text-red-500">*</span>'}
                </label>
                <input type="text" name="var_${Utils.escapeHtml(v.name)}"
                    value="${Utils.escapeHtml(v.defaultValue || '')}"
                    class="form-input text-sm"
                    ${v.defaultValue === null ? 'required' : ''}>
            </div>
        `).join('');
    }

    function bindExecuteFormEvents(modelId) {
        $('#back-btn').on('click', () => render());

        // Toggle structured output fields visibility
        $('#structured-output-toggle').on('change', function() {
            if ($(this).is(':checked')) {
                $('#structured-output-fields').removeClass('hidden');
            } else {
                $('#structured-output-fields').addClass('hidden');
            }
        });

        // Load example JSON schema
        $('#example-schema-btn').on('click', function() {
            const exampleSchema = {
                type: 'object',
                properties: {
                    answer: {
                        type: 'string',
                        description: 'The main response or answer'
                    },
                    confidence: {
                        type: 'number',
                        minimum: 0,
                        maximum: 1,
                        description: 'Confidence level from 0 to 1'
                    },
                    reasoning: {
                        type: 'string',
                        description: 'Explanation of the reasoning'
                    },
                    sources: {
                        type: 'array',
                        items: { type: 'string' },
                        description: 'List of sources used'
                    }
                },
                required: ['answer'],
                additionalProperties: false
            };
            $('#json-schema-input').val(JSON.stringify(exampleSchema, null, 2));
            Utils.showToast('Example schema loaded', 'info');
        });

        // Update variables when prompt changes
        $('#prompt-select').on('change', async function() {
            const promptId = $(this).val();

            if (!promptId) {
                $('#variables-container').addClass('hidden');
                $('#variables-fields').html('');
                return;
            }

            try {
                const prompt = await API.getPrompt(promptId);
                const variables = extractVariablesFromContent(prompt.content);

                if (variables.length > 0) {
                    $('#variables-fields').html(renderVariableFields(variables));
                    $('#variables-container').removeClass('hidden');
                } else {
                    $('#variables-container').addClass('hidden');
                    $('#variables-fields').html('');
                }
            } catch (error) {
                Utils.showToast('Failed to load prompt', 'error');
            }
        });

        $('#execute-form').on('submit', async function(e) {
            e.preventDefault();

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Executing...');

            $('#response-area').html('<p class="text-gray-400 italic">Waiting for response...</p>');
            $('#usage-info').addClass('hidden');

            const formData = new FormData(this);
            const data = {
                user_message: formData.get('user_message'),
                variables: {}
            };

            const promptId = formData.get('prompt_id');

            if (promptId) {
                data.prompt_id = promptId;
            }

            const temperature = formData.get('temperature');

            if (temperature) {
                data.temperature = parseFloat(temperature);
            }

            const maxTokens = formData.get('max_tokens');

            if (maxTokens) {
                data.max_tokens = parseInt(maxTokens);
            }

            // Collect variables
            for (const [key, value] of formData.entries()) {
                if (key.startsWith('var_')) {
                    const varName = key.substring(4);
                    data.variables[varName] = value;
                }
            }

            // Add structured output if enabled
            if (formData.get('structured_output_enabled') === 'on') {
                const schemaStr = formData.get('json_schema');

                if (schemaStr) {
                    try {
                        const schema = JSON.parse(schemaStr);
                        data.response_format = {
                            type: 'json_schema',
                            json_schema: {
                                name: formData.get('schema_name') || 'response',
                                strict: formData.get('strict_schema') === 'on',
                                schema: schema
                            }
                        };
                    } catch (e) {
                        Utils.showToast('Invalid JSON schema', 'error');
                        $btn.prop('disabled', false).text(originalText);
                        return;
                    }
                }
            }

            try {
                const result = await API.executeModel(modelId, data);

                // Check if response is valid JSON (structured output)
                let contentHtml;
                let structuredData = null;

                try {
                    structuredData = JSON.parse(result.content);
                    contentHtml = `
                        <div class="mb-2 text-xs font-medium text-green-600">Structured Output Detected</div>
                        <pre class="whitespace-pre-wrap bg-gray-100 p-2 rounded text-xs overflow-auto">${Utils.escapeHtml(JSON.stringify(structuredData, null, 2))}</pre>
                    `;
                } catch {
                    contentHtml = `<pre class="whitespace-pre-wrap">${Utils.escapeHtml(result.content)}</pre>`;
                }

                $('#response-area').html(contentHtml);

                $('#usage-info').removeClass('hidden').html(`
                    <div class="grid grid-cols-4 gap-4">
                        <div>
                            <span class="text-gray-500">Prompt Tokens:</span>
                            <span class="font-medium">${result.usage.prompt_tokens}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">Completion Tokens:</span>
                            <span class="font-medium">${result.usage.completion_tokens}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">Total Tokens:</span>
                            <span class="font-medium">${result.usage.total_tokens}</span>
                        </div>
                        <div>
                            <span class="text-gray-500">Time:</span>
                            <span class="font-medium">${result.execution_time_ms}ms</span>
                        </div>
                    </div>
                `);

                Utils.showToast('Execution completed', 'success');
            } catch (error) {
                $('#response-area').html(`<p class="text-red-500">${Utils.escapeHtml(error.message)}</p>`);
                Utils.showToast(error.message, 'error');
            } finally {
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    return { render };
})();
