/**
 * External APIs View
 * Manages external API configurations (base URL and base headers for HTTP Request workflow steps)
 */
const ExternalApisView = (function() {
    let currentApis = [];

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listExternalApis();
            currentApis = data.external_apis || [];
            $('#content').html(renderList(currentApis));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(apis) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${apis.length} external API(s)</p>
                <button id="create-external-api-btn" class="btn btn-primary">+ New External API</button>
            </div>

            ${apis.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>Name / ID</th>
                                <th>Base URL</th>
                                <th>Headers</th>
                                <th>Status</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${apis.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No external APIs configured yet')}

            ${renderModal()}
        `;
    }

    function renderRow(api) {
        const headerCount = Object.keys(api.base_headers || {}).length;
        const statusClass = api.enabled ? 'badge-success' : 'badge-gray';
        const statusText = api.enabled ? 'Enabled' : 'Disabled';

        return `
            <tr>
                <td>
                    <div class="font-medium">${Utils.escapeHtml(api.name)}</div>
                    <div class="font-mono text-xs text-gray-500">${Utils.escapeHtml(api.id)}</div>
                </td>
                <td><code class="text-sm">${Utils.escapeHtml(api.base_url)}</code></td>
                <td class="text-center">
                    ${headerCount > 0
                        ? `<span class="badge badge-info">${headerCount} header${headerCount !== 1 ? 's' : ''}</span>`
                        : '<span class="text-gray-400">-</span>'}
                </td>
                <td><span class="badge ${statusClass}">${statusText}</span></td>
                <td>
                    <button class="edit-btn btn-sm btn-edit mr-2" data-id="${Utils.escapeHtml(api.id)}">Edit</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(api.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function renderModal() {
        return `
            <div id="external-api-modal" class="modal hidden">
                <div class="modal-backdrop"></div>
                <div class="modal-content max-w-lg">
                    <div class="modal-header">
                        <h3 id="external-api-modal-title" class="text-lg font-semibold">Create External API</h3>
                        <button class="close-modal text-gray-400 hover:text-gray-600">&times;</button>
                    </div>
                    <form id="external-api-form">
                        <div class="modal-body">
                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                                <input type="text" id="external-api-id" name="id"
                                    class="form-input" placeholder="my-external-api" required>
                                <p class="text-xs text-gray-500 mt-1">Alphanumeric, hyphens, and underscores only</p>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                                <input type="text" id="external-api-name" name="name"
                                    class="form-input" placeholder="My External API" required>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                                <textarea id="external-api-description" name="description"
                                    class="form-input" rows="2" placeholder="Optional description..."></textarea>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Base URL</label>
                                <input type="url" id="external-api-base-url" name="base_url"
                                    class="form-input" placeholder="https://api.example.com" required>
                                <p class="text-xs text-gray-500 mt-1">The base URL for all HTTP requests</p>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Base Headers</label>
                                <div id="base-headers-container">
                                    <div class="text-gray-500 text-sm py-2">No base headers configured</div>
                                </div>
                                <button type="button" id="add-header-btn" class="text-sm text-blue-600 hover:text-blue-800 mt-2">+ Add Header</button>
                                <p class="text-xs text-gray-500 mt-1">Headers included with every request (e.g., Content-Type)</p>
                            </div>

                            <div class="flex items-center">
                                <input type="checkbox" id="external-api-enabled" name="enabled" checked>
                                <label for="external-api-enabled" class="ml-2 text-sm text-gray-700">Enabled</label>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button type="button" class="btn btn-secondary close-modal">Cancel</button>
                            <button type="submit" class="btn btn-primary">Save</button>
                        </div>
                    </form>
                </div>
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-external-api-btn').on('click', () => showModal());

        $('.edit-btn').on('click', function() {
            const id = $(this).data('id');
            const api = currentApis.find(a => a.id === id);

            if (api) {
                showModal(api);
            }
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });

        // Modal events (only close via close button, not backdrop click)
        $('#external-api-modal .close-modal').on('click', hideModal);
        $('#external-api-form').on('submit', handleFormSubmit);
        $('#add-header-btn').on('click', addHeaderRow);

        // Delegated event for removing headers
        $(document).off('click', '.remove-header-btn');
        $(document).on('click', '.remove-header-btn', function() {
            $(this).closest('.header-row').remove();
            // Show "no headers" message if empty
            if ($('#base-headers-container .header-row').length === 0) {
                $('#base-headers-container').html('<div class="text-gray-500 text-sm py-2">No base headers configured</div>');
            }
        });
    }

    function showModal(api = null) {
        const isEdit = !!api;

        $('#external-api-modal-title').text(isEdit ? 'Edit External API' : 'Create External API');
        $('#external-api-form')[0].reset();

        $('#external-api-id').val(api?.id || '').prop('disabled', isEdit);
        $('#external-api-name').val(api?.name || '');
        $('#external-api-description').val(api?.description || '');
        $('#external-api-base-url').val(api?.base_url || '');
        $('#external-api-enabled').prop('checked', api?.enabled !== false);

        // Store editing id
        $('#external-api-form').data('edit-id', api?.id || null);

        // Render headers
        renderBaseHeaders(api?.base_headers || {});

        $('#external-api-modal').removeClass('hidden');
    }

    function hideModal() {
        $('#external-api-modal').addClass('hidden');
        $('#external-api-form').data('edit-id', null);
    }

    function renderBaseHeaders(headers) {
        const container = $('#base-headers-container');
        container.empty();

        const entries = Object.entries(headers || {});

        if (entries.length === 0) {
            container.html('<div class="text-gray-500 text-sm py-2">No base headers configured</div>');
        } else {
            entries.forEach(([key, value]) => {
                container.append(createHeaderRow(key, value));
            });
        }
    }

    function createHeaderRow(key = '', value = '') {
        return `
            <div class="header-row flex gap-2 mb-2">
                <input type="text" class="form-input flex-1 header-key" placeholder="Header Name" value="${Utils.escapeHtml(key)}">
                <input type="text" class="form-input flex-1 header-value" placeholder="Header Value" value="${Utils.escapeHtml(value)}">
                <button type="button" class="remove-header-btn text-red-500 hover:text-red-700 px-2">&times;</button>
            </div>
        `;
    }

    function addHeaderRow() {
        const container = $('#base-headers-container');
        // Remove "no headers" message if present
        container.find('.text-gray-500').remove();
        container.append(createHeaderRow());
    }

    async function handleFormSubmit(e) {
        e.preventDefault();

        const editId = $('#external-api-form').data('edit-id');
        const id = $('#external-api-id').val().trim();
        const name = $('#external-api-name').val().trim();
        const description = $('#external-api-description').val().trim() || null;
        const baseUrl = $('#external-api-base-url').val().trim();
        const enabled = $('#external-api-enabled').is(':checked');

        // Collect headers
        const baseHeaders = {};
        $('#base-headers-container .header-row').each(function() {
            const key = $(this).find('.header-key').val().trim();
            const value = $(this).find('.header-value').val().trim();

            if (key) {
                baseHeaders[key] = value;
            }
        });

        // Validation
        if (!id && !editId) {
            Utils.showToast('ID is required', 'error');
            return;
        }

        if (!name) {
            Utils.showToast('Name is required', 'error');
            return;
        }

        if (!baseUrl) {
            Utils.showToast('Base URL is required', 'error');
            return;
        }

        const $btn = $('#external-api-form button[type="submit"]');
        const originalText = $btn.text();
        $btn.prop('disabled', true).text('Saving...');

        try {
            if (editId) {
                await API.updateExternalApi(editId, {
                    name,
                    description,
                    base_url: baseUrl,
                    base_headers: baseHeaders,
                    enabled
                });
                Utils.showToast('External API updated successfully', 'success');
            } else {
                await API.createExternalApi({
                    id,
                    name,
                    description,
                    base_url: baseUrl,
                    base_headers: baseHeaders
                });
                Utils.showToast('External API created successfully', 'success');
            }

            hideModal();
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
            $btn.prop('disabled', false).text(originalText);
        }
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete external API "${id}"?`)) {
            return;
        }

        try {
            await API.deleteExternalApi(id);
            Utils.showToast('External API deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
