/**
 * API client for Admin endpoints
 */
const API = (function() {
    const BASE_URL = '/api/v1';

    function getHeaders() {
        const token = Auth.getToken();
        return {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${token}`
        };
    }

    async function request(method, endpoint, data = null) {
        const options = {
            method: method,
            headers: getHeaders()
        };

        if (data && method !== 'GET') {
            options.body = JSON.stringify(data);
        }

        const response = await fetch(`${BASE_URL}${endpoint}`, options);

        if (response.status === 401) {
            Auth.clearToken();
            Auth.showLoginModal();
            throw new Error('Authentication required');
        }

        if (!response.ok) {
            let errorMessage = 'Request failed';

            try {
                const error = await response.json();
                errorMessage = error.error?.message || error.message || errorMessage;
            } catch (e) {
                // Ignore JSON parse errors
            }
            throw new Error(errorMessage);
        }

        // Handle empty responses
        const text = await response.text();

        if (!text) return null;
        return JSON.parse(text);
    }

    async function uploadFiles(endpoint, files) {
        const token = Auth.getToken();
        const formData = new FormData();

        for (const file of files) {
            formData.append('files', file, file.name);
        }

        const response = await fetch(`${BASE_URL}${endpoint}`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${token}`
            },
            body: formData
        });

        if (response.status === 401) {
            Auth.clearToken();
            Auth.showLoginModal();
            throw new Error('Authentication required');
        }

        if (!response.ok) {
            let errorMessage = 'Upload failed';

            try {
                const error = await response.json();
                errorMessage = error.error?.message || error.message || errorMessage;
            } catch (e) {
                // Ignore JSON parse errors
            }
            throw new Error(errorMessage);
        }

        const text = await response.text();

        if (!text) return null;
        return JSON.parse(text);
    }

    return {
        // Models
        listModels: () => request('GET', '/models'),
        getModel: (id) => request('GET', `/models/${encodeURIComponent(id)}`),
        createModel: (data) => request('POST', '/models', data),
        updateModel: (id, data) => request('PUT', `/models/${encodeURIComponent(id)}`, data),
        deleteModel: (id) => request('DELETE', `/models/${encodeURIComponent(id)}`),
        executeModel: (id, data) => request('POST', `/models/${encodeURIComponent(id)}/execute`, data),

        // Prompts
        listPrompts: () => request('GET', '/prompts'),
        getPrompt: (id) => request('GET', `/prompts/${encodeURIComponent(id)}`),
        createPrompt: (data) => request('POST', '/prompts', data),
        updatePrompt: (id, data) => request('PUT', `/prompts/${encodeURIComponent(id)}`, data),
        deletePrompt: (id) => request('DELETE', `/prompts/${encodeURIComponent(id)}`),
        renderPrompt: (id, variables) => request('POST', `/prompts/${encodeURIComponent(id)}/render`, { variables }),
        listPromptVersions: (id) => request('GET', `/prompts/${encodeURIComponent(id)}/versions`),
        revertPromptVersion: (id, version) => request('POST', `/prompts/${encodeURIComponent(id)}/revert/${version}`),

        // API Keys
        listApiKeys: () => request('GET', '/api-keys'),
        getApiKey: (id) => request('GET', `/api-keys/${encodeURIComponent(id)}`),
        createApiKey: (data) => request('POST', '/api-keys', data),
        updateApiKey: (id, data) => request('PUT', `/api-keys/${encodeURIComponent(id)}`, data),
        deleteApiKey: (id) => request('DELETE', `/api-keys/${encodeURIComponent(id)}`),
        suspendApiKey: (id) => request('POST', `/api-keys/${encodeURIComponent(id)}/suspend`),
        activateApiKey: (id) => request('POST', `/api-keys/${encodeURIComponent(id)}/activate`),
        revokeApiKey: (id) => request('POST', `/api-keys/${encodeURIComponent(id)}/revoke`),

        // Workflows
        listWorkflows: () => request('GET', '/workflows'),
        getWorkflow: (id) => request('GET', `/workflows/${encodeURIComponent(id)}`),
        createWorkflow: (data) => request('POST', '/workflows', data),
        updateWorkflow: (id, data) => request('PUT', `/workflows/${encodeURIComponent(id)}`, data),
        deleteWorkflow: (id) => request('DELETE', `/workflows/${encodeURIComponent(id)}`),
        testWorkflow: (id, data) => request('POST', `/workflows/${encodeURIComponent(id)}/test`, data),
        executeWorkflow: (id, data) => request('POST', `/workflows/${encodeURIComponent(id)}/execute`, data),
        cloneWorkflow: (id, data) => request('POST', `/workflows/${encodeURIComponent(id)}/clone`, data),

        // Credentials
        listCredentials: () => request('GET', '/credentials'),
        getCredential: (id) => request('GET', `/credentials/${encodeURIComponent(id)}`),
        createCredential: (data) => request('POST', '/credentials', data),
        updateCredential: (id, data) => request('PUT', `/credentials/${encodeURIComponent(id)}`, data),
        deleteCredential: (id) => request('DELETE', `/credentials/${encodeURIComponent(id)}`),
        listCredentialProviders: () => request('GET', '/credentials/providers'),
        testCredential: (id, data) => request('POST', `/credentials/${encodeURIComponent(id)}/test`, data),

        // External APIs
        listExternalApis: () => request('GET', '/external-apis'),
        getExternalApi: (id) => request('GET', `/external-apis/${encodeURIComponent(id)}`),
        createExternalApi: (data) => request('POST', '/external-apis', data),
        updateExternalApi: (id, data) => request('PUT', `/external-apis/${encodeURIComponent(id)}`, data),
        deleteExternalApi: (id) => request('DELETE', `/external-apis/${encodeURIComponent(id)}`),

        // Knowledge Bases
        listKnowledgeBases: () => request('GET', '/knowledge-bases'),
        getKnowledgeBase: (id) => request('GET', `/knowledge-bases/${encodeURIComponent(id)}`),
        createKnowledgeBase: (data) => request('POST', '/knowledge-bases', data),
        updateKnowledgeBase: (id, data) => request('PUT', `/knowledge-bases/${encodeURIComponent(id)}`, data),
        deleteKnowledgeBase: (id) => request('DELETE', `/knowledge-bases/${encodeURIComponent(id)}`),
        listKnowledgeBaseTypes: () => request('GET', '/knowledge-bases/types'),
        // Knowledge Base document management
        listDocuments: (kbId) => request('GET', `/knowledge-bases/${encodeURIComponent(kbId)}/documents`),
        ingestDocument: (kbId, data) => request('POST', `/knowledge-bases/${encodeURIComponent(kbId)}/documents`, data),
        getDocument: (kbId, docId) => request('GET', `/knowledge-bases/${encodeURIComponent(kbId)}/documents/${encodeURIComponent(docId)}`),
        deleteDocument: (kbId, docId) => request('DELETE', `/knowledge-bases/${encodeURIComponent(kbId)}/documents/${encodeURIComponent(docId)}`),
        getDocumentChunks: (kbId, docId) => request('GET', `/knowledge-bases/${encodeURIComponent(kbId)}/documents/${encodeURIComponent(docId)}/chunks`),
        disableDocument: (kbId, docId) => request('POST', `/knowledge-bases/${encodeURIComponent(kbId)}/documents/${encodeURIComponent(docId)}/disable`),
        enableDocument: (kbId, docId) => request('POST', `/knowledge-bases/${encodeURIComponent(kbId)}/documents/${encodeURIComponent(docId)}/enable`),
        ingestFiles: (kbId, files) => uploadFiles(`/knowledge-bases/${encodeURIComponent(kbId)}/documents/upload`, files),
        listIngestionOperations: (kbId) => request('GET', `/knowledge-bases/${encodeURIComponent(kbId)}/ingestions`),

        // Experiments (A/B Testing)
        listExperiments: (params) => {
            const query = new URLSearchParams();
            if (params?.status) query.set('status', params.status);
            if (params?.model_id) query.set('model_id', params.model_id);
            if (params?.limit) query.set('limit', params.limit);
            if (params?.offset) query.set('offset', params.offset);
            const qs = query.toString();
            return request('GET', `/experiments${qs ? '?' + qs : ''}`);
        },
        getExperiment: (id) => request('GET', `/experiments/${encodeURIComponent(id)}`),
        createExperiment: (data) => request('POST', '/experiments', data),
        updateExperiment: (id, data) => request('PUT', `/experiments/${encodeURIComponent(id)}`, data),
        deleteExperiment: (id) => request('DELETE', `/experiments/${encodeURIComponent(id)}`),
        addVariant: (experimentId, variant) => request('POST', `/experiments/${encodeURIComponent(experimentId)}/variants`, variant),
        removeVariant: (experimentId, variantId) => request('DELETE', `/experiments/${encodeURIComponent(experimentId)}/variants/${encodeURIComponent(variantId)}`),
        startExperiment: (id) => request('POST', `/experiments/${encodeURIComponent(id)}/start`),
        pauseExperiment: (id) => request('POST', `/experiments/${encodeURIComponent(id)}/pause`),
        resumeExperiment: (id) => request('POST', `/experiments/${encodeURIComponent(id)}/resume`),
        completeExperiment: (id) => request('POST', `/experiments/${encodeURIComponent(id)}/complete`),
        getExperimentResults: (id) => request('GET', `/experiments/${encodeURIComponent(id)}/results`),

        // Test Cases
        listTestCases: (params) => {
            const query = new URLSearchParams();
            if (params?.test_type) query.set('test_type', params.test_type);
            if (params?.enabled !== undefined) query.set('enabled', params.enabled);
            if (params?.tag) query.set('tag', params.tag);
            if (params?.model_id) query.set('model_id', params.model_id);
            if (params?.workflow_id) query.set('workflow_id', params.workflow_id);
            if (params?.limit) query.set('limit', params.limit);
            if (params?.offset) query.set('offset', params.offset);
            const qs = query.toString();
            return request('GET', `/test-cases${qs ? '?' + qs : ''}`);
        },
        getTestCase: (id) => request('GET', `/test-cases/${encodeURIComponent(id)}`),
        createTestCase: (data) => request('POST', '/test-cases', data),
        updateTestCase: (id, data) => request('PUT', `/test-cases/${encodeURIComponent(id)}`, data),
        deleteTestCase: (id) => request('DELETE', `/test-cases/${encodeURIComponent(id)}`),
        executeTestCase: (id) => request('POST', `/test-cases/${encodeURIComponent(id)}/execute`),
        getTestCaseResults: (id, params) => {
            const query = new URLSearchParams();
            if (params?.passed !== undefined) query.set('passed', params.passed);
            if (params?.limit) query.set('limit', params.limit);
            if (params?.offset) query.set('offset', params.offset);
            const qs = query.toString();
            return request('GET', `/test-cases/${encodeURIComponent(id)}/results${qs ? '?' + qs : ''}`);
        },

        // Configuration
        listConfig: () => request('GET', '/config'),
        listConfigByCategory: (category) => request('GET', `/config/category/${encodeURIComponent(category)}`),
        getConfig: (key) => request('GET', `/config/${encodeURIComponent(key)}`),
        updateConfig: (key, data) => request('PUT', `/config/${encodeURIComponent(key)}`, data),
        resetConfig: () => request('DELETE', '/config'),

        // Teams
        listTeams: () => request('GET', '/teams'),
        getTeam: (id) => request('GET', `/teams/${encodeURIComponent(id)}`),
        createTeam: (data) => request('POST', '/teams', data),
        updateTeam: (id, data) => request('PUT', `/teams/${encodeURIComponent(id)}`, data),
        deleteTeam: (id) => request('DELETE', `/teams/${encodeURIComponent(id)}`),
        suspendTeam: (id) => request('POST', `/teams/${encodeURIComponent(id)}/suspend`),
        activateTeam: (id) => request('POST', `/teams/${encodeURIComponent(id)}/activate`),

        // Budgets
        listBudgets: () => request('GET', '/budgets'),
        listBudgetsByTeam: (teamId) => request('GET', `/budgets/by-team/${encodeURIComponent(teamId)}`),
        getBudget: (id) => request('GET', `/budgets/${encodeURIComponent(id)}`),
        createBudget: (data) => request('POST', '/budgets', data),
        updateBudget: (id, data) => request('PUT', `/budgets/${encodeURIComponent(id)}`, data),
        deleteBudget: (id) => request('DELETE', `/budgets/${encodeURIComponent(id)}`),
        resetBudget: (id) => request('POST', `/budgets/${encodeURIComponent(id)}/reset`),
        checkBudget: (data) => request('POST', '/budgets/check', data),

        // Execution Logs
        listExecutionLogs: (params) => {
            const query = new URLSearchParams();
            if (params?.execution_type) query.set('execution_type', params.execution_type);
            if (params?.resource_id) query.set('resource_id', params.resource_id);
            if (params?.status) query.set('status', params.status);
            if (params?.api_key_id) query.set('api_key_id', params.api_key_id);
            if (params?.user_id) query.set('user_id', params.user_id);
            if (params?.from_date) query.set('from_date', params.from_date);
            if (params?.to_date) query.set('to_date', params.to_date);
            if (params?.limit) query.set('limit', params.limit);
            if (params?.offset) query.set('offset', params.offset);
            const qs = query.toString();
            return request('GET', `/execution-logs${qs ? '?' + qs : ''}`);
        },
        getExecutionLog: (id) => request('GET', `/execution-logs/${encodeURIComponent(id)}`),
        deleteExecutionLog: (id) => request('DELETE', `/execution-logs/${encodeURIComponent(id)}`),
        getExecutionStats: (params) => {
            const query = new URLSearchParams();
            if (params?.execution_type) query.set('execution_type', params.execution_type);
            if (params?.resource_id) query.set('resource_id', params.resource_id);
            if (params?.status) query.set('status', params.status);
            if (params?.api_key_id) query.set('api_key_id', params.api_key_id);
            if (params?.user_id) query.set('user_id', params.user_id);
            if (params?.from_date) query.set('from_date', params.from_date);
            if (params?.to_date) query.set('to_date', params.to_date);
            const qs = query.toString();
            return request('GET', `/execution-logs/stats${qs ? '?' + qs : ''}`);
        },
        cleanupExecutionLogs: (data) => request('POST', '/execution-logs/cleanup', data),

        // Webhooks
        listWebhooks: () => request('GET', '/webhooks'),
        getWebhook: (id) => request('GET', `/webhooks/${encodeURIComponent(id)}`),
        createWebhook: (data) => request('POST', '/webhooks', data),
        updateWebhook: (id, data) => request('PUT', `/webhooks/${encodeURIComponent(id)}`, data),
        deleteWebhook: (id) => request('DELETE', `/webhooks/${encodeURIComponent(id)}`),
        resetWebhook: (id) => request('POST', `/webhooks/${encodeURIComponent(id)}/reset`),
        getWebhookDeliveries: (id, params) => {
            const query = new URLSearchParams();
            if (params?.limit) query.set('limit', params.limit);
            if (params?.offset) query.set('offset', params.offset);
            const qs = query.toString();
            return request('GET', `/webhooks/${encodeURIComponent(id)}/deliveries${qs ? '?' + qs : ''}`);
        },
        listWebhookEventTypes: () => request('GET', '/webhooks/event-types')
    };
})();
