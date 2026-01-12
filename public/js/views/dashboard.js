/**
 * Dashboard view with statistics and graphs
 */
const Dashboard = (function() {
    let charts = {};

    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const [models, prompts, apiKeys, workflows, credentials, stats, logs] = await Promise.all([
                API.listModels().catch(() => ({ models: [], total: 0 })),
                API.listPrompts().catch(() => ({ prompts: [], total: 0 })),
                API.listApiKeys().catch(() => ({ api_keys: [], total: 0 })),
                API.listWorkflows().catch(() => ({ workflows: [], total: 0 })),
                API.listCredentialProviders().catch(() => ({ providers: [] })),
                API.getExecutionStats().catch(() => getEmptyStats()),
                API.listExecutionLogs({ limit: 1000 }).catch(() => ({ logs: [], total: 0 }))
            ]);

            const timeseriesData = aggregateLogsByDate(logs.logs || []);
            const modelUsageData = aggregateByModel(logs.logs || []);

            $('#content').html(renderDashboard({
                models: models.total || models.models?.length || 0,
                prompts: prompts.total || prompts.prompts?.length || 0,
                apiKeys: apiKeys.total || apiKeys.api_keys?.length || 0,
                workflows: workflows.total || workflows.workflows?.length || 0,
                providers: credentials.providers || [],
                stats: stats,
                timeseriesData: timeseriesData,
                modelUsageData: modelUsageData
            }));

            bindEvents();
            renderCharts(timeseriesData, modelUsageData, stats);
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function getEmptyStats() {
        return {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            success_rate: 0,
            avg_execution_time_ms: 0,
            total_cost_micros: 0,
            total_input_tokens: 0,
            total_output_tokens: 0
        };
    }

    function aggregateLogsByDate(logs) {
        const byDate = {};
        const now = new Date();

        // Initialize last 14 days
        for (let i = 13; i >= 0; i--) {
            const date = new Date(now);
            date.setDate(date.getDate() - i);
            const key = date.toISOString().split('T')[0];
            byDate[key] = { cost: 0, tokens: 0, executions: 0, successes: 0 };
        }

        // Aggregate logs
        logs.forEach(log => {
            const date = log.created_at ? log.created_at.split('T')[0] : null;

            if (date && byDate[date]) {
                byDate[date].cost += (log.cost_micros || 0) / 1000000;
                byDate[date].tokens += log.token_usage?.total_tokens || 0;
                byDate[date].executions += 1;

                if (log.status === 'success') {
                    byDate[date].successes += 1;
                }
            }
        });

        return Object.entries(byDate).map(([date, data]) => ({
            date,
            ...data,
            successRate: data.executions > 0 ? (data.successes / data.executions) * 100 : 0
        }));
    }

    function aggregateByModel(logs) {
        const byModel = {};

        logs.forEach(log => {
            if (log.execution_type === 'model' && log.resource_id) {
                const modelId = log.resource_name || log.resource_id;

                if (!byModel[modelId]) {
                    byModel[modelId] = { executions: 0, tokens: 0, cost: 0 };
                }
                byModel[modelId].executions += 1;
                byModel[modelId].tokens += log.token_usage?.total_tokens || 0;
                byModel[modelId].cost += (log.cost_micros || 0) / 1000000;
            }
        });

        return Object.entries(byModel)
            .map(([model, data]) => ({ model, ...data }))
            .sort((a, b) => b.executions - a.executions)
            .slice(0, 10);
    }

    function renderDashboard(data) {
        return `
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
                ${renderStatCard('Total Executions', data.stats.total_executions, 'blue')}
                ${renderStatCard('Success Rate', data.stats.success_rate.toFixed(1) + '%', 'green')}
                ${renderStatCard('Total Cost', '$' + (data.stats.total_cost_micros / 1000000).toFixed(4), 'orange')}
                ${renderStatCard('Avg Response', data.stats.avg_execution_time_ms.toFixed(0) + 'ms', 'purple')}
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
                ${renderResourceCard('Models', data.models, '#models', 'blue')}
                ${renderResourceCard('Prompts', data.prompts, '#prompts', 'green')}
                ${renderResourceCard('API Keys', data.apiKeys, '#api-keys', 'purple')}
                ${renderResourceCard('Workflows', data.workflows, '#workflows', 'orange')}
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
                <div class="card">
                    <h3 class="text-lg font-semibold mb-4">Cost Over Time (Last 14 Days)</h3>
                    <div class="h-64">
                        <canvas id="cost-chart"></canvas>
                    </div>
                </div>
                <div class="card">
                    <h3 class="text-lg font-semibold mb-4">Tokens Consumed (Last 14 Days)</h3>
                    <div class="h-64">
                        <canvas id="tokens-chart"></canvas>
                    </div>
                </div>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
                <div class="card">
                    <h3 class="text-lg font-semibold mb-4">Executions & Success Rate</h3>
                    <div class="h-64">
                        <canvas id="executions-chart"></canvas>
                    </div>
                </div>
                <div class="card">
                    <h3 class="text-lg font-semibold mb-4">Top Model Usage</h3>
                    <div class="h-64">
                        <canvas id="model-usage-chart"></canvas>
                    </div>
                </div>
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
                    <h3 class="text-lg font-semibold mb-4">Token Summary</h3>
                    <div class="space-y-4">
                        <div class="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
                            <span class="text-gray-600">Input Tokens</span>
                            <span class="font-mono text-lg font-semibold">${data.stats.total_input_tokens.toLocaleString()}</span>
                        </div>
                        <div class="flex justify-between items-center p-3 bg-gray-50 rounded-lg">
                            <span class="text-gray-600">Output Tokens</span>
                            <span class="font-mono text-lg font-semibold">${data.stats.total_output_tokens.toLocaleString()}</span>
                        </div>
                        <div class="flex justify-between items-center p-3 bg-blue-50 rounded-lg">
                            <span class="text-blue-600 font-medium">Total Tokens</span>
                            <span class="font-mono text-lg font-bold text-blue-600">${(data.stats.total_input_tokens + data.stats.total_output_tokens).toLocaleString()}</span>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    function renderStatCard(label, value, color) {
        const colors = {
            blue: 'text-blue-600 bg-blue-50',
            green: 'text-green-600 bg-green-50',
            purple: 'text-purple-600 bg-purple-50',
            orange: 'text-orange-600 bg-orange-50'
        };

        return `
            <div class="card text-center">
                <p class="text-sm text-gray-500 mb-1">${label}</p>
                <p class="text-2xl font-bold ${colors[color].split(' ')[0]}">${value}</p>
            </div>
        `;
    }

    function renderResourceCard(title, count, link, color) {
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
                        <p class="text-2xl font-bold mt-1">${count}</p>
                    </div>
                    <div class="w-10 h-10 ${colors[color]} rounded-full flex items-center justify-center text-white text-sm font-bold">
                        ${count}
                    </div>
                </div>
            </a>
        `;
    }

    function renderCharts(timeseriesData, modelUsageData, stats) {
        // Check if Chart.js is available
        if (typeof Chart === 'undefined') {
            console.warn('Chart.js not loaded, skipping chart rendering');
            return;
        }

        // Destroy existing charts
        Object.values(charts).forEach(chart => chart.destroy());
        charts = {};

        const labels = timeseriesData.map(d => formatDate(d.date));

        // Cost Chart
        const costCtx = document.getElementById('cost-chart');

        if (costCtx) {
            charts.cost = new Chart(costCtx, {
                type: 'line',
                data: {
                    labels: labels,
                    datasets: [{
                        label: 'Cost ($)',
                        data: timeseriesData.map(d => d.cost),
                        borderColor: 'rgb(249, 115, 22)',
                        backgroundColor: 'rgba(249, 115, 22, 0.1)',
                        fill: true,
                        tension: 0.3
                    }]
                },
                options: getLineChartOptions('$')
            });
        }

        // Tokens Chart
        const tokensCtx = document.getElementById('tokens-chart');

        if (tokensCtx) {
            charts.tokens = new Chart(tokensCtx, {
                type: 'line',
                data: {
                    labels: labels,
                    datasets: [{
                        label: 'Tokens',
                        data: timeseriesData.map(d => d.tokens),
                        borderColor: 'rgb(59, 130, 246)',
                        backgroundColor: 'rgba(59, 130, 246, 0.1)',
                        fill: true,
                        tension: 0.3
                    }]
                },
                options: getLineChartOptions('')
            });
        }

        // Executions Chart (dual axis)
        const execCtx = document.getElementById('executions-chart');

        if (execCtx) {
            charts.executions = new Chart(execCtx, {
                type: 'bar',
                data: {
                    labels: labels,
                    datasets: [
                        {
                            label: 'Executions',
                            data: timeseriesData.map(d => d.executions),
                            backgroundColor: 'rgba(59, 130, 246, 0.7)',
                            yAxisID: 'y'
                        },
                        {
                            label: 'Success Rate (%)',
                            data: timeseriesData.map(d => d.successRate),
                            type: 'line',
                            borderColor: 'rgb(34, 197, 94)',
                            backgroundColor: 'rgba(34, 197, 94, 0.1)',
                            yAxisID: 'y1',
                            tension: 0.3
                        }
                    ]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: { position: 'top' }
                    },
                    scales: {
                        y: {
                            type: 'linear',
                            position: 'left',
                            title: { display: true, text: 'Executions' },
                            beginAtZero: true
                        },
                        y1: {
                            type: 'linear',
                            position: 'right',
                            title: { display: true, text: 'Success Rate (%)' },
                            min: 0,
                            max: 100,
                            grid: { drawOnChartArea: false }
                        }
                    }
                }
            });
        }

        // Model Usage Chart
        const modelCtx = document.getElementById('model-usage-chart');

        if (modelCtx) {
            charts.modelUsage = new Chart(modelCtx, {
                type: 'bar',
                data: {
                    labels: modelUsageData.map(d => truncateLabel(d.model, 15)),
                    datasets: [{
                        label: 'Executions',
                        data: modelUsageData.map(d => d.executions),
                        backgroundColor: [
                            'rgba(59, 130, 246, 0.7)',
                            'rgba(34, 197, 94, 0.7)',
                            'rgba(249, 115, 22, 0.7)',
                            'rgba(168, 85, 247, 0.7)',
                            'rgba(236, 72, 153, 0.7)',
                            'rgba(20, 184, 166, 0.7)',
                            'rgba(245, 158, 11, 0.7)',
                            'rgba(239, 68, 68, 0.7)',
                            'rgba(107, 114, 128, 0.7)',
                            'rgba(99, 102, 241, 0.7)'
                        ]
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    indexAxis: 'y',
                    plugins: {
                        legend: { display: false }
                    },
                    scales: {
                        x: { beginAtZero: true }
                    }
                }
            });
        }
    }

    function getLineChartOptions(prefix) {
        return {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: { position: 'top' }
            },
            scales: {
                y: {
                    beginAtZero: true,
                    ticks: {
                        callback: function(value) {
                            return prefix + value.toLocaleString();
                        }
                    }
                }
            }
        };
    }

    function formatDate(dateStr) {
        const date = new Date(dateStr);
        return (date.getMonth() + 1) + '/' + date.getDate();
    }

    function truncateLabel(str, maxLen) {
        return str.length > maxLen ? str.substring(0, maxLen) + '...' : str;
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
