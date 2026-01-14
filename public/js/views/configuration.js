/**
 * Configuration management view
 */
const Configuration = (function() {
    const categories = [
        { value: 'general', label: 'General' },
        { value: 'persistence', label: 'Persistence' },
        { value: 'logging', label: 'Logging' },
        { value: 'security', label: 'Security' },
        { value: 'cache', label: 'Cache' },
        { value: 'rate_limit', label: 'Rate Limit' }
    ];

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listConfig();
            $('#content').html(renderList(data.config || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(config) {
        const grouped = groupByCategory(config);

        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${config.length} configuration entries</p>
                <button id="reset-config-btn" class="btn btn-secondary">Reset to Defaults</button>
            </div>

            ${categories.map(cat => renderCategorySection(cat, grouped[cat.value] || [])).join('')}
        `;
    }

    function groupByCategory(config) {
        const grouped = {};

        for (const entry of config) {
            const cat = entry.category.toLowerCase();

            if (!grouped[cat]) {
                grouped[cat] = [];
            }
            grouped[cat].push(entry);
        }
        return grouped;
    }

    function renderCategorySection(category, entries) {
        if (entries.length === 0) return '';

        return `
            <div class="card mb-4">
                <h3 class="font-semibold text-lg mb-4">${category.label}</h3>
                <div class="space-y-4">
                    ${entries.map(renderConfigEntry).join('')}
                </div>
            </div>
        `;
    }

    function renderConfigEntry(entry) {
        const valueDisplay = formatValue(entry.value);
        const typeLabel = entry.value.type;

        return `
            <div class="flex items-start justify-between border-b pb-4 last:border-b-0 last:pb-0">
                <div class="flex-1">
                    <div class="flex items-center gap-2">
                        <span class="font-mono text-sm font-medium">${Utils.escapeHtml(entry.key)}</span>
                        <span class="badge badge-gray text-xs">${typeLabel}</span>
                    </div>
                    ${entry.description ? `<p class="text-sm text-gray-500 mt-1">${Utils.escapeHtml(entry.description)}</p>` : ''}
                    <p class="text-xs text-gray-400 mt-1">Updated: ${Utils.formatDate(entry.updated_at)}</p>
                </div>
                <div class="flex items-center gap-2 ml-4">
                    <span class="font-mono text-sm bg-gray-100 px-2 py-1 rounded">${valueDisplay}</span>
                    <button class="edit-btn btn-sm btn-edit" data-key="${Utils.escapeHtml(entry.key)}"
                            data-type="${entry.value.type}" data-value="${Utils.escapeHtml(JSON.stringify(entry.value.value))}">
                        Edit
                    </button>
                </div>
            </div>
        `;
    }

    function formatValue(value) {
        switch (value.type) {
            case 'string':
                return `"${Utils.escapeHtml(value.value)}"`;
            case 'integer':
            case 'float':
                return value.value.toString();
            case 'boolean':
                return value.value ? 'true' : 'false';
            case 'string_list':
                return `[${value.value.map(s => `"${Utils.escapeHtml(s)}"`).join(', ')}]`;
            default:
                return JSON.stringify(value.value);
        }
    }

    function bindListEvents() {
        $('#reset-config-btn').on('click', confirmReset);

        $('.edit-btn').on('click', function() {
            const key = $(this).data('key');
            const type = $(this).data('type');
            const value = $(this).data('value');
            showEditModal(key, type, value);
        });
    }

    function showEditModal(key, type, currentValue) {
        const inputHtml = renderValueInput(type, currentValue);

        const modal = `
            <div id="edit-modal" class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
                <div class="bg-white rounded-lg shadow-xl p-6 w-full max-w-md">
                    <h3 class="text-lg font-semibold mb-4">Edit Configuration</h3>
                    <form id="edit-config-form">
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Key</label>
                            <input type="text" value="${Utils.escapeHtml(key)}" class="form-input bg-gray-100" readonly>
                        </div>
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Value (${type})</label>
                            ${inputHtml}
                        </div>
                        <div class="flex justify-end gap-3 mt-6">
                            <button type="button" id="cancel-edit-btn" class="btn btn-secondary">Cancel</button>
                            <button type="submit" class="btn btn-primary">Save</button>
                        </div>
                    </form>
                </div>
            </div>
        `;

        $('body').append(modal);
        bindModalEvents(key, type);
    }

    function renderValueInput(type, currentValue) {
        switch (type) {
            case 'string':
                return `<input type="text" name="value" value="${Utils.escapeHtml(currentValue)}" class="form-input">`;
            case 'integer':
                return `<input type="number" name="value" value="${currentValue}" class="form-input" step="1">`;
            case 'float':
                return `<input type="number" name="value" value="${currentValue}" class="form-input" step="0.01">`;
            case 'boolean':
                const checked = currentValue === true || currentValue === 'true' ? 'checked' : '';
                return `
                    <select name="value" class="form-input">
                        <option value="true" ${currentValue ? 'selected' : ''}>true</option>
                        <option value="false" ${!currentValue ? 'selected' : ''}>false</option>
                    </select>
                `;
            case 'string_list':
                const list = Array.isArray(currentValue) ? currentValue : JSON.parse(currentValue);
                return `
                    <textarea name="value" class="form-input h-32" placeholder="One value per line">${list.join('\n')}</textarea>
                    <p class="text-xs text-gray-500 mt-1">Enter one value per line</p>
                `;
            default:
                return `<input type="text" name="value" value="${Utils.escapeHtml(JSON.stringify(currentValue))}" class="form-input">`;
        }
    }

    function bindModalEvents(key, type) {
        $('#cancel-edit-btn').on('click', () => $('#edit-modal').remove());

        $('#edit-config-form').on('submit', async function(e) {
            e.preventDefault();

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                const rawValue = $('[name="value"]').val();
                const value = parseValue(type, rawValue);

                await API.updateConfig(key, { value });
                Utils.showToast('Configuration updated', 'success');
                $('#edit-modal').remove();
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    function parseValue(type, rawValue) {
        switch (type) {
            case 'string':
                return { type: 'string', value: rawValue };
            case 'integer':
                return { type: 'integer', value: parseInt(rawValue, 10) };
            case 'float':
                return { type: 'float', value: parseFloat(rawValue) };
            case 'boolean':
                return { type: 'boolean', value: rawValue === 'true' };
            case 'string_list':
                const lines = rawValue.split('\n').map(s => s.trim()).filter(s => s.length > 0);
                return { type: 'string_list', value: lines };
            default:
                return { type: 'string', value: rawValue };
        }
    }

    async function confirmReset() {
        if (!Utils.confirm('Are you sure you want to reset all configuration to defaults? This cannot be undone.')) {
            return;
        }

        try {
            await API.resetConfig();
            Utils.showToast('Configuration reset to defaults', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
