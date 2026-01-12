/**
 * API Keys CRUD view
 */
const ApiKeys = (function() {
    let teams = [];

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const [keysData, teamsData] = await Promise.all([
                API.listApiKeys(),
                API.listTeams()
            ]);
            teams = teamsData.teams || [];
            $('#content').html(renderList(keysData.api_keys || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(apiKeys) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${apiKeys.length} API key(s)</p>
                <button id="create-key-btn" class="btn btn-primary">+ New API Key</button>
            </div>

            ${apiKeys.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>Name</th>
                                <th>Team</th>
                                <th>Key Prefix</th>
                                <th>Status</th>
                                <th>Permissions</th>
                                <th>Last Used</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${apiKeys.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No API keys configured yet')}
        `;
    }

    function renderRow(key) {
        const statusColors = {
            active: 'badge-success',
            suspended: 'badge-warning',
            revoked: 'badge-error',
            expired: 'badge-gray'
        };

        const permissions = [];

        if (key.permissions?.admin) permissions.push('Admin');
        if (key.permissions?.models === 'all') permissions.push('Models');
        if (key.permissions?.prompts === 'all') permissions.push('Prompts');

        const teamName = teams.find(t => t.id === key.team_id)?.name || key.team_id;

        return `
            <tr>
                <td>
                    <div class="font-medium">${Utils.escapeHtml(key.name)}</div>
                    ${key.description ? `<div class="text-xs text-gray-500">${Utils.escapeHtml(key.description)}</div>` : ''}
                </td>
                <td class="text-sm">${Utils.escapeHtml(teamName)}</td>
                <td class="font-mono text-sm">${Utils.escapeHtml(key.key_prefix)}...</td>
                <td><span class="badge ${statusColors[key.status] || 'badge-gray'}">${key.status}</span></td>
                <td class="text-sm">${permissions.length > 0 ? permissions.join(', ') : 'None'}</td>
                <td class="text-sm text-gray-500">${key.last_used_at ? Utils.formatDate(key.last_used_at) : 'Never'}</td>
                <td>
                    <div class="flex items-center gap-1">
                        ${key.status === 'active' ? `
                            <button class="suspend-btn btn-sm btn-warning-sm" data-id="${Utils.escapeHtml(key.id)}">Suspend</button>
                        ` : ''}
                        ${key.status === 'suspended' ? `
                            <button class="activate-btn btn-sm btn-success-sm" data-id="${Utils.escapeHtml(key.id)}">Activate</button>
                        ` : ''}
                        ${key.status !== 'revoked' ? `
                            <button class="revoke-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(key.id)}">Revoke</button>
                        ` : ''}
                        <button class="delete-btn btn-sm btn-gray-sm" data-id="${Utils.escapeHtml(key.id)}">Delete</button>
                    </div>
                </td>
            </tr>
        `;
    }

    function renderForm() {
        const teamOptions = teams.map(t =>
            `<option value="${Utils.escapeHtml(t.id)}">${Utils.escapeHtml(t.name)}</option>`
        ).join('');

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Create API Key</h2>
                </div>

                <form id="key-form" class="card">
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" class="form-input" placeholder="My API Key" required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Team</label>
                        <select name="team_id" class="form-input" required>
                            ${teamOptions}
                        </select>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <input type="text" name="description" class="form-input" placeholder="Optional description">
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Permissions</h3>

                        <div class="mb-4">
                            <label class="flex items-center">
                                <input type="checkbox" name="permissions.admin" class="mr-2">
                                <span class="text-sm">Admin access (full control)</span>
                            </label>
                        </div>

                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">Models</label>
                                <select name="permissions.models" class="form-input">
                                    <option value="all">All</option>
                                    <option value="none">None</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">Prompts</label>
                                <select name="permissions.prompts" class="form-input">
                                    <option value="all">All</option>
                                    <option value="none">None</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">Knowledge Bases</label>
                                <select name="permissions.knowledge_bases" class="form-input">
                                    <option value="all">All</option>
                                    <option value="none">None</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">Chains</label>
                                <select name="permissions.chains" class="form-input">
                                    <option value="all">All</option>
                                    <option value="none">None</option>
                                </select>
                            </div>
                        </div>
                    </div>

                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">Create</button>
                    </div>
                </form>
            </div>
        `;
    }

    function renderKeyCreated(result) {
        return `
            <div class="max-w-2xl">
                <div class="card">
                    <div class="text-center mb-6">
                        <div class="text-green-500 text-5xl mb-4">&#10003;</div>
                        <h2 class="text-xl font-semibold">API Key Created</h2>
                    </div>

                    <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-4 mb-6">
                        <p class="text-yellow-800 text-sm font-medium mb-2">
                            Important: Copy your API key now. You won't be able to see it again!
                        </p>
                        <div class="flex items-center gap-2">
                            <input type="text" id="secret-key" value="${Utils.escapeHtml(result.secret)}"
                                class="form-input font-mono text-sm flex-1" readonly>
                            <button id="copy-btn" class="btn btn-primary">Copy</button>
                        </div>
                    </div>

                    <div class="text-center">
                        <button id="done-btn" class="btn btn-secondary">Done</button>
                    </div>
                </div>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-key-btn').on('click', () => showForm());

        $('.suspend-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Are you sure you want to suspend this API key?')) {
                try {
                    await API.suspendApiKey(id);
                    Utils.showToast('API key suspended', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });

        $('.activate-btn').on('click', async function() {
            const id = $(this).data('id');

            try {
                await API.activateApiKey(id);
                Utils.showToast('API key activated', 'success');
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
            }
        });

        $('.revoke-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Are you sure you want to revoke this API key? This action cannot be undone.')) {
                try {
                    await API.revokeApiKey(id);
                    Utils.showToast('API key revoked', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });

        $('.delete-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Are you sure you want to delete this API key?')) {
                try {
                    await API.deleteApiKey(id);
                    Utils.showToast('API key deleted', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });
    }

    function showForm() {
        $('#content').html(renderForm());
        bindFormEvents();
    }

    function bindFormEvents() {
        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#key-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            // Structure permissions correctly
            const permissions = {
                admin: formData.permissions?.admin || false,
                models: formData.permissions?.models || 'none',
                prompts: formData.permissions?.prompts || 'none',
                knowledge_bases: formData.permissions?.knowledge_bases || 'none',
                chains: formData.permissions?.chains || 'none'
            };

            const data = {
                name: formData.name,
                team_id: formData.team_id,
                description: formData.description,
                permissions: permissions
            };

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Creating...');

            try {
                const result = await API.createApiKey(data);
                $('#content').html(renderKeyCreated(result));
                bindKeyCreatedEvents();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    function bindKeyCreatedEvents() {
        $('#copy-btn').on('click', function() {
            const $input = $('#secret-key');
            $input.select();
            document.execCommand('copy');
            Utils.showToast('API key copied to clipboard', 'success');
        });

        $('#done-btn').on('click', () => render());
    }

    return { render };
})();
