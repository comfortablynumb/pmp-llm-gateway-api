/**
 * Credentials CRUD view
 */
const Credentials = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listCredentials();
            $('#content').html(renderList(data.credentials || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(credentials) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${credentials.length} credential(s)</p>
                <button id="create-credential-btn" class="btn btn-primary">+ New Credential</button>
            </div>

            ${credentials.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Provider</th>
                                <th>Endpoint</th>
                                <th>Status</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${credentials.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No credentials configured yet')}
        `;
    }

    function renderRow(cred) {
        const statusClass = cred.enabled ? 'badge-success' : 'badge-gray';
        const statusText = cred.enabled ? 'Enabled' : 'Disabled';
        const providerLabel = getProviderLabel(cred.credential_type);
        const category = getProviderCategory(cred.credential_type);

        let categoryClass, categoryLabel;

        if (category === 'llm') {
            categoryClass = 'badge-success';
            categoryLabel = 'LLM';
        } else if (category === 'knowledge_base') {
            categoryClass = 'badge-warning';
            categoryLabel = 'KB';
        } else {
            categoryClass = 'bg-orange-100 text-orange-800';
            categoryLabel = 'HTTP';
        }

        const isHttpProvider = category === 'http';
        // pgvector can be tested (database connection), other KB providers cannot
        const canTest = category === 'llm' || cred.credential_type === 'pgvector';

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(cred.id)}</td>
                <td>${Utils.escapeHtml(cred.name)}</td>
                <td>
                    <span class="badge ${categoryClass} mr-2">${categoryLabel}</span>
                    ${Utils.escapeHtml(providerLabel)}
                </td>
                <td class="font-mono text-sm">${Utils.escapeHtml(cred.endpoint || '-')}</td>
                <td><span class="badge ${statusClass}">${statusText}</span></td>
                <td>
                    ${canTest ? `<button class="test-btn btn-sm btn-success-sm mr-2" data-id="${Utils.escapeHtml(cred.id)}" data-type="${Utils.escapeHtml(cred.credential_type)}">Test</button>` : ''}
                    <button class="edit-btn btn-sm btn-edit mr-2" data-id="${Utils.escapeHtml(cred.id)}">Edit</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(cred.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function getProviderLabel(type) {
        const labels = {
            // LLM Providers
            'openai': 'OpenAI',
            'anthropic': 'Anthropic',
            'azure_openai': 'Azure OpenAI',
            'aws_bedrock': 'AWS Bedrock',
            // Knowledge Base Providers
            'pgvector': 'PostgreSQL pgvector',
            'aws_knowledge_base': 'AWS Bedrock KB',
            'pinecone': 'Pinecone',
            // HTTP API
            'http_api_key': 'HTTP API Key'
        };
        return labels[type] || type;
    }

    function getProviderCategory(type) {
        const kbProviders = ['pgvector', 'aws_knowledge_base', 'pinecone'];
        const httpProviders = ['http_api_key'];

        if (kbProviders.includes(type)) return 'knowledge_base';
        if (httpProviders.includes(type)) return 'http';

        return 'llm';
    }

    function requiresApiKey(type) {
        // These providers use IAM or connection strings instead of API keys
        const noApiKeyProviders = ['aws_bedrock', 'pgvector', 'aws_knowledge_base'];
        return !noApiKeyProviders.includes(type);
    }

    function renderForm(cred = null) {
        const isEdit = !!cred;
        const title = isEdit ? 'Edit Credential' : 'Create Credential';
        const credType = cred?.credential_type || 'openai';
        const showAzureFields = credType === 'azure_openai';
        const showPgvectorFields = credType === 'pgvector';
        const showAwsKbFields = credType === 'aws_knowledge_base';
        const showPineconeFields = credType === 'pinecone';
        const isKbProvider = ['pgvector', 'aws_knowledge_base', 'pinecone'].includes(credType);

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="credential-form" class="card">
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                        <input type="text" name="id" value="${Utils.escapeHtml(cred?.id || '')}"
                            class="form-input ${isEdit ? 'bg-gray-100' : ''}"
                            placeholder="my-credential-id" ${isEdit ? 'readonly' : 'required'}>
                        <p class="text-xs text-gray-500 mt-1">Alphanumeric, hyphens, and underscores only, max 50 chars</p>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" value="${Utils.escapeHtml(cred?.name || '')}"
                            class="form-input" placeholder="My API Credential" required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Provider</label>
                        <select name="credential_type" id="provider-select" class="form-input" ${isEdit ? 'disabled' : 'required'}>
                            <optgroup label="LLM Providers">
                                <option value="openai" ${cred?.credential_type === 'openai' ? 'selected' : ''}>OpenAI</option>
                                <option value="anthropic" ${cred?.credential_type === 'anthropic' ? 'selected' : ''}>Anthropic</option>
                                <option value="azure_openai" ${cred?.credential_type === 'azure_openai' ? 'selected' : ''}>Azure OpenAI</option>
                                <option value="aws_bedrock" ${cred?.credential_type === 'aws_bedrock' ? 'selected' : ''}>AWS Bedrock</option>
                            </optgroup>
                            <optgroup label="Knowledge Base Providers">
                                <option value="pgvector" ${cred?.credential_type === 'pgvector' ? 'selected' : ''}>PostgreSQL pgvector</option>
                                <option value="aws_knowledge_base" ${cred?.credential_type === 'aws_knowledge_base' ? 'selected' : ''}>AWS Bedrock Knowledge Base</option>
                                <option value="pinecone" ${cred?.credential_type === 'pinecone' ? 'selected' : ''}>Pinecone</option>
                            </optgroup>
                            <optgroup label="HTTP Providers">
                                <option value="http_api_key" ${cred?.credential_type === 'http_api_key' ? 'selected' : ''}>HTTP API Key</option>
                            </optgroup>
                        </select>
                        ${isEdit ? `<input type="hidden" name="credential_type" value="${Utils.escapeHtml(cred?.credential_type || '')}">` : ''}
                    </div>

                    <div id="api-key-section" class="mb-4 ${requiresApiKey(credType) ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">API Key</label>
                        <input type="password" name="api_key" value=""
                            class="form-input" placeholder="${isEdit ? '(unchanged)' : 'sk-...'}" ${isEdit || !requiresApiKey(credType) ? '' : 'required'}>
                        ${isEdit ? '<p class="text-xs text-gray-500 mt-1">Leave blank to keep current key</p>' : ''}
                        ${!requiresApiKey(credType) ? '<p class="text-xs text-gray-500 mt-1">This provider uses IAM authentication instead of API keys</p>' : ''}
                    </div>

                    <!-- Azure OpenAI fields -->
                    <div id="azure-endpoint-section" class="mb-4 ${showAzureFields ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Endpoint URL</label>
                        <input type="url" name="endpoint" value="${Utils.escapeHtml(cred?.endpoint || '')}"
                            class="form-input azure-field" placeholder="https://your-resource.openai.azure.com">
                        <p class="text-xs text-gray-500 mt-1">Required for Azure OpenAI</p>
                    </div>

                    <div id="azure-deployment-section" class="mb-4 ${showAzureFields ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Deployment Name</label>
                        <input type="text" name="deployment" value="${Utils.escapeHtml(cred?.deployment || '')}"
                            class="form-input azure-field" placeholder="my-gpt-4-deployment">
                        <p class="text-xs text-gray-500 mt-1">Azure OpenAI deployment name</p>
                    </div>

                    <!-- pgvector fields -->
                    <div id="pgvector-section" class="mb-4 ${showPgvectorFields ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Connection String</label>
                        <input type="text" name="endpoint" value="${Utils.escapeHtml(cred?.endpoint || '')}"
                            class="form-input pgvector-field" placeholder="postgresql://user:pass@host:5432/db">
                        <p class="text-xs text-gray-500 mt-1">PostgreSQL connection string with pgvector extension</p>
                    </div>

                    <!-- AWS Knowledge Base fields -->
                    <div id="aws-kb-section" class="mb-4 ${showAwsKbFields ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Knowledge Base ID</label>
                        <input type="text" name="deployment" value="${Utils.escapeHtml(cred?.deployment || '')}"
                            class="form-input aws-kb-field" placeholder="KB123ABC456">
                        <p class="text-xs text-gray-500 mt-1">AWS Bedrock Knowledge Base ID</p>
                    </div>

                    <div id="aws-kb-region-section" class="mb-4 ${showAwsKbFields ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">AWS Region</label>
                        <input type="text" name="endpoint" value="${Utils.escapeHtml(cred?.endpoint || '')}"
                            class="form-input aws-kb-field" placeholder="us-east-1">
                        <p class="text-xs text-gray-500 mt-1">AWS region where the Knowledge Base is deployed</p>
                    </div>

                    <!-- Pinecone fields -->
                    <div id="pinecone-section" class="mb-4 ${showPineconeFields ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Index Host URL</label>
                        <input type="url" name="endpoint" value="${Utils.escapeHtml(cred?.endpoint || '')}"
                            class="form-input pinecone-field" placeholder="https://index-name-abc123.svc.environment.pinecone.io">
                        <p class="text-xs text-gray-500 mt-1">Pinecone index host URL</p>
                    </div>

                    <div id="pinecone-namespace-section" class="mb-4 ${showPineconeFields ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Namespace</label>
                        <input type="text" name="deployment" value="${Utils.escapeHtml(cred?.deployment || '')}"
                            class="form-input pinecone-field" placeholder="default">
                        <p class="text-xs text-gray-500 mt-1">Optional: Pinecone namespace</p>
                    </div>

                    <!-- HTTP API Key fields -->
                    <div id="http-header-name-section" class="mb-4 ${credType === 'http_api_key' ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Header Name</label>
                        <input type="text" name="deployment" value="${Utils.escapeHtml(cred?.deployment || 'Authorization')}"
                            class="form-input http-api-key-field" placeholder="Authorization">
                        <p class="text-xs text-gray-500 mt-1">HTTP header name for the API key (e.g., Authorization, X-API-Key)</p>
                    </div>

                    <div id="http-header-value-section" class="mb-4 ${credType === 'http_api_key' ? '' : 'hidden'}">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Header Value Template</label>
                        <input type="text" name="header_value" value="${Utils.escapeHtml(cred?.header_value || 'Bearer ${api-key}')}"
                            class="form-input http-api-key-field" placeholder="Bearer \${api-key}">
                        <p class="text-xs text-gray-500 mt-1">Header value template. Use <code>\${api-key}</code> to insert the API key</p>
                    </div>

                    ${isEdit ? `
                        <div class="flex items-center mt-4">
                            <input type="checkbox" name="enabled" id="enabled" ${cred?.enabled !== false ? 'checked' : ''}>
                            <label for="enabled" class="ml-2 text-sm text-gray-700">Enabled</label>
                        </div>
                    ` : ''}

                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">${isEdit ? 'Update' : 'Create'}</button>
                    </div>
                </form>
            </div>
        `;
    }

    function getDefaultModel(providerType) {
        const defaults = {
            'openai': 'gpt-4o-mini',
            'anthropic': 'claude-3-haiku-20240307',
            'azure_openai': 'gpt-4o-mini',
            'aws_bedrock': 'anthropic.claude-3-haiku-20240307-v1:0'
        };
        return defaults[providerType] || 'gpt-4o-mini';
    }

    function renderTestForm(credId, providerType) {
        const defaultModel = getDefaultModel(providerType);
        const providerLabel = getProviderLabel(providerType);
        const isPgvector = providerType === 'pgvector';

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Test Credential: ${Utils.escapeHtml(credId)}</h2>
                </div>

                <div class="card mb-6">
                    <div class="mb-4">
                        <span class="text-sm text-gray-600">Provider:</span>
                        <span class="font-medium ml-2">${Utils.escapeHtml(providerLabel)}</span>
                    </div>

                    ${isPgvector ? `
                        <p class="text-sm text-gray-600 mb-4">
                            This will test the PostgreSQL database connection and verify the pgvector extension is installed.
                        </p>
                        <form id="test-form">
                            <input type="hidden" name="model" value="PostgreSQL">
                            <input type="hidden" name="message" value="test">
                            <button type="submit" class="btn btn-primary">Test Connection</button>
                        </form>
                    ` : `
                        <form id="test-form">
                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Model</label>
                                <input type="text" name="model" value="${Utils.escapeHtml(defaultModel)}"
                                    class="form-input" placeholder="Model ID" required>
                                <p class="text-xs text-gray-500 mt-1">The model to use for the test request</p>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Test Message</label>
                                <textarea name="message" rows="3" class="form-input"
                                    placeholder="Enter a test message..." required>Hello! Please respond with a short greeting.</textarea>
                            </div>

                            <button type="submit" class="btn btn-primary">Run Test</button>
                        </form>
                    `}
                </div>

                <div id="test-result" class="hidden">
                    <div class="card">
                        <h3 class="font-medium mb-4">Test Result</h3>
                        <div id="test-result-content"></div>
                    </div>
                </div>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-credential-btn').on('click', () => showForm());

        $('.test-btn').on('click', function() {
            const id = $(this).data('id');
            const type = $(this).data('type');
            showTest(id, type);
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
        let cred = null;

        if (id) {
            $('#content').html(Utils.renderLoading());

            try {
                cred = await API.getCredential(id);
            } catch (error) {
                Utils.showToast('Failed to load credential', 'error');
                return render();
            }
        }

        $('#content').html(renderForm(cred));
        bindFormEvents(id);
    }

    function showTest(credId, providerType) {
        $('#content').html(renderTestForm(credId, providerType));
        bindTestEvents(credId);
    }

    function bindTestEvents(credId) {
        $('#back-btn').on('click', () => render());

        $('#test-form').on('submit', async function(e) {
            e.preventDefault();

            const formData = Utils.getFormData(this);
            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Testing...');

            try {
                const result = await API.testCredential(credId, formData);
                displayTestResult(result);
            } catch (error) {
                displayTestResult({
                    success: false,
                    provider: 'unknown',
                    model: formData.model,
                    error: error.message,
                    latency_ms: 0
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
                <span class="text-gray-500 text-sm ml-4">${result.latency_ms}ms</span>
            </div>

            <div class="space-y-3 text-sm">
                <div>
                    <span class="text-gray-600">Provider:</span>
                    <span class="font-medium ml-2">${Utils.escapeHtml(result.provider)}</span>
                </div>
                <div>
                    <span class="text-gray-600">Model:</span>
                    <span class="font-mono ml-2">${Utils.escapeHtml(result.model)}</span>
                </div>
        `;

        if (result.response) {
            html += `
                <div>
                    <span class="text-gray-600">Response:</span>
                    <pre class="mt-2 bg-gray-50 p-3 rounded text-sm whitespace-pre-wrap">${Utils.escapeHtml(result.response)}</pre>
                </div>
            `;
        }

        if (result.error) {
            html += `
                <div>
                    <span class="text-gray-600">Error:</span>
                    <pre class="mt-2 bg-red-50 text-red-700 p-3 rounded text-sm whitespace-pre-wrap">${Utils.escapeHtml(result.error)}</pre>
                </div>
            `;
        }

        html += '</div>';

        $('#test-result').removeClass('hidden');
        $('#test-result-content').html(html);
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        // Show/hide provider-specific fields based on selection
        $('#provider-select').on('change', function() {
            const provider = $(this).val();

            // Hide all provider-specific sections
            $('#azure-endpoint-section, #azure-deployment-section').addClass('hidden');
            $('#pgvector-section').addClass('hidden');
            $('#aws-kb-section, #aws-kb-region-section').addClass('hidden');
            $('#pinecone-section, #pinecone-namespace-section').addClass('hidden');
            $('#http-header-name-section, #http-header-value-section').addClass('hidden');

            // Show/hide API key section based on provider
            if (requiresApiKey(provider)) {
                $('#api-key-section').removeClass('hidden');
                $('#api-key-section input[name="api_key"]').prop('required', !editId);
            } else {
                $('#api-key-section').addClass('hidden');
                $('#api-key-section input[name="api_key"]').prop('required', false);
            }

            // Show relevant sections based on provider
            if (provider === 'azure_openai') {
                $('#azure-endpoint-section, #azure-deployment-section').removeClass('hidden');
            } else if (provider === 'pgvector') {
                $('#pgvector-section').removeClass('hidden');
            } else if (provider === 'aws_knowledge_base') {
                $('#aws-kb-section, #aws-kb-region-section').removeClass('hidden');
            } else if (provider === 'pinecone') {
                $('#pinecone-section, #pinecone-namespace-section').removeClass('hidden');
            } else if (provider === 'http_api_key') {
                $('#http-header-name-section, #http-header-value-section').removeClass('hidden');
            }
        });

        $('#credential-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            // Remove empty api_key on update
            if (editId && !formData.api_key) {
                delete formData.api_key;
            }

            // Remove empty endpoint/deployment/header_value
            if (!formData.endpoint) {
                delete formData.endpoint;
            }

            if (!formData.deployment) {
                delete formData.deployment;
            }

            if (!formData.header_value) {
                delete formData.header_value;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                if (editId) {
                    delete formData.id;
                    delete formData.credential_type;
                    await API.updateCredential(editId, formData);
                    Utils.showToast('Credential updated successfully', 'success');
                } else {
                    await API.createCredential(formData);
                    Utils.showToast('Credential created successfully', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete credential "${id}"?`)) {
            return;
        }

        try {
            await API.deleteCredential(id);
            Utils.showToast('Credential deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
