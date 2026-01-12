/**
 * Teams CRUD view
 */
const Teams = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listTeams();
            $('#content').html(renderList(data.teams || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(teams) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${teams.length} team(s)</p>
                <button id="create-team-btn" class="btn btn-primary">+ New Team</button>
            </div>

            ${teams.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Description</th>
                                <th>Status</th>
                                <th>Created</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${teams.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No teams configured yet')}
        `;
    }

    function renderRow(team) {
        const statusColors = {
            active: 'badge-success',
            suspended: 'badge-warning'
        };

        const isAdministrators = team.id === 'administrators';

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(team.id)}</td>
                <td>
                    <div class="font-medium">${Utils.escapeHtml(team.name)}</div>
                </td>
                <td class="text-sm text-gray-500">
                    ${team.description ? Utils.escapeHtml(team.description) : '-'}
                </td>
                <td><span class="badge ${statusColors[team.status] || 'badge-gray'}">${team.status}</span></td>
                <td class="text-sm text-gray-500">${Utils.formatDate(team.created_at)}</td>
                <td>
                    <div class="flex items-center gap-1">
                        <button class="edit-btn btn-sm btn-secondary-sm" data-id="${Utils.escapeHtml(team.id)}">Edit</button>
                        ${!isAdministrators ? `
                            ${team.status === 'active' ? `
                                <button class="suspend-btn btn-sm btn-warning-sm" data-id="${Utils.escapeHtml(team.id)}">Suspend</button>
                            ` : ''}
                            ${team.status === 'suspended' ? `
                                <button class="activate-btn btn-sm btn-success-sm" data-id="${Utils.escapeHtml(team.id)}">Activate</button>
                            ` : ''}
                            <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(team.id)}">Delete</button>
                        ` : ''}
                    </div>
                </td>
            </tr>
        `;
    }

    function renderForm(team = null) {
        const isEdit = !!team;
        const title = isEdit ? 'Edit Team' : 'Create Team';

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="team-form" class="card">
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                        <input type="text" name="id" class="form-input"
                            placeholder="my-team"
                            value="${isEdit ? Utils.escapeHtml(team.id) : ''}"
                            ${isEdit ? 'readonly' : 'required'}
                            pattern="^[a-z][a-z0-9-]*[a-z0-9]$"
                            title="Lowercase letters, numbers, and hyphens only. Must start with a letter.">
                        <p class="text-xs text-gray-500 mt-1">Lowercase letters, numbers, and hyphens only</p>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" class="form-input"
                            placeholder="My Team"
                            value="${isEdit ? Utils.escapeHtml(team.name) : ''}"
                            required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <textarea name="description" class="form-input" rows="2"
                            placeholder="Optional description">${isEdit && team.description ? Utils.escapeHtml(team.description) : ''}</textarea>
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
        $('#create-team-btn').on('click', () => showForm());

        $('.edit-btn').on('click', async function() {
            const id = $(this).data('id');

            try {
                const team = await API.getTeam(id);
                showForm(team);
            } catch (error) {
                Utils.showToast(error.message, 'error');
            }
        });

        $('.suspend-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Are you sure you want to suspend this team?')) {
                try {
                    await API.suspendTeam(id);
                    Utils.showToast('Team suspended', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });

        $('.activate-btn').on('click', async function() {
            const id = $(this).data('id');

            try {
                await API.activateTeam(id);
                Utils.showToast('Team activated', 'success');
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
            }
        });

        $('.delete-btn').on('click', async function() {
            const id = $(this).data('id');

            if (Utils.confirm('Are you sure you want to delete this team? This action cannot be undone.')) {
                try {
                    await API.deleteTeam(id);
                    Utils.showToast('Team deleted', 'success');
                    render();
                } catch (error) {
                    Utils.showToast(error.message, 'error');
                }
            }
        });
    }

    function showForm(team = null) {
        $('#content').html(renderForm(team));
        bindFormEvents(team);
    }

    function bindFormEvents(team = null) {
        const isEdit = !!team;

        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#team-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            const data = {
                name: formData.name,
                description: formData.description || null
            };

            if (!isEdit) {
                data.id = formData.id;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text(isEdit ? 'Updating...' : 'Creating...');

            try {
                if (isEdit) {
                    await API.updateTeam(team.id, data);
                    Utils.showToast('Team updated', 'success');
                } else {
                    await API.createTeam(data);
                    Utils.showToast('Team created', 'success');
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
