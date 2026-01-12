/**
 * Webhooks CRUD view
 */
const Webhooks = (function() {
    let eventTypes = [];

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const [webhooksData, eventTypesData] = await Promise.all([
                API.listWebhooks(),
                API.listWebhookEventTypes()
            ]);
            eventTypes = eventTypesData.event_types || [];
            $('#content').html(renderList(webhooksData.webhooks || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(webhooks) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${webhooks.length} webhook(s)</p>
                <button id="create-webhook-btn" class="btn btn-primary">+ New Webhook</button>
            </div>

            ${webhooks.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>Name</th>
                                <th>URL</th>
                                <th>Events</th>
                                <th>Status</th>
                                <th>Failures</th>
                                <th>Last Activity</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${webhooks.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No webhooks configured yet')}
        `;
    }

    function renderRow(webhook) {
        const statusColors = {
            active: 'badge-success',
            disabled: 'badge-gray',
            failing: 'badge-warning'
        };

        const truncatedUrl = webhook.url.length > 50
            ? webhook.url.substring(0, 47) + '...'
            : webhook.url;

        const lastActivity = webhook.last_success_at || webhook.last_failure_at;

        return `
            <tr>
                <td>
                    <div class="font-medium">${Utils.escapeHtml(webhook.name)}</div>
                    ${webhook.description ? `<div class="text-xs text-gray-500">${Utils.escapeHtml(webhook.description)}</div>` : ''}
                    <div class="text-xs text-gray-400 font-mono">${Utils.escapeHtml(webhook.id)}</div>
                </td>
                <td>
                    <div class="font-mono text-sm" title="${Utils.escapeHtml(webhook.url)}">${Utils.escapeHtml(truncatedUrl)}</div>
                    ${webhook.has_secret ? '<span class="text-xs text-green-600">Signed</span>' : ''}
                </td>
                <td class="text-sm">
                    ${webhook.events.map(e => `<span class="inline-block bg-gray-100 rounded px-1 mr-1 mb-1 text-xs">${e}</span>`).join('')}
                </td>
                <td><span class="badge ${statusColors[webhook.status] || 'badge-gray'}">${webhook.status}</span></td>
                <td class="text-sm ${webhook.failure_count > 0 ? 'text-red-600' : 'text-gray-500'}">${webhook.failure_count}/${webhook.max_retries}</td>
                <td class="text-sm text-gray-500">${lastActivity ? Utils.formatDate(lastActivity) : 'Never'}</td>
                <td>
                    <div class="flex items-center gap-1">
                        <button class="view-btn btn-sm btn-secondary-sm" data-id="${Utils.escapeHtml(webhook.id)}">View</button>
                        <button class="deliveries-btn btn-sm btn-secondary-sm" data-id="${Utils.escapeHtml(webhook.id)}">Deliveries</button>
                        ${webhook.status === 'failing' ? `
                            <button class="reset-btn btn-sm btn-warning-sm" data-id="${Utils.escapeHtml(webhook.id)}">Reset</button>
                        ` : ''}
                        <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(webhook.id)}">Delete</button>
                    </div>
                </td>
            </tr>
        `;
    }

    function renderForm(webhook = null) {
        const isEdit = !!webhook;
        const title = isEdit ? 'Edit Webhook' : 'Create Webhook';

        const eventCheckboxes = eventTypes.map(et => `
            <label class="flex items-start mb-2">
                <input type="checkbox" name="events" value="${Utils.escapeHtml(et.name)}"
                    class="mt-1 mr-2" ${webhook?.events?.includes(et.name) ? 'checked' : ''}>
                <div>
                    <span class="text-sm font-medium">${Utils.escapeHtml(et.name)}</span>
                    <div class="text-xs text-gray-500">${Utils.escapeHtml(et.description)}</div>
                </div>
            </label>
        `).join('');

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="webhook-form" class="card">
                    ${!isEdit ? `
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                        <input type="text" name="id" class="form-input font-mono"
                            placeholder="webhook-id" required
                            pattern="[a-zA-Z0-9_-]+" title="Only letters, numbers, hyphens and underscores">
                    </div>
                    ` : ''}

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" class="form-input"
                            placeholder="My Webhook" required
                            value="${isEdit ? Utils.escapeHtml(webhook.name) : ''}">
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <input type="text" name="description" class="form-input"
                            placeholder="Optional description"
                            value="${isEdit && webhook.description ? Utils.escapeHtml(webhook.description) : ''}">
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">URL</label>
                        <input type="url" name="url" class="form-input font-mono"
                            placeholder="https://example.com/webhook" required
                            value="${isEdit ? Utils.escapeHtml(webhook.url) : ''}">
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            Secret ${isEdit && webhook.has_secret ? '(leave blank to keep existing)' : '(optional)'}
                        </label>
                        <input type="password" name="secret" class="form-input font-mono"
                            placeholder="${isEdit && webhook.has_secret ? '********' : 'Optional signing secret'}">
                        <p class="text-xs text-gray-500 mt-1">Used to sign webhook payloads with HMAC-SHA256</p>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Events</h3>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
                            ${eventCheckboxes}
                        </div>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Retry Configuration</h3>
                        <div class="grid grid-cols-3 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">Max Retries</label>
                                <input type="number" name="max_retries" class="form-input"
                                    min="0" max="10" value="${isEdit ? webhook.max_retries : 3}">
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">Retry Delay (sec)</label>
                                <input type="number" name="retry_delay_secs" class="form-input"
                                    min="1" max="3600" value="${isEdit ? webhook.retry_delay_secs : 60}">
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">Timeout (sec)</label>
                                <input type="number" name="timeout_secs" class="form-input"
                                    min="1" max="300" value="${isEdit ? webhook.timeout_secs : 30}">
                            </div>
                        </div>
                    </div>

                    ${isEdit ? `
                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Status</h3>
                        <select name="status" class="form-input">
                            <option value="active" ${webhook.status === 'active' ? 'selected' : ''}>Active</option>
                            <option value="disabled" ${webhook.status === 'disabled' ? 'selected' : ''}>Disabled</option>
                        </select>
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

    function renderDeliveries(webhookId, deliveries) {
        const statusColors = {
            pending: 'badge-gray',
            success: 'badge-success',
            failed: 'badge-error',
            retrying: 'badge-warning'
        };

        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Webhook Deliveries</h2>
                </div>

                <div class="mb-4 text-sm text-gray-600">
                    Showing last ${deliveries.length} deliveries for webhook <span class="font-mono">${Utils.escapeHtml(webhookId)}</span>
                </div>

                ${deliveries.length > 0 ? `
                    <div class="card p-0 overflow-hidden">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>ID</th>
                                    <th>Event</th>
                                    <th>Status</th>
                                    <th>Attempts</th>
                                    <th>Response</th>
                                    <th>Created</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${deliveries.map(d => `
                                    <tr>
                                        <td class="font-mono text-xs">${Utils.escapeHtml(d.id)}</td>
                                        <td><span class="bg-gray-100 rounded px-1 text-xs">${Utils.escapeHtml(d.event_type)}</span></td>
                                        <td><span class="badge ${statusColors[d.status] || 'badge-gray'}">${d.status}</span></td>
                                        <td class="text-sm">${d.attempts}</td>
                                        <td class="text-sm">
                                            ${d.response_status ? `<span class="font-mono">${d.response_status}</span>` : ''}
                                            ${d.error_message ? `<span class="text-red-600 text-xs">${Utils.escapeHtml(d.error_message)}</span>` : ''}
                                        </td>
                                        <td class="text-sm text-gray-500">${Utils.formatDate(d.created_at)}</td>
                                    </tr>
                                `).join('')}
                            </tbody>
                        </table>
                    </div>
                ` : Utils.renderEmpty('No deliveries yet')}
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-webhook-btn').on('click', () => showForm());

        $('.view-btn').on('click', async function() {
            const id = $(this).data('id');

            try {
                const webhook = await API.getWebhook(id);
                showForm(webhook);
            } catch (error) {
                Utils.showToast(error.message, 'error');
            }
        });

        $('.deliveries-btn').on('click', async function() {
            const id = $(this).data('id');
            showDeliveries(id);
        });

        $('.reset-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Reset this webhook and re-enable it?')) {
                try {
                    await API.resetWebhook(id);
                    Utils.showToast('Webhook reset successfully', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });

        $('.delete-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Are you sure you want to delete this webhook?')) {
                try {
                    await API.deleteWebhook(id);
                    Utils.showToast('Webhook deleted', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });
    }

    function showForm(webhook = null) {
        $('#content').html(renderForm(webhook));
        bindFormEvents(webhook);
    }

    async function showDeliveries(webhookId) {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.getWebhookDeliveries(webhookId);
            $('#content').html(renderDeliveries(webhookId, data.deliveries || []));
            $('#back-btn').on('click', () => render());
        } catch (error) {
            Utils.showToast(error.message, 'error');
            render();
        }
    }

    function bindFormEvents(webhook = null) {
        const isEdit = !!webhook;

        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#webhook-form').on('submit', async function(e) {
            e.preventDefault();

            const events = [];
            $('input[name="events"]:checked').each(function() {
                events.push($(this).val());
            });

            if (events.length === 0) {
                Utils.showToast('Please select at least one event', 'error');
                return;
            }

            const data = {
                name: $('input[name="name"]').val(),
                description: $('input[name="description"]').val() || null,
                url: $('input[name="url"]').val(),
                events: events,
                max_retries: parseInt($('input[name="max_retries"]').val()) || 3,
                retry_delay_secs: parseInt($('input[name="retry_delay_secs"]').val()) || 60,
                timeout_secs: parseInt($('input[name="timeout_secs"]').val()) || 30
            };

            const secret = $('input[name="secret"]').val();

            if (secret) {
                data.secret = secret;
            }

            if (!isEdit) {
                data.id = $('input[name="id"]').val();
            } else {
                data.status = $('select[name="status"]').val();
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text(isEdit ? 'Updating...' : 'Creating...');

            try {
                if (isEdit) {
                    await API.updateWebhook(webhook.id, data);
                    Utils.showToast('Webhook updated', 'success');
                } else {
                    await API.createWebhook(data);
                    Utils.showToast('Webhook created', 'success');
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
