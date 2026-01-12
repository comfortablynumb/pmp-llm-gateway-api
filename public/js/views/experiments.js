/**
 * Experiments (A/B Testing) CRUD view
 */
const Experiments = (function() {
    let models = [];

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listExperiments();
            $('#content').html(renderList(data.experiments || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(experiments) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${experiments.length} experiment(s)</p>
                <button id="create-experiment-btn" class="btn btn-primary">+ New Experiment</button>
            </div>

            ${experiments.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Status</th>
                                <th>Variants</th>
                                <th>Created</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${experiments.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No experiments configured yet')}
        `;
    }

    function renderRow(experiment) {
        const statusBadge = getStatusBadge(experiment.status);
        const variantCount = experiment.variants?.length || 0;
        const createdDate = new Date(experiment.created_at).toLocaleDateString();

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(experiment.id)}</td>
                <td>${Utils.escapeHtml(experiment.name)}</td>
                <td><span class="badge ${statusBadge.class}">${statusBadge.text}</span></td>
                <td>${variantCount} variant(s)</td>
                <td class="text-sm text-gray-500">${createdDate}</td>
                <td>
                    ${renderActionButtons(experiment)}
                </td>
            </tr>
        `;
    }

    function getStatusBadge(status) {
        switch (status) {
            case 'draft': return { class: 'badge-gray', text: 'Draft' };
            case 'active': return { class: 'badge-success', text: 'Active' };
            case 'paused': return { class: 'badge-warning', text: 'Paused' };
            case 'completed': return { class: 'badge-info', text: 'Completed' };
            default: return { class: 'badge-gray', text: status };
        }
    }

    function renderActionButtons(experiment) {
        const id = Utils.escapeHtml(experiment.id);
        let buttons = `
            <button class="edit-btn btn-sm btn-edit mr-1" data-id="${id}">Edit</button>
            <button class="results-btn btn-sm btn-info mr-1" data-id="${id}">Results</button>
        `;

        switch (experiment.status) {
            case 'draft':
                buttons += `<button class="start-btn btn-sm btn-success mr-1" data-id="${id}">Start</button>`;
                buttons += `<button class="delete-btn btn-sm btn-delete" data-id="${id}">Delete</button>`;
                break;
            case 'active':
                buttons += `<button class="pause-btn btn-sm btn-warning mr-1" data-id="${id}">Pause</button>`;
                buttons += `<button class="complete-btn btn-sm btn-info" data-id="${id}">Complete</button>`;
                break;
            case 'paused':
                buttons += `<button class="resume-btn btn-sm btn-success mr-1" data-id="${id}">Resume</button>`;
                buttons += `<button class="complete-btn btn-sm btn-info" data-id="${id}">Complete</button>`;
                break;
            case 'completed':
                buttons += `<button class="delete-btn btn-sm btn-delete" data-id="${id}">Delete</button>`;
                break;
        }

        return buttons;
    }

    function renderForm(experiment = null) {
        const isEdit = !!experiment;
        const title = isEdit ? 'Edit Experiment' : 'Create Experiment';

        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="experiment-form" class="card">
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                        <input type="text" name="id" value="${Utils.escapeHtml(experiment?.id || '')}"
                            class="form-input ${isEdit ? 'bg-gray-100' : ''}"
                            placeholder="my-experiment" ${isEdit ? 'readonly' : 'required'}>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" value="${Utils.escapeHtml(experiment?.name || '')}"
                            class="form-input" placeholder="My Experiment" required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <textarea name="description" class="form-input" rows="2"
                            placeholder="Optional description">${Utils.escapeHtml(experiment?.description || '')}</textarea>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <div class="flex justify-between items-center mb-3">
                            <h3 class="font-medium">Variants</h3>
                            <button type="button" id="add-variant-btn" class="btn-sm btn-primary">+ Add Variant</button>
                        </div>
                        <div id="variants-container">
                            ${(experiment?.variants || []).map((v, i) => renderVariantForm(v, i)).join('')}
                        </div>
                        <p class="text-xs text-gray-500 mt-2">Add at least 2 variants for A/B testing</p>
                    </div>

                    <div class="border-t pt-4 mt-4">
                        <h3 class="font-medium mb-3">Traffic Allocation</h3>
                        <div id="traffic-allocation-container">
                            ${renderTrafficAllocation(experiment)}
                        </div>
                        <p class="text-xs text-gray-500 mt-2">Percentages must sum to 100</p>
                    </div>

                    <div class="flex justify-end gap-3 mt-6 pt-4 border-t">
                        <button type="button" id="cancel-btn" class="btn btn-secondary">Cancel</button>
                        <button type="submit" class="btn btn-primary">${isEdit ? 'Update' : 'Create'}</button>
                    </div>
                </form>
            </div>
        `;
    }

    function renderVariantForm(variant = null, index = 0) {
        const isControl = variant?.is_control || false;
        const configType = variant?.config?.type || 'model_reference';
        const modelId = variant?.config?.model_id || '';

        return `
            <div class="variant-item border rounded p-4 mb-3" data-index="${index}">
                <div class="flex justify-between items-start mb-3">
                    <h4 class="font-medium">Variant ${index + 1}</h4>
                    <button type="button" class="remove-variant-btn text-red-500 hover:text-red-700">&times;</button>
                </div>

                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">Variant ID</label>
                        <input type="text" class="form-input variant-id"
                            value="${Utils.escapeHtml(variant?.id || '')}"
                            placeholder="control" required>
                    </div>
                    <div>
                        <label class="block text-sm text-gray-600 mb-1">Name</label>
                        <input type="text" class="form-input variant-name"
                            value="${Utils.escapeHtml(variant?.name || '')}"
                            placeholder="Control Group" required>
                    </div>
                </div>

                <div class="mt-3">
                    <label class="block text-sm text-gray-600 mb-1">Config Type</label>
                    <select class="form-input variant-config-type">
                        <option value="model_reference" ${configType === 'model_reference' ? 'selected' : ''}>Model Reference</option>
                        <option value="config_override" ${configType === 'config_override' ? 'selected' : ''}>Config Override</option>
                    </select>
                </div>

                <div class="mt-3">
                    <label class="block text-sm text-gray-600 mb-1">Model</label>
                    <select class="form-input variant-model-id" required>
                        <option value="">Select a model</option>
                        ${models.sort((a, b) => a.name.localeCompare(b.name)).map(m => `<option value="${Utils.escapeHtml(m.id)}" ${m.id === modelId ? 'selected' : ''}>${Utils.escapeHtml(m.name)} (${Utils.escapeHtml(m.id)})</option>`).join('')}
                    </select>
                </div>

                <div class="config-overrides mt-3 ${configType !== 'config_override' ? 'hidden' : ''}">
                    <div class="grid grid-cols-3 gap-3">
                        <div>
                            <label class="block text-xs text-gray-500 mb-1">Temperature</label>
                            <input type="number" step="0.1" min="0" max="2"
                                class="form-input text-sm variant-temperature"
                                value="${variant?.config?.temperature ?? ''}" placeholder="0.7">
                        </div>
                        <div>
                            <label class="block text-xs text-gray-500 mb-1">Max Tokens</label>
                            <input type="number" min="1"
                                class="form-input text-sm variant-max-tokens"
                                value="${variant?.config?.max_tokens ?? ''}" placeholder="4096">
                        </div>
                        <div>
                            <label class="block text-xs text-gray-500 mb-1">Top P</label>
                            <input type="number" step="0.1" min="0" max="1"
                                class="form-input text-sm variant-top-p"
                                value="${variant?.config?.top_p ?? ''}" placeholder="1.0">
                        </div>
                    </div>
                </div>

                <div class="flex items-center mt-3">
                    <input type="checkbox" class="variant-is-control" ${isControl ? 'checked' : ''}>
                    <label class="ml-2 text-sm text-gray-700">Control variant (baseline for comparison)</label>
                </div>
            </div>
        `;
    }

    function renderTrafficAllocation(experiment) {
        const variants = experiment?.variants || [];
        const allocations = experiment?.traffic_allocation || [];

        if (variants.length === 0) {
            return '<p class="text-gray-500 text-sm">Add variants first</p>';
        }

        return variants.map((v, i) => {
            const allocation = allocations.find(a => a.variant_id === v.id);
            const percentage = allocation?.percentage || Math.floor(100 / variants.length);

            return `
                <div class="flex items-center gap-3 mb-2">
                    <label class="w-32 text-sm">${Utils.escapeHtml(v.name || v.id)}</label>
                    <input type="number" class="form-input w-24 traffic-percentage"
                        data-variant-id="${Utils.escapeHtml(v.id)}"
                        min="0" max="100" value="${percentage}">
                    <span class="text-gray-500">%</span>
                </div>
            `;
        }).join('');
    }

    function renderResults(results) {
        return `
            <div class="max-w-6xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Experiment Results: ${Utils.escapeHtml(results.experiment_name)}</h2>
                </div>

                <div class="grid grid-cols-4 gap-4 mb-6">
                    <div class="card">
                        <p class="text-sm text-gray-500">Status</p>
                        <p class="text-2xl font-bold">${Utils.escapeHtml(results.status)}</p>
                    </div>
                    <div class="card">
                        <p class="text-sm text-gray-500">Total Requests</p>
                        <p class="text-2xl font-bold">${results.total_requests.toLocaleString()}</p>
                    </div>
                    <div class="card">
                        <p class="text-sm text-gray-500">Duration</p>
                        <p class="text-2xl font-bold">${results.duration_hours ? results.duration_hours.toFixed(1) + 'h' : 'N/A'}</p>
                    </div>
                    <div class="card">
                        <p class="text-sm text-gray-500">Winner</p>
                        <p class="text-2xl font-bold">${results.winner_variant_id ? Utils.escapeHtml(results.winner_variant_id) : 'TBD'}</p>
                    </div>
                </div>

                ${results.recommendation ? `
                    <div class="card bg-blue-50 border-blue-200 mb-6">
                        <p class="text-sm font-medium text-blue-800">Recommendation</p>
                        <p class="text-blue-700">${Utils.escapeHtml(results.recommendation)}</p>
                    </div>
                ` : ''}

                <div class="card mb-6">
                    <h3 class="font-medium mb-4">Variant Metrics</h3>
                    <div class="overflow-x-auto">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>Variant</th>
                                    <th>Requests</th>
                                    <th>Success Rate</th>
                                    <th>Avg Latency</th>
                                    <th>P95 Latency</th>
                                    <th>Total Tokens</th>
                                    <th>Avg Cost</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${results.variant_metrics.map(m => `
                                    <tr>
                                        <td class="font-medium">${Utils.escapeHtml(m.variant_name)}</td>
                                        <td>${m.total_requests.toLocaleString()}</td>
                                        <td>${(m.success_rate * 100).toFixed(1)}%</td>
                                        <td>${m.latency.avg_ms.toFixed(0)}ms</td>
                                        <td>${m.latency.p95_ms}ms</td>
                                        <td>${m.total_tokens.toLocaleString()}</td>
                                        <td>$${(m.avg_cost_micros / 1000000).toFixed(4)}</td>
                                    </tr>
                                `).join('')}
                            </tbody>
                        </table>
                    </div>
                </div>

                ${results.significance_tests.length > 0 ? `
                    <div class="card">
                        <h3 class="font-medium mb-4">Statistical Significance</h3>
                        <div class="overflow-x-auto">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>Metric</th>
                                        <th>Control</th>
                                        <th>Treatment</th>
                                        <th>Change</th>
                                        <th>P-Value</th>
                                        <th>Significant</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    ${results.significance_tests.map(t => `
                                        <tr>
                                            <td>${Utils.escapeHtml(t.metric)}</td>
                                            <td>${t.control_mean.toFixed(2)}</td>
                                            <td>${t.treatment_mean.toFixed(2)}</td>
                                            <td class="${t.relative_change >= 0 ? 'text-green-600' : 'text-red-600'}">${(t.relative_change * 100).toFixed(1)}%</td>
                                            <td>${t.p_value.toFixed(4)}</td>
                                            <td>
                                                <span class="badge ${t.is_significant ? 'badge-success' : 'badge-gray'}">
                                                    ${t.is_significant ? 'Yes' : 'No'}
                                                </span>
                                            </td>
                                        </tr>
                                    `).join('')}
                                </tbody>
                            </table>
                        </div>
                    </div>
                ` : ''}
            </div>
        `;
    }

    function bindListEvents() {
        $('#create-experiment-btn').on('click', () => showForm());

        $('.edit-btn').on('click', function() {
            const id = $(this).data('id');
            showForm(id);
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });

        $('.results-btn').on('click', function() {
            const id = $(this).data('id');
            showResults(id);
        });

        $('.start-btn').on('click', function() {
            const id = $(this).data('id');
            changeStatus(id, 'start', 'Starting...');
        });

        $('.pause-btn').on('click', function() {
            const id = $(this).data('id');
            changeStatus(id, 'pause', 'Pausing...');
        });

        $('.resume-btn').on('click', function() {
            const id = $(this).data('id');
            changeStatus(id, 'resume', 'Resuming...');
        });

        $('.complete-btn').on('click', function() {
            const id = $(this).data('id');

            if (!Utils.confirm('Complete this experiment? This action cannot be undone.')) {
                return;
            }
            changeStatus(id, 'complete', 'Completing...');
        });
    }

    async function showForm(id = null) {
        let experiment = null;
        $('#content').html(Utils.renderLoading());

        try {
            // Load models first
            const modelData = await API.listModels();
            models = modelData.models || [];

            if (id) {
                experiment = await API.getExperiment(id);
            }
        } catch (error) {
            Utils.showToast('Failed to load data', 'error');
            return render();
        }

        $('#content').html(renderForm(experiment));
        bindFormEvents(id);
    }

    function bindFormEvents(editId) {
        let variantIndex = $('.variant-item').length;

        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#add-variant-btn').on('click', function() {
            $('#variants-container').append(renderVariantForm(null, variantIndex++));
            bindVariantEvents();
            updateTrafficAllocation();
        });

        bindVariantEvents();

        $('#experiment-form').on('submit', async function(e) {
            e.preventDefault();

            const formData = {
                id: $('input[name="id"]').val(),
                name: $('input[name="name"]').val(),
                description: $('textarea[name="description"]').val() || null,
                variants: collectVariants(),
                traffic_allocation: collectTrafficAllocation()
            };

            // Validate
            if (formData.variants.length < 2) {
                Utils.showToast('At least 2 variants are required', 'error');
                return;
            }

            const totalTraffic = formData.traffic_allocation.reduce((sum, a) => sum + a.percentage, 0);

            if (totalTraffic !== 100) {
                Utils.showToast('Traffic allocation must sum to 100%', 'error');
                return;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                if (editId) {
                    delete formData.id;
                    await API.updateExperiment(editId, formData);
                    Utils.showToast('Experiment updated successfully', 'success');
                } else {
                    await API.createExperiment(formData);
                    Utils.showToast('Experiment created successfully', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    function bindVariantEvents() {
        $('.remove-variant-btn').off('click').on('click', function() {
            $(this).closest('.variant-item').remove();
            updateTrafficAllocation();
        });

        $('.variant-config-type').off('change').on('change', function() {
            const $item = $(this).closest('.variant-item');
            const isOverride = $(this).val() === 'config_override';
            $item.find('.config-overrides').toggleClass('hidden', !isOverride);
        });

        $('.variant-id, .variant-name').off('input').on('input', function() {
            updateTrafficAllocation();
        });
    }

    function updateTrafficAllocation() {
        const variants = collectVariants();
        const perVariant = variants.length > 0 ? Math.floor(100 / variants.length) : 0;
        let html = '';

        variants.forEach((v, i) => {
            const isLast = i === variants.length - 1;
            const percentage = isLast ? 100 - (perVariant * (variants.length - 1)) : perVariant;

            html += `
                <div class="flex items-center gap-3 mb-2">
                    <label class="w-32 text-sm">${Utils.escapeHtml(v.name || v.id)}</label>
                    <input type="number" class="form-input w-24 traffic-percentage"
                        data-variant-id="${Utils.escapeHtml(v.id)}"
                        min="0" max="100" value="${percentage}">
                    <span class="text-gray-500">%</span>
                </div>
            `;
        });

        $('#traffic-allocation-container').html(html || '<p class="text-gray-500 text-sm">Add variants first</p>');
    }

    function collectVariants() {
        const variants = [];
        $('.variant-item').each(function() {
            const $item = $(this);
            const configType = $item.find('.variant-config-type').val();

            const variant = {
                id: $item.find('.variant-id').val(),
                name: $item.find('.variant-name').val(),
                is_control: $item.find('.variant-is-control').is(':checked'),
                config: {
                    type: configType,
                    model_id: $item.find('.variant-model-id').val()
                }
            };

            if (configType === 'config_override') {
                const temp = $item.find('.variant-temperature').val();
                const maxTokens = $item.find('.variant-max-tokens').val();
                const topP = $item.find('.variant-top-p').val();

                if (temp) variant.config.temperature = parseFloat(temp);
                if (maxTokens) variant.config.max_tokens = parseInt(maxTokens);
                if (topP) variant.config.top_p = parseFloat(topP);
            }

            if (variant.id && variant.config.model_id) {
                variants.push(variant);
            }
        });

        return variants;
    }

    function collectTrafficAllocation() {
        const allocations = [];
        $('.traffic-percentage').each(function() {
            const variantId = $(this).data('variant-id');
            const percentage = parseInt($(this).val()) || 0;

            if (variantId) {
                allocations.push({
                    variant_id: variantId,
                    percentage: percentage
                });
            }
        });

        return allocations;
    }

    async function showResults(id) {
        $('#content').html(Utils.renderLoading());

        try {
            const results = await API.getExperimentResults(id);
            $('#content').html(renderResults(results));
            $('#back-btn').on('click', () => render());
        } catch (error) {
            Utils.showToast('Failed to load results', 'error');
            render();
        }
    }

    async function changeStatus(id, action, loadingText) {
        try {
            const apiCall = {
                'start': API.startExperiment,
                'pause': API.pauseExperiment,
                'resume': API.resumeExperiment,
                'complete': API.completeExperiment
            }[action];

            await apiCall(id);
            Utils.showToast(`Experiment ${action}ed successfully`, 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete experiment "${id}"?`)) {
            return;
        }

        try {
            await API.deleteExperiment(id);
            Utils.showToast('Experiment deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    return { render };
})();
