/**
 * Budgets CRUD view
 */
const Budgets = (function() {
    let teamsCache = [];
    let apiKeysCache = [];

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const [budgetsData, teamsData, apiKeysData] = await Promise.all([
                API.listBudgets(),
                API.listTeams(),
                API.listApiKeys()
            ]);

            teamsCache = teamsData.teams || [];
            apiKeysCache = apiKeysData.api_keys || [];

            $('#content').html(renderList(budgetsData.budgets || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(budgets) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${budgets.length} budget(s)</p>
                <button id="create-budget-btn" class="btn btn-primary">+ New Budget</button>
            </div>

            ${budgets.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Period</th>
                                <th>Scope</th>
                                <th>Usage</th>
                                <th>Status</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${budgets.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No budgets configured yet')}
        `;
    }

    function renderRow(budget) {
        const statusColors = {
            active: 'badge-success',
            warning: 'badge-warning',
            exceeded: 'badge-error',
            paused: 'badge-gray'
        };

        const scopeLabels = {
            all_api_keys: 'All API Keys',
            specific_api_keys: 'Specific Keys',
            teams: 'Teams',
            mixed: 'Mixed'
        };

        const usagePercent = budget.usage_percent.toFixed(1);
        const usageColor = budget.usage_percent >= 90 ? 'text-red-600' :
                          budget.usage_percent >= 75 ? 'text-yellow-600' : 'text-green-600';

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(budget.id)}</td>
                <td>
                    <div class="font-medium">${Utils.escapeHtml(budget.name)}</div>
                    ${budget.description ? `<div class="text-xs text-gray-500">${Utils.escapeHtml(budget.description)}</div>` : ''}
                </td>
                <td class="capitalize">${budget.period}</td>
                <td>
                    <span class="text-sm">${scopeLabels[budget.scope] || budget.scope}</span>
                    ${renderScopeDetails(budget)}
                </td>
                <td>
                    <div class="flex items-center gap-2">
                        <div class="w-24 bg-gray-200 rounded-full h-2">
                            <div class="h-2 rounded-full ${usageColor.replace('text-', 'bg-')}"
                                 style="width: ${Math.min(budget.usage_percent, 100)}%"></div>
                        </div>
                        <span class="${usageColor} text-sm font-medium">${usagePercent}%</span>
                    </div>
                    <div class="text-xs text-gray-500 mt-1">
                        $${budget.current_usage_usd.toFixed(2)} / $${budget.hard_limit_usd.toFixed(2)}
                    </div>
                </td>
                <td>
                    <span class="badge ${statusColors[budget.status] || 'badge-gray'}">${budget.status}</span>
                    ${!budget.enabled ? '<span class="badge badge-gray ml-1">disabled</span>' : ''}
                </td>
                <td>
                    <div class="flex items-center gap-1">
                        <button class="edit-btn btn-sm btn-secondary-sm" data-id="${Utils.escapeHtml(budget.id)}">Edit</button>
                        <button class="reset-btn btn-sm btn-warning-sm" data-id="${Utils.escapeHtml(budget.id)}" title="Reset current period">Reset</button>
                        <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(budget.id)}">Delete</button>
                    </div>
                </td>
            </tr>
        `;
    }

    function renderScopeDetails(budget) {
        const details = [];

        if (budget.team_ids?.length > 0) {
            details.push(`<div class="text-xs text-gray-500">${budget.team_ids.length} team(s)</div>`);
        }

        if (budget.api_key_ids?.length > 0) {
            details.push(`<div class="text-xs text-gray-500">${budget.api_key_ids.length} key(s)</div>`);
        }

        return details.join('');
    }

    function renderForm(budget = null) {
        const isEdit = !!budget;
        const title = isEdit ? 'Edit Budget' : 'Create Budget';

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="budget-form" class="card">
                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                            <input type="text" name="id" class="form-input"
                                placeholder="monthly-budget"
                                value="${isEdit ? Utils.escapeHtml(budget.id) : ''}"
                                ${isEdit ? 'readonly' : 'required'}
                                pattern="^[a-z][a-z0-9-]*[a-z0-9]$"
                                title="Lowercase letters, numbers, and hyphens only">
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                            <input type="text" name="name" class="form-input"
                                placeholder="Monthly Budget"
                                value="${isEdit ? Utils.escapeHtml(budget.name) : ''}"
                                required>
                        </div>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <textarea name="description" class="form-input" rows="2"
                            placeholder="Optional description">${isEdit && budget.description ? Utils.escapeHtml(budget.description) : ''}</textarea>
                    </div>

                    <div class="grid grid-cols-3 gap-4 mb-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Period</label>
                            <select name="period" class="form-input" ${isEdit ? 'disabled' : 'required'}>
                                <option value="daily" ${budget?.period === 'daily' ? 'selected' : ''}>Daily</option>
                                <option value="weekly" ${budget?.period === 'weekly' ? 'selected' : ''}>Weekly</option>
                                <option value="monthly" ${!budget || budget?.period === 'monthly' ? 'selected' : ''}>Monthly</option>
                                <option value="lifetime" ${budget?.period === 'lifetime' ? 'selected' : ''}>Lifetime</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Hard Limit (USD)</label>
                            <input type="number" name="hard_limit_usd" class="form-input"
                                placeholder="100.00" step="0.01" min="0.01"
                                value="${isEdit ? budget.hard_limit_usd : ''}"
                                required>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Soft Limit (USD)</label>
                            <input type="number" name="soft_limit_usd" class="form-input"
                                placeholder="80.00 (optional)" step="0.01" min="0"
                                value="${isEdit && budget.soft_limit_usd ? budget.soft_limit_usd : ''}">
                            <p class="text-xs text-gray-500 mt-1">Warning threshold</p>
                        </div>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Alert Thresholds (%)</label>
                        <input type="text" name="alert_thresholds" class="form-input"
                            placeholder="50, 75, 90"
                            value="${isEdit && budget.alerts?.length ? budget.alerts.map(a => a.threshold_percent).join(', ') : '50, 75, 90'}">
                        <p class="text-xs text-gray-500 mt-1">Comma-separated percentages</p>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Scope</h3>

                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Teams</label>
                            <select name="team_ids" class="form-input" multiple size="4">
                                ${teamsCache.map(team => `
                                    <option value="${Utils.escapeHtml(team.id)}"
                                        ${budget?.team_ids?.includes(team.id) ? 'selected' : ''}>
                                        ${Utils.escapeHtml(team.name)} (${team.id})
                                    </option>
                                `).join('')}
                            </select>
                            <p class="text-xs text-gray-500 mt-1">Hold Ctrl/Cmd to select multiple. Leave empty for all teams.</p>
                        </div>

                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">API Keys</label>
                            <select name="api_key_ids" class="form-input" multiple size="4">
                                ${apiKeysCache.map(key => `
                                    <option value="${Utils.escapeHtml(key.id)}"
                                        ${budget?.api_key_ids?.includes(key.id) ? 'selected' : ''}>
                                        ${Utils.escapeHtml(key.name)} (${key.id})
                                    </option>
                                `).join('')}
                            </select>
                            <p class="text-xs text-gray-500 mt-1">Hold Ctrl/Cmd to select multiple. Leave empty for all keys.</p>
                        </div>
                    </div>

                    <div class="mb-4">
                        <label class="flex items-center">
                            <input type="checkbox" name="enabled" class="mr-2"
                                ${!budget || budget.enabled ? 'checked' : ''}>
                            <span class="text-sm font-medium text-gray-700">Enabled</span>
                        </label>
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
        $('#create-budget-btn').on('click', () => showForm());

        $('.edit-btn').on('click', async function() {
            const id = $(this).data('id');

            try {
                const budget = await API.getBudget(id);
                showForm(budget);
            } catch (error) {
                Utils.showToast(error.message, 'error');
            }
        });

        $('.reset-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Reset the current period usage to $0.00? This cannot be undone.')) {
                try {
                    await API.resetBudget(id);
                    Utils.showToast('Budget period reset', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });

        $('.delete-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Delete this budget? This cannot be undone.')) {
                try {
                    await API.deleteBudget(id);
                    Utils.showToast('Budget deleted', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });
    }

    function showForm(budget = null) {
        $('#content').html(renderForm(budget));
        bindFormEvents(budget);
    }

    function bindFormEvents(budget = null) {
        const isEdit = !!budget;

        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#budget-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            // Parse alert thresholds
            const alertThresholds = formData.alert_thresholds
                ? formData.alert_thresholds.split(',').map(s => parseInt(s.trim())).filter(n => !isNaN(n) && n > 0 && n <= 100)
                : [];

            // Get selected teams and API keys
            const teamIds = Array.from($('select[name="team_ids"]').find(':selected')).map(opt => opt.value);
            const apiKeyIds = Array.from($('select[name="api_key_ids"]').find(':selected')).map(opt => opt.value);

            const data = {
                name: formData.name,
                description: formData.description || null,
                hard_limit_usd: parseFloat(formData.hard_limit_usd),
                soft_limit_usd: formData.soft_limit_usd ? parseFloat(formData.soft_limit_usd) : null,
                alert_thresholds: alertThresholds.length > 0 ? alertThresholds : null,
                team_ids: teamIds.length > 0 ? teamIds : null,
                api_key_ids: apiKeyIds.length > 0 ? apiKeyIds : null,
                enabled: formData.enabled === 'on'
            };

            if (!isEdit) {
                data.id = formData.id;
                data.period = formData.period;
            }

            // Validate soft limit < hard limit
            if (data.soft_limit_usd && data.soft_limit_usd >= data.hard_limit_usd) {
                Utils.showToast('Soft limit must be less than hard limit', 'error');
                return;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text(isEdit ? 'Updating...' : 'Creating...');

            try {
                if (isEdit) {
                    await API.updateBudget(budget.id, data);
                    Utils.showToast('Budget updated', 'success');
                } else {
                    await API.createBudget(data);
                    Utils.showToast('Budget created', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    return { render };
})();
