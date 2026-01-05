/**
 * Credentials view (read-only)
 */
const Credentials = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listCredentialProviders();
            $('#content').html(renderList(data.providers || []));
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(providers) {
        return `
            <div class="mb-6">
                <p class="text-gray-600">
                    Credential providers are configured via environment variables or external services.
                    This view shows which providers are currently available.
                </p>
            </div>

            ${providers.length > 0 ? `
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    ${providers.map(renderProvider).join('')}
                </div>
            ` : Utils.renderEmpty('No credential providers configured')}

            <div class="card mt-6">
                <h3 class="font-medium mb-4">Available Provider Types</h3>
                <div class="space-y-4 text-sm">
                    <div class="flex items-start">
                        <span class="badge badge-success mr-3">ENV</span>
                        <div>
                            <p class="font-medium">Environment Variables</p>
                            <p class="text-gray-500">Credentials stored in environment variables (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)</p>
                        </div>
                    </div>
                    <div class="flex items-start">
                        <span class="badge badge-gray mr-3">AWS</span>
                        <div>
                            <p class="font-medium">AWS Secrets Manager</p>
                            <p class="text-gray-500">Credentials stored in AWS Secrets Manager with automatic rotation support</p>
                        </div>
                    </div>
                    <div class="flex items-start">
                        <span class="badge badge-gray mr-3">Vault</span>
                        <div>
                            <p class="font-medium">HashiCorp Vault</p>
                            <p class="text-gray-500">Credentials stored in HashiCorp Vault with KV secrets engine</p>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    function renderProvider(provider) {
        return `
            <div class="card">
                <div class="flex items-center justify-between mb-2">
                    <h3 class="font-medium">${Utils.escapeHtml(provider.provider_type)}</h3>
                    <span class="badge badge-success">Active</span>
                </div>
                ${provider.description ? `
                    <p class="text-sm text-gray-500">${Utils.escapeHtml(provider.description)}</p>
                ` : ''}
            </div>
        `;
    }

    return { render };
})();
