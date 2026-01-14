/**
 * Execution logs view with filtering and statistics
 */
const ExecutionLogs = (function() {
    let currentFilters = {};

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const [logsData, statsData] = await Promise.all([
                API.listExecutionLogs(currentFilters),
                API.getExecutionStats(currentFilters)
            ]);

            $('#content').html(renderContent(logsData, statsData));
            bindEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderContent(logsData, statsData) {
        const logs = logsData.logs || [];
        const stats = statsData;

        return `
            ${renderStats(stats)}
            ${renderFilters()}
            ${renderLogsList(logs, logsData.total)}
        `;
    }

    function renderStats(stats) {
        const successRate = stats.success_rate.toFixed(1);
        const avgTime = stats.avg_execution_time_ms.toFixed(0);
        const totalCost = (stats.total_cost_micros / 1000000).toFixed(4);

        return `
            <div class="grid grid-cols-4 gap-4 mb-6">
                <div class="card text-center">
                    <p class="text-3xl font-bold text-blue-600">${stats.total_executions}</p>
                    <p class="text-sm text-gray-500">Total Executions</p>
                </div>
                <div class="card text-center">
                    <p class="text-3xl font-bold text-green-600">${successRate}%</p>
                    <p class="text-sm text-gray-500">Success Rate</p>
                </div>
                <div class="card text-center">
                    <p class="text-3xl font-bold text-purple-600">${avgTime}ms</p>
                    <p class="text-sm text-gray-500">Avg Response Time</p>
                </div>
                <div class="card text-center">
                    <p class="text-3xl font-bold text-orange-600">$${totalCost}</p>
                    <p class="text-sm text-gray-500">Total Cost</p>
                </div>
            </div>

            <div class="grid grid-cols-2 gap-4 mb-6">
                <div class="card">
                    <h3 class="font-semibold mb-3">Token Usage</h3>
                    <div class="flex justify-between text-sm">
                        <span class="text-gray-600">Input Tokens:</span>
                        <span class="font-mono">${stats.total_input_tokens.toLocaleString()}</span>
                    </div>
                    <div class="flex justify-between text-sm mt-1">
                        <span class="text-gray-600">Output Tokens:</span>
                        <span class="font-mono">${stats.total_output_tokens.toLocaleString()}</span>
                    </div>
                </div>
                <div class="card">
                    <h3 class="font-semibold mb-3">Execution Summary</h3>
                    <div class="flex justify-between text-sm">
                        <span class="text-gray-600">Successful:</span>
                        <span class="font-mono text-green-600">${stats.successful_executions}</span>
                    </div>
                    <div class="flex justify-between text-sm mt-1">
                        <span class="text-gray-600">Failed:</span>
                        <span class="font-mono text-red-600">${stats.failed_executions}</span>
                    </div>
                </div>
            </div>
        `;
    }

    function renderFilters() {
        return `
            <div class="card mb-6">
                <div class="flex items-center gap-4 flex-wrap">
                    <div class="flex items-center gap-2">
                        <label class="text-sm text-gray-600">Type:</label>
                        <select id="filter-type" class="form-input w-40">
                            <option value="">All</option>
                            <option value="model" ${currentFilters.execution_type === 'model' ? 'selected' : ''}>Model</option>
                            <option value="workflow" ${currentFilters.execution_type === 'workflow' ? 'selected' : ''}>Workflow</option>
                            <option value="chat_completion" ${currentFilters.execution_type === 'chat_completion' ? 'selected' : ''}>Chat Completion</option>
                        </select>
                    </div>
                    <div class="flex items-center gap-2">
                        <label class="text-sm text-gray-600">Status:</label>
                        <select id="filter-status" class="form-input w-32">
                            <option value="">All</option>
                            <option value="success" ${currentFilters.status === 'success' ? 'selected' : ''}>Success</option>
                            <option value="failed" ${currentFilters.status === 'failed' ? 'selected' : ''}>Failed</option>
                            <option value="timeout" ${currentFilters.status === 'timeout' ? 'selected' : ''}>Timeout</option>
                            <option value="cancelled" ${currentFilters.status === 'cancelled' ? 'selected' : ''}>Cancelled</option>
                        </select>
                    </div>
                    <div class="flex items-center gap-2">
                        <label class="text-sm text-gray-600">Resource:</label>
                        <input type="text" id="filter-resource" class="form-input w-40" placeholder="Resource ID"
                            value="${currentFilters.resource_id || ''}">
                    </div>
                    <button id="apply-filters-btn" class="btn btn-primary">Apply</button>
                    <button id="clear-filters-btn" class="btn btn-secondary">Clear</button>
                    <div class="flex-1"></div>
                    <button id="cleanup-btn" class="btn btn-delete">Cleanup Old Logs</button>
                </div>
            </div>
        `;
    }

    function renderLogsList(logs, total) {
        return `
            <div class="flex justify-between items-center mb-4">
                <p class="text-gray-600">Showing ${logs.length} of ${total} logs</p>
            </div>

            ${logs.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>Time</th>
                                <th>Type</th>
                                <th>Resource</th>
                                <th>Mode</th>
                                <th>Status</th>
                                <th>Duration</th>
                                <th>Tokens</th>
                                <th>Cost</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${logs.map(renderLogRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No execution logs found')}
        `;
    }

    function renderLogRow(log) {
        const statusClass = getStatusClass(log.status);
        const tokens = log.token_usage ? log.token_usage.total_tokens : '-';
        const cost = log.cost_micros ? `$${(log.cost_micros / 1000000).toFixed(6)}` : '-';
        const modeClass = log.is_async ? 'badge-purple' : 'badge-blue';
        const modeText = log.is_async ? 'Async' : 'Sync';

        return `
            <tr>
                <td class="text-sm">${Utils.formatDate(log.created_at)}</td>
                <td><span class="badge badge-gray">${Utils.escapeHtml(log.execution_type)}</span></td>
                <td class="font-mono text-sm">
                    ${Utils.escapeHtml(log.resource_name || log.resource_id)}
                </td>
                <td><span class="badge ${modeClass}">${modeText}</span></td>
                <td><span class="badge ${statusClass}">${Utils.escapeHtml(log.status)}</span></td>
                <td class="font-mono text-sm">${log.execution_time_ms}ms</td>
                <td class="font-mono text-sm">${tokens}</td>
                <td class="font-mono text-sm">${cost}</td>
                <td>
                    <button class="view-btn btn-sm btn-edit" data-id="${Utils.escapeHtml(log.id)}">View</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(log.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function getStatusClass(status) {
        switch (status.toLowerCase()) {
            case 'success': return 'badge-success';
            case 'failed': return 'badge-error';
            case 'timeout': return 'badge-warning';
            case 'cancelled': return 'badge-gray';
            default: return 'badge-gray';
        }
    }

    function bindEvents() {
        $('#apply-filters-btn').on('click', applyFilters);
        $('#clear-filters-btn').on('click', clearFilters);
        $('#cleanup-btn').on('click', showCleanupModal);

        $('.view-btn').on('click', function() {
            const id = $(this).data('id');
            showLogDetails(id);
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });
    }

    function applyFilters() {
        currentFilters = {};

        const type = $('#filter-type').val();
        const status = $('#filter-status').val();
        const resource = $('#filter-resource').val();

        if (type) currentFilters.execution_type = type;

        if (status) currentFilters.status = status;

        if (resource) currentFilters.resource_id = resource;

        currentFilters.limit = 100;
        render();
    }

    function clearFilters() {
        currentFilters = {};
        render();
    }

    async function showLogDetails(id) {
        try {
            const log = await API.getExecutionLog(id);
            const modal = renderLogDetailsModal(log);
            $('body').append(modal);
            bindModalEvents();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    function renderLogDetailsModal(log) {
        const modeClass = log.is_async ? 'badge-purple' : 'badge-blue';
        const modeText = log.is_async ? 'Async' : 'Sync';

        return `
            <div id="details-modal" class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
                <div class="bg-white rounded-lg shadow-xl p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto">
                    <div class="flex justify-between items-center mb-4">
                        <h3 class="text-lg font-semibold">Execution Log Details</h3>
                        <button id="close-modal-btn" class="text-gray-500 hover:text-gray-700 text-2xl">&times;</button>
                    </div>

                    <div class="grid grid-cols-3 gap-4 mb-4">
                        <div>
                            <label class="text-sm text-gray-500">ID</label>
                            <p class="font-mono text-sm">${Utils.escapeHtml(log.id)}</p>
                        </div>
                        <div>
                            <label class="text-sm text-gray-500">Created At</label>
                            <p class="text-sm">${Utils.formatDate(log.created_at)}</p>
                        </div>
                        <div>
                            <label class="text-sm text-gray-500">Execution Mode</label>
                            <p><span class="badge ${modeClass}">${modeText}</span></p>
                        </div>
                        <div>
                            <label class="text-sm text-gray-500">Execution Type</label>
                            <p><span class="badge badge-gray">${Utils.escapeHtml(log.execution_type)}</span></p>
                        </div>
                        <div>
                            <label class="text-sm text-gray-500">Status</label>
                            <p><span class="badge ${getStatusClass(log.status)}">${Utils.escapeHtml(log.status)}</span></p>
                        </div>
                        <div>
                            <label class="text-sm text-gray-500">Resource</label>
                            <p class="font-mono text-sm">${Utils.escapeHtml(log.resource_name || log.resource_id)}</p>
                        </div>
                        <div>
                            <label class="text-sm text-gray-500">Execution Time</label>
                            <p class="font-mono text-sm">${log.execution_time_ms}ms</p>
                        </div>
                    </div>

                    ${log.token_usage ? `
                        <div class="border-t pt-4 mb-4">
                            <h4 class="font-medium mb-2">Token Usage</h4>
                            <div class="grid grid-cols-3 gap-4">
                                <div>
                                    <label class="text-sm text-gray-500">Input</label>
                                    <p class="font-mono">${log.token_usage.input_tokens}</p>
                                </div>
                                <div>
                                    <label class="text-sm text-gray-500">Output</label>
                                    <p class="font-mono">${log.token_usage.output_tokens}</p>
                                </div>
                                <div>
                                    <label class="text-sm text-gray-500">Total</label>
                                    <p class="font-mono">${log.token_usage.total_tokens}</p>
                                </div>
                            </div>
                        </div>
                    ` : ''}

                    ${log.cost_micros ? `
                        <div class="border-t pt-4 mb-4">
                            <h4 class="font-medium mb-2">Cost</h4>
                            <p class="font-mono text-lg">$${(log.cost_micros / 1000000).toFixed(6)}</p>
                        </div>
                    ` : ''}

                    ${log.input ? `
                        <div class="border-t pt-4 mb-4">
                            <h4 class="font-medium mb-2">Input</h4>
                            <pre class="bg-gray-100 p-3 rounded text-sm overflow-x-auto max-h-48">${Utils.escapeHtml(JSON.stringify(log.input, null, 2))}</pre>
                        </div>
                    ` : ''}

                    ${log.output ? `
                        <div class="border-t pt-4 mb-4">
                            <h4 class="font-medium mb-2">Output</h4>
                            <pre class="bg-gray-100 p-3 rounded text-sm overflow-x-auto max-h-48">${Utils.escapeHtml(JSON.stringify(log.output, null, 2))}</pre>
                        </div>
                    ` : ''}

                    ${log.workflow_steps && log.workflow_steps.length > 0 ? `
                        <div class="border-t pt-4 mb-4">
                            <h4 class="font-medium mb-2">Workflow Steps</h4>
                            <div class="space-y-3">
                                ${log.workflow_steps.map((step, index) => `
                                    <div class="border rounded p-3 ${step.status === 'success' ? 'border-green-200 bg-green-50' : 'border-red-200 bg-red-50'}">
                                        <div class="flex justify-between items-center mb-2">
                                            <span class="font-medium">${index + 1}. ${Utils.escapeHtml(step.step_name)}</span>
                                            <div class="flex items-center gap-2">
                                                <span class="badge badge-gray text-xs">${Utils.escapeHtml(step.step_type)}</span>
                                                <span class="badge ${getStatusClass(step.status)} text-xs">${Utils.escapeHtml(step.status)}</span>
                                                <span class="text-xs text-gray-500">${step.execution_time_ms}ms</span>
                                            </div>
                                        </div>
                                        ${step.input ? `
                                            <details class="mb-2">
                                                <summary class="text-xs text-gray-600 cursor-pointer hover:text-gray-800">Input</summary>
                                                <pre class="bg-white p-2 rounded text-xs mt-1 overflow-x-auto max-h-32">${Utils.escapeHtml(JSON.stringify(step.input, null, 2))}</pre>
                                            </details>
                                        ` : ''}
                                        ${step.output ? `
                                            <details class="mb-2">
                                                <summary class="text-xs text-gray-600 cursor-pointer hover:text-gray-800">Output</summary>
                                                <pre class="bg-white p-2 rounded text-xs mt-1 overflow-x-auto max-h-32">${Utils.escapeHtml(JSON.stringify(step.output, null, 2))}</pre>
                                            </details>
                                        ` : ''}
                                        ${step.error ? `
                                            <div class="text-xs text-red-600 mt-1">Error: ${Utils.escapeHtml(step.error)}</div>
                                        ` : ''}
                                    </div>
                                `).join('')}
                            </div>
                        </div>
                    ` : ''}

                    <div class="border-t pt-4 mb-4">
                        <h4 class="font-medium mb-2">Executor</h4>
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="text-sm text-gray-500">User ID</label>
                                <p class="font-mono text-sm">${Utils.escapeHtml(log.executor.user_id || '-')}</p>
                            </div>
                            <div>
                                <label class="text-sm text-gray-500">API Key ID</label>
                                <p class="font-mono text-sm">${Utils.escapeHtml(log.executor.api_key_id || '-')}</p>
                            </div>
                            <div>
                                <label class="text-sm text-gray-500">IP Address</label>
                                <p class="font-mono text-sm">${Utils.escapeHtml(log.executor.ip_address || '-')}</p>
                            </div>
                            <div>
                                <label class="text-sm text-gray-500">User Agent</label>
                                <p class="text-sm truncate">${Utils.escapeHtml(log.executor.user_agent || '-')}</p>
                            </div>
                        </div>
                    </div>

                    ${log.error ? `
                        <div class="border-t pt-4">
                            <h4 class="font-medium mb-2 text-red-600">Error</h4>
                            <pre class="bg-red-50 text-red-800 p-4 rounded text-sm overflow-x-auto">${Utils.escapeHtml(log.error)}</pre>
                        </div>
                    ` : ''}
                </div>
            </div>
        `;
    }

    function bindModalEvents() {
        $('#close-modal-btn').on('click', () => $('#details-modal').remove());
    }

    async function confirmDelete(id) {
        if (!Utils.confirm('Are you sure you want to delete this execution log?')) {
            return;
        }

        try {
            await API.deleteExecutionLog(id);
            Utils.showToast('Execution log deleted', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    function showCleanupModal() {
        const modal = `
            <div id="cleanup-modal" class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
                <div class="bg-white rounded-lg shadow-xl p-6 w-full max-w-md">
                    <h3 class="text-lg font-semibold mb-4">Cleanup Old Logs</h3>
                    <form id="cleanup-form">
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Delete logs older than (days)</label>
                            <input type="number" name="days" value="30" min="1" max="365" class="form-input">
                            <p class="text-xs text-gray-500 mt-1">Leave empty to use default retention period from configuration</p>
                        </div>
                        <div class="flex justify-end gap-3 mt-6">
                            <button type="button" id="cancel-cleanup-btn" class="btn btn-secondary">Cancel</button>
                            <button type="submit" class="btn btn-delete">Delete Old Logs</button>
                        </div>
                    </form>
                </div>
            </div>
        `;

        $('body').append(modal);

        $('#cancel-cleanup-btn').on('click', () => $('#cleanup-modal').remove());

        $('#cleanup-form').on('submit', async function(e) {
            e.preventDefault();

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Deleting...');

            try {
                const days = parseInt($('[name="days"]').val(), 10);
                const result = await API.cleanupExecutionLogs({ days: days || null });
                Utils.showToast(`Deleted ${result.deleted_count} old logs`, 'success');
                $('#cleanup-modal').remove();
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    return { render };
})();
