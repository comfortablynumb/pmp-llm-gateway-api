/**
 * Dashboard view
 */
const Dashboard = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const [models, prompts, apiKeys, workflows, credentials] = await Promise.all([
                API.listModels().catch(() => ({ models: [], total: 0 })),
                API.listPrompts().catch(() => ({ prompts: [], total: 0 })),
                API.listApiKeys().catch(() => ({ api_keys: [], total: 0 })),
                API.listWorkflows().catch(() => ({ workflows: [], total: 0 })),
                API.listCredentialProviders().catch(() => ({ providers: [] }))
            ]);

            $('#content').html(renderDashboard({
                models: models.total || models.models?.length || 0,
                prompts: prompts.total || prompts.prompts?.length || 0,
                apiKeys: apiKeys.total || apiKeys.api_keys?.length || 0,
                workflows: workflows.total || workflows.workflows?.length || 0,
                providers: credentials.providers || []
            }));

            bindEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderDashboard(data) {
        return `
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
                ${renderCard('Models', data.models, '#models', 'blue')}
                ${renderCard('Prompts', data.prompts, '#prompts', 'green')}
                ${renderCard('API Keys', data.apiKeys, '#api-keys', 'purple')}
                ${renderCard('Workflows', data.workflows, '#workflows', 'orange')}
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <div class="card">
                    <h3 class="text-lg font-semibold mb-4">Quick Actions</h3>
                    <div class="space-y-2">
                        <a href="#models" class="quick-action block p-3 rounded-lg hover:bg-gray-50 border border-gray-200" data-route="models">
                            + Create Model
                        </a>
                        <a href="#prompts" class="quick-action block p-3 rounded-lg hover:bg-gray-50 border border-gray-200" data-route="prompts">
                            + Create Prompt
                        </a>
                        <a href="#api-keys" class="quick-action block p-3 rounded-lg hover:bg-gray-50 border border-gray-200" data-route="api-keys">
                            + Create API Key
                        </a>
                        <a href="#workflows" class="quick-action block p-3 rounded-lg hover:bg-gray-50 border border-gray-200" data-route="workflows">
                            + Create Workflow
                        </a>
                    </div>
                </div>

                <div class="card">
                    <h3 class="text-lg font-semibold mb-4">Credential Providers</h3>
                    ${data.providers.length > 0 ? `
                        <div class="space-y-2">
                            ${data.providers.map(p => `
                                <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                                    <span class="font-medium">${Utils.escapeHtml(p.provider_type)}</span>
                                    <span class="text-sm text-gray-500">${Utils.escapeHtml(p.description || '')}</span>
                                </div>
                            `).join('')}
                        </div>
                    ` : `
                        <p class="text-gray-500">No credential providers configured</p>
                    `}
                </div>
            </div>
        `;
    }

    function renderCard(title, count, link, color) {
        const colors = {
            blue: 'bg-blue-500',
            green: 'bg-green-500',
            purple: 'bg-purple-500',
            orange: 'bg-orange-500'
        };

        return `
            <a href="${link}" class="card hover:shadow-lg transition-shadow cursor-pointer">
                <div class="flex items-center justify-between">
                    <div>
                        <p class="text-gray-500 text-sm">${title}</p>
                        <p class="text-3xl font-bold mt-1">${count}</p>
                    </div>
                    <div class="w-12 h-12 ${colors[color]} rounded-full flex items-center justify-center text-white text-xl">
                        ${count}
                    </div>
                </div>
            </a>
        `;
    }

    function bindEvents() {
        $('.quick-action').on('click', function(e) {
            e.preventDefault();
            const route = $(this).data('route');
            App.navigate(route);
        });
    }

    return { render };
})();
