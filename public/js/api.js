/**
 * API client for Admin endpoints
 */
const API = (function() {
    const BASE_URL = '/api';

    function getHeaders() {
        const apiKey = Auth.getApiKey();
        return {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`
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
            Auth.clearApiKey();
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

    return {
        // Models
        listModels: () => request('GET', '/models'),
        getModel: (id) => request('GET', `/models/${encodeURIComponent(id)}`),
        createModel: (data) => request('POST', '/models', data),
        updateModel: (id, data) => request('PUT', `/models/${encodeURIComponent(id)}`, data),
        deleteModel: (id) => request('DELETE', `/models/${encodeURIComponent(id)}`),

        // Prompts
        listPrompts: () => request('GET', '/prompts'),
        getPrompt: (id) => request('GET', `/prompts/${encodeURIComponent(id)}`),
        createPrompt: (data) => request('POST', '/prompts', data),
        updatePrompt: (id, data) => request('PUT', `/prompts/${encodeURIComponent(id)}`, data),
        deletePrompt: (id) => request('DELETE', `/prompts/${encodeURIComponent(id)}`),
        renderPrompt: (id, variables) => request('POST', `/prompts/${encodeURIComponent(id)}/render`, { variables }),

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

        // Credentials
        listCredentialProviders: () => request('GET', '/credentials/providers')
    };
})();
