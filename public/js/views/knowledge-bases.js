/**
 * Knowledge Bases CRUD view
 */
const KnowledgeBases = (function() {
    async function render() {
        $('#content').html(Utils.renderLoading());

        try {
            const data = await API.listKnowledgeBases();
            $('#content').html(renderList(data.knowledge_bases || []));
            bindListEvents();
        } catch (error) {
            $('#content').html(Utils.renderError(error.message));
        }
    }

    function renderList(knowledgeBases) {
        return `
            <div class="flex justify-between items-center mb-6">
                <p class="text-gray-600">${knowledgeBases.length} knowledge base(s)</p>
                <button id="create-kb-btn" class="btn btn-primary">+ New Knowledge Base</button>
            </div>

            ${knowledgeBases.length > 0 ? `
                <div class="card p-0 overflow-hidden">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>Name</th>
                                <th>Type</th>
                                <th>Embedding Model</th>
                                <th>Status</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${knowledgeBases.map(renderRow).join('')}
                        </tbody>
                    </table>
                </div>
            ` : Utils.renderEmpty('No knowledge bases configured yet')}
        `;
    }

    function renderRow(kb) {
        const statusClass = kb.enabled ? 'badge-success' : 'badge-gray';
        const statusText = kb.enabled ? 'Enabled' : 'Disabled';
        const typeLabel = getTypeLabel(kb.kb_type);

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(kb.id)}</td>
                <td>${Utils.escapeHtml(kb.name)}</td>
                <td>${Utils.escapeHtml(typeLabel)}</td>
                <td class="font-mono text-sm">${Utils.escapeHtml(kb.embedding_model)} (${kb.embedding_dimensions}d)</td>
                <td><span class="badge ${statusClass}">${statusText}</span></td>
                <td>
                    <button class="docs-btn btn-sm btn-success-sm mr-2" data-id="${Utils.escapeHtml(kb.id)}">Documents</button>
                    <button class="edit-btn btn-sm btn-edit mr-2" data-id="${Utils.escapeHtml(kb.id)}">Edit</button>
                    <button class="delete-btn btn-sm btn-delete" data-id="${Utils.escapeHtml(kb.id)}">Delete</button>
                </td>
            </tr>
        `;
    }

    function getTypeLabel(type) {
        const labels = {
            'pgvector': 'PostgreSQL pgvector',
            'aws_knowledge_base': 'AWS Bedrock KB',
            'pinecone': 'Pinecone'
        };
        return labels[type] || type;
    }

    async function renderForm(kb = null) {
        const isEdit = !!kb;
        const title = isEdit ? 'Edit Knowledge Base' : 'Create Knowledge Base';

        // Load credentials for dropdown
        let credentials = [];

        try {
            const data = await API.listCredentials();
            credentials = (data.credentials || []).filter(c =>
                ['pgvector', 'aws_knowledge_base', 'pinecone'].includes(c.credential_type)
            );
        } catch (e) {
            console.error('Failed to load credentials:', e);
        }

        return `
            <div class="max-w-2xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">${title}</h2>
                </div>

                <form id="kb-form" class="card">
                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">ID</label>
                        <input type="text" name="id" value="${Utils.escapeHtml(kb?.id || '')}"
                            class="form-input ${isEdit ? 'bg-gray-100' : ''}"
                            placeholder="my-knowledge-base" ${isEdit ? 'readonly' : 'required'}>
                        <p class="text-xs text-gray-500 mt-1">Alphanumeric with hyphens only, max 50 chars</p>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                        <input type="text" name="name" value="${Utils.escapeHtml(kb?.name || '')}"
                            class="form-input" placeholder="My Knowledge Base" required>
                    </div>

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                        <input type="text" name="description" value="${Utils.escapeHtml(kb?.description || '')}"
                            class="form-input" placeholder="Optional description">
                    </div>

                    ${!isEdit ? `
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Type</label>
                            <select name="kb_type" class="form-input" required>
                                <option value="pgvector">PostgreSQL pgvector</option>
                                <option value="aws_knowledge_base">AWS Bedrock Knowledge Base</option>
                                <option value="pinecone">Pinecone</option>
                            </select>
                        </div>
                    ` : ''}

                    <div class="mb-4">
                        <label class="block text-sm font-medium text-gray-700 mb-1">Credential</label>
                        ${!isEdit ? `
                            <select name="credential_id" class="form-input" required>
                                <option value="">Select a credential...</option>
                                ${credentials.map(c => `
                                    <option value="${Utils.escapeHtml(c.id)}" ${kb?.credential_id === c.id ? 'selected' : ''}>
                                        ${Utils.escapeHtml(c.name)} (${getTypeLabel(c.credential_type)})
                                    </option>
                                `).join('')}
                            </select>
                            ${credentials.length === 0 ? '<p class="text-xs text-red-500 mt-1">No knowledge base credentials found. Create one first.</p>' : ''}
                        ` : `
                            <input type="text" value="${Utils.escapeHtml(kb?.credential_id || 'N/A')}"
                                class="form-input bg-gray-100" readonly>
                        `}
                    </div>

                    ${!isEdit ? `
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Embedding Model</label>
                            <input type="text" name="embedding_model" value="text-embedding-3-small"
                                class="form-input" placeholder="text-embedding-3-small" required>
                        </div>

                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Embedding Dimensions</label>
                            <input type="number" name="embedding_dimensions" value="1536"
                                class="form-input" placeholder="1536" required min="1" max="8192">
                        </div>
                    ` : `
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">Embedding Model</label>
                            <input type="text" value="${Utils.escapeHtml(kb?.embedding_model || '')} (${kb?.embedding_dimensions || 0}d)"
                                class="form-input bg-gray-100" readonly>
                        </div>
                    `}

                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Default Top K</label>
                            <input type="number" name="default_top_k" value="${kb?.default_top_k || 10}"
                                class="form-input" placeholder="10" min="1" max="1000">
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">Similarity Threshold</label>
                            <input type="number" name="default_similarity_threshold"
                                value="${kb?.default_similarity_threshold || 0.7}"
                                class="form-input" placeholder="0.7" min="0" max="1" step="0.05">
                        </div>
                    </div>

                    ${isEdit ? `
                        <div class="flex items-center mt-4">
                            <input type="checkbox" name="enabled" id="enabled" ${kb?.enabled !== false ? 'checked' : ''}>
                            <label for="enabled" class="ml-2 text-sm text-gray-700">Enabled</label>
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

    function bindListEvents() {
        $('#create-kb-btn').on('click', () => showForm());

        $('.docs-btn').on('click', function() {
            const id = $(this).data('id');
            showDocuments(id);
        });

        $('.edit-btn').on('click', function() {
            const id = $(this).data('id');
            showForm(id);
        });

        $('.delete-btn').on('click', function() {
            const id = $(this).data('id');
            confirmDelete(id);
        });
    }

    async function showForm(id = null) {
        let kb = null;

        if (id) {
            $('#content').html(Utils.renderLoading());

            try {
                kb = await API.getKnowledgeBase(id);
            } catch (error) {
                Utils.showToast('Failed to load knowledge base', 'error');
                return render();
            }
        }

        $('#content').html(await renderForm(kb));
        bindFormEvents(id);
    }

    function bindFormEvents(editId) {
        $('#back-btn, #cancel-btn').on('click', () => render());

        $('#kb-form').on('submit', async function(e) {
            e.preventDefault();
            const formData = Utils.getFormData(this);

            // Convert numeric values
            if (formData.default_top_k) {
                formData.default_top_k = parseInt(formData.default_top_k, 10);
            }

            if (formData.default_similarity_threshold) {
                formData.default_similarity_threshold = parseFloat(formData.default_similarity_threshold);
            }

            if (formData.embedding_dimensions) {
                formData.embedding_dimensions = parseInt(formData.embedding_dimensions, 10);
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Saving...');

            try {
                if (editId) {
                    delete formData.id;
                    delete formData.kb_type;
                    delete formData.credential_id;
                    delete formData.embedding_model;
                    delete formData.embedding_dimensions;
                    await API.updateKnowledgeBase(editId, formData);
                    Utils.showToast('Knowledge base updated successfully', 'success');
                } else {
                    await API.createKnowledgeBase(formData);
                    Utils.showToast('Knowledge base created successfully', 'success');
                }
                render();
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text(originalText);
            }
        });
    }

    async function confirmDelete(id) {
        if (!Utils.confirm(`Are you sure you want to delete knowledge base "${id}"?`)) {
            return;
        }

        try {
            await API.deleteKnowledgeBase(id);
            Utils.showToast('Knowledge base deleted successfully', 'success');
            render();
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    // ========================================================================
    // Document Management
    // ========================================================================

    async function showDocuments(kbId) {
        $('#content').html(Utils.renderLoading());

        try {
            // Fetch data sequentially for better error isolation
            let kb, docsData, ingestionsData;

            try {
                kb = await API.getKnowledgeBase(kbId);
            } catch (e) {
                throw new Error(`Failed to get KB: ${e.message}`);
            }

            try {
                docsData = await API.listDocuments(kbId);
            } catch (e) {
                docsData = { documents: [], total: 0 };
            }

            try {
                ingestionsData = await API.listIngestionOperations(kbId);
            } catch (e) {
                throw new Error(`Failed to list ingestions: ${e.message}`);
            }

            const documents = docsData.documents || [];
            const operations = ingestionsData.operations || [];

            try {
                $('#content').html(renderDocumentsView(kb, documents, operations));
            } catch (e) {
                throw new Error(`Failed to render: ${e.message}`);
            }

            bindDocumentsEvents(kbId);

            // Start polling if there are pending/in_progress operations
            const pendingOps = operations.filter(
                op => op.status === 'pending' || op.status === 'in_progress'
            );

            if (pendingOps.length > 0) {
                startPollingIngestions(kbId);
            }
        } catch (error) {
            console.error('Failed to load documents:', error);
            Utils.showToast(error.message, 'error');
            render();
        }
    }

    let pollingInterval = null;

    function startPollingIngestions(kbId) {
        if (pollingInterval) return;

        pollingInterval = setInterval(async () => {
            try {
                const data = await API.listIngestionOperations(kbId);
                const operations = data.operations || [];

                // Update the operations table
                updateOperationsTable(operations);

                // Check if any are still pending/in_progress
                const pendingOps = operations.filter(
                    op => op.status === 'pending' || op.status === 'in_progress'
                );

                if (pendingOps.length === 0) {
                    stopPollingIngestions();
                    // Refresh the full documents view
                    showDocuments(kbId);
                }
            } catch (e) {
                console.error('Failed to poll ingestions:', e);
            }
        }, 2000);
    }

    function stopPollingIngestions() {
        if (pollingInterval) {
            clearInterval(pollingInterval);
            pollingInterval = null;
        }
    }

    function updateOperationsTable(operations) {
        const pendingOps = operations.filter(
            op => op.status === 'pending' || op.status === 'in_progress'
        );

        if (pendingOps.length === 0) {
            $('#pending-operations').hide();
            return;
        }

        $('#pending-operations').show();
        $('#pending-ops-body').html(pendingOps.map(renderOperationRow).join(''));
    }

    function renderOperationRow(op) {
        const statusClass = op.status === 'pending' ? 'badge-yellow' :
                           op.status === 'in_progress' ? 'badge-blue' :
                           op.status === 'success' ? 'badge-success' : 'badge-red';

        return `
            <tr>
                <td class="font-mono text-sm">${Utils.escapeHtml(op.source_name || 'Unknown')}</td>
                <td><span class="badge ${statusClass}">${op.status}</span></td>
                <td class="text-xs text-gray-500">${Utils.formatDate(op.created_at)}</td>
            </tr>
        `;
    }

    function renderDocumentsView(kb, documents, ingestions) {
        // Filter pending/in_progress operations
        const pendingOps = (ingestions || []).filter(
            op => op.status === 'pending' || op.status === 'in_progress'
        );

        return `
            <div class="max-w-4xl">
                <div class="flex items-center mb-6">
                    <button id="back-btn" class="mr-4 text-gray-500 hover:text-gray-700">&larr; Back</button>
                    <h2 class="text-xl font-semibold">Documents in ${Utils.escapeHtml(kb.name)}</h2>
                </div>

                <div class="flex justify-between items-center mb-4">
                    <p class="text-gray-600">${documents.length} document(s)</p>
                    <div class="flex gap-2">
                        <button id="ingest-btn" class="btn btn-primary">+ Ingest Document</button>
                        <button id="upload-files-btn" class="btn btn-secondary">+ Upload Files</button>
                    </div>
                </div>

                <!-- Pending Operations Section -->
                <div id="pending-operations" class="card mb-4 bg-blue-50 border-blue-200" style="${pendingOps.length === 0 ? 'display:none;' : ''}">
                    <div class="flex items-center mb-2">
                        <svg class="animate-spin h-5 w-5 text-blue-600 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                        </svg>
                        <h3 class="font-semibold text-blue-800">Ingestion in Progress</h3>
                    </div>
                    <table class="data-table text-sm">
                        <thead>
                            <tr>
                                <th>Source</th>
                                <th>Status</th>
                                <th>Started</th>
                            </tr>
                        </thead>
                        <tbody id="pending-ops-body">
                            ${pendingOps.map(renderOperationRow).join('')}
                        </tbody>
                    </table>
                </div>

                <!-- Documents Table -->
                ${documents.length > 0 ? `
                    <div class="card p-0 overflow-hidden">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>Title</th>
                                    <th>Source File</th>
                                    <th>Chunks</th>
                                    <th>Status</th>
                                    <th>Created</th>
                                    <th>Actions</th>
                                </tr>
                            </thead>
                            <tbody>
                                ${documents.map(doc => `
                                    <tr>
                                        <td class="font-medium">${Utils.escapeHtml(doc.title || 'Untitled')}</td>
                                        <td class="font-mono text-sm">${Utils.escapeHtml(doc.source_filename || '-')}</td>
                                        <td>${doc.chunk_count}</td>
                                        <td>
                                            ${doc.disabled ?
                                                '<span class="badge badge-gray">Disabled</span>' :
                                                '<span class="badge badge-success">Active</span>'
                                            }
                                        </td>
                                        <td class="text-xs text-gray-500">${Utils.formatDate(doc.created_at)}</td>
                                        <td>
                                            <button class="view-chunks-btn btn-sm btn-gray-sm mr-1" data-id="${doc.id}">Chunks</button>
                                            ${doc.disabled ?
                                                `<button class="enable-doc-btn btn-sm btn-success-sm mr-1" data-id="${doc.id}">Enable</button>` :
                                                `<button class="disable-doc-btn btn-sm btn-yellow-sm mr-1" data-id="${doc.id}">Disable</button>`
                                            }
                                            <button class="delete-doc-btn btn-sm btn-delete" data-id="${doc.id}">Delete</button>
                                        </td>
                                    </tr>
                                `).join('')}
                            </tbody>
                        </table>
                    </div>
                ` : Utils.renderEmpty('No documents yet. Use "Ingest Document" to add one.')}

                <!-- Hidden chunks container -->
                <div id="chunks-container" class="mt-4" style="display: none;">
                    <h3 class="text-lg font-medium mb-2">Document Chunks</h3>
                    <div id="chunks-content"></div>
                </div>
            </div>
        `;
    }

    function bindDocumentsEvents(kbId) {
        $('#back-btn').on('click', () => {
            stopPollingIngestions();
            render();
        });

        // Ingestion buttons
        $('#ingest-btn').on('click', () => showIngestModal(kbId));
        $('#upload-files-btn').on('click', () => showUploadFilesModal(kbId));

        // Document actions
        $('.view-chunks-btn').on('click', async function() {
            const docId = $(this).data('id');
            await viewChunks(kbId, docId);
        });

        $('.delete-doc-btn').on('click', async function() {
            const docId = $(this).data('id');
            await deleteDocument(kbId, docId);
        });

        $('.disable-doc-btn').on('click', async function() {
            const docId = $(this).data('id');
            await disableDocument(kbId, docId);
        });

        $('.enable-doc-btn').on('click', async function() {
            const docId = $(this).data('id');
            await enableDocument(kbId, docId);
        });
    }

    // Document functions
    async function viewChunks(kbId, docId) {
        try {
            const data = await API.getDocumentChunks(kbId, docId);
            const chunks = data.chunks || [];

            const chunksHtml = chunks.map((chunk) => `
                <div class="card mb-2">
                    <div class="flex justify-between items-center mb-2">
                        <span class="font-mono text-sm text-gray-500">Chunk ${chunk.chunk_index + 1}</span>
                        <span class="text-xs text-gray-400">${chunk.content.length} chars ${chunk.token_count ? `/ ${chunk.token_count} tokens` : ''}</span>
                    </div>
                    <pre class="bg-gray-50 p-2 rounded text-sm overflow-x-auto whitespace-pre-wrap">${Utils.escapeHtml(chunk.content.substring(0, 500))}${chunk.content.length > 500 ? '...' : ''}</pre>
                </div>
            `).join('');

            $('#chunks-container').show();
            $('#chunks-content').html(chunksHtml || '<p class="text-gray-500">No chunks found</p>');
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    async function deleteDocument(kbId, docId) {
        if (!Utils.confirm('Are you sure you want to delete this document and all its chunks?')) {
            return;
        }

        try {
            await API.deleteDocument(kbId, docId);
            Utils.showToast('Document deleted successfully', 'success');
            showDocuments(kbId);
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    async function disableDocument(kbId, docId) {
        try {
            await API.disableDocument(kbId, docId);
            Utils.showToast('Document disabled', 'success');
            showDocuments(kbId);
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    async function enableDocument(kbId, docId) {
        try {
            await API.enableDocument(kbId, docId);
            Utils.showToast('Document enabled', 'success');
            showDocuments(kbId);
        } catch (error) {
            Utils.showToast(error.message, 'error');
        }
    }

    function showIngestModal(kbId) {
        const modalHtml = `
            <div id="ingest-modal" class="modal-backdrop">
                <div class="modal-content" style="max-width: 600px;">
                    <div class="modal-header">
                        <h3 class="text-lg font-semibold">Ingest Document</h3>
                        <button id="close-modal" class="text-gray-500 hover:text-gray-700">&times;</button>
                    </div>
                    <form id="ingest-form-v2">
                        <div class="modal-body">
                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Title (optional)</label>
                                <input type="text" name="title" class="form-input" placeholder="My Document">
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Description (optional)</label>
                                <textarea name="description" class="form-input" rows="2" placeholder="Optional description"></textarea>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Filename (optional)</label>
                                <input type="text" name="filename" class="form-input" placeholder="document.txt">
                                <p class="text-xs text-gray-500 mt-1">Used to detect parser type from extension.</p>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Content *</label>
                                <textarea name="content" class="form-input" rows="8" required placeholder="Paste your document content here..."></textarea>
                            </div>

                            <div class="grid grid-cols-2 gap-4 mb-4">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">Parser Type</label>
                                    <select name="parser_type" class="form-input">
                                        <option value="">Auto-detect</option>
                                        <option value="plain_text">Plain Text</option>
                                        <option value="markdown">Markdown</option>
                                        <option value="html">HTML</option>
                                        <option value="json">JSON</option>
                                    </select>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">Chunking Strategy</label>
                                    <select name="chunking_type" class="form-input">
                                        <option value="fixed_size">Fixed Size</option>
                                        <option value="sentence">Sentence</option>
                                        <option value="paragraph">Paragraph</option>
                                        <option value="recursive">Recursive</option>
                                    </select>
                                </div>
                            </div>

                            <div class="grid grid-cols-2 gap-4 mb-4">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">Chunk Size</label>
                                    <input type="number" name="chunk_size" class="form-input" value="1000" min="100" max="10000">
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">Chunk Overlap</label>
                                    <input type="number" name="chunk_overlap" class="form-input" value="200" min="0" max="1000">
                                </div>
                            </div>

                            <div class="mb-4">
                                <label class="block text-sm font-medium text-gray-700 mb-1">Metadata (JSON, optional)</label>
                                <textarea name="metadata" class="form-input font-mono text-sm" rows="3" placeholder='{"category": "docs", "author": "John"}'></textarea>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button type="button" id="cancel-ingest" class="btn btn-secondary">Cancel</button>
                            <button type="submit" class="btn btn-primary">Ingest</button>
                        </div>
                    </form>
                </div>
            </div>
        `;

        $('body').append(modalHtml);

        $('#close-modal, #cancel-ingest').on('click', () => {
            $('#ingest-modal').remove();
        });

        $('#ingest-form-v2').on('submit', async function(e) {
            e.preventDefault();

            const formData = Utils.getFormData(this);

            // Validate content
            if (!formData.content || formData.content.trim() === '') {
                Utils.showToast('Content is required', 'error');
                return;
            }

            // Parse metadata if provided
            let metadata = {};

            if (formData.metadata && formData.metadata.trim()) {
                try {
                    metadata = JSON.parse(formData.metadata);
                } catch (err) {
                    Utils.showToast('Invalid metadata JSON', 'error');
                    return;
                }
            }

            const request = {
                content: formData.content,
                metadata
            };

            if (formData.title) request.title = formData.title;
            if (formData.description) request.description = formData.description;
            if (formData.filename) request.filename = formData.filename;
            if (formData.parser_type) request.parser_type = formData.parser_type;
            if (formData.chunking_type) request.chunking_type = formData.chunking_type;
            if (formData.chunk_size) request.chunk_size = parseInt(formData.chunk_size, 10);
            if (formData.chunk_overlap) request.chunk_overlap = parseInt(formData.chunk_overlap, 10);

            const $btn = $(this).find('button[type="submit"]');
            $btn.prop('disabled', true).text('Ingesting...');

            try {
                await API.ingestDocument(kbId, request);
                Utils.showToast('Document ingested successfully', 'success');
                $('#ingest-modal').remove();
                showDocuments(kbId);
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text('Ingest');
            }
        });
    }

    function showUploadFilesModal(kbId) {
        const modalHtml = `
            <div id="upload-files-modal" class="modal-backdrop">
                <div class="modal-content" style="max-width: 500px;">
                    <div class="modal-header">
                        <h3 class="text-lg font-semibold">Upload Files</h3>
                        <button id="close-upload-modal" class="text-gray-500 hover:text-gray-700">&times;</button>
                    </div>
                    <form id="upload-files-form">
                        <div class="modal-body">
                            <div class="mb-4">
                                <label class="form-label">Select Files</label>
                                <input type="file" id="files-input" multiple
                                    accept=".txt,.md,.html,.htm,.json,.csv,.xml"
                                    class="w-full p-2 border rounded cursor-pointer">
                                <p class="text-xs text-gray-500 mt-1">
                                    Supported: .txt, .md, .html, .json, .csv, .xml (multiple files allowed)
                                </p>
                            </div>
                            <div id="selected-files" class="mb-4" style="display:none;">
                                <label class="form-label">Selected Files</label>
                                <div id="files-list" class="text-sm bg-gray-50 rounded p-2 max-h-32 overflow-y-auto"></div>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button type="button" id="cancel-upload" class="btn btn-secondary">Cancel</button>
                            <button type="submit" id="upload-btn" class="btn btn-primary" disabled>Upload Files</button>
                        </div>
                    </form>
                </div>
            </div>
        `;

        $('body').append(modalHtml);

        $('#close-upload-modal, #cancel-upload').on('click', () => {
            $('#upload-files-modal').remove();
        });

        $('#files-input').on('change', function() {
            const files = this.files;

            if (files.length > 0) {
                const filesList = Array.from(files).map(f =>
                    `<div class="flex justify-between py-1 border-b last:border-0">
                        <span>${Utils.escapeHtml(f.name)}</span>
                        <span class="text-gray-400">${(f.size / 1024).toFixed(1)} KB</span>
                    </div>`
                ).join('');
                $('#files-list').html(filesList);
                $('#selected-files').show();
                $('#upload-btn').prop('disabled', false);
            } else {
                $('#selected-files').hide();
                $('#upload-btn').prop('disabled', true);
            }
        });

        $('#upload-files-form').on('submit', async function(e) {
            e.preventDefault();
            const files = $('#files-input')[0].files;

            if (files.length === 0) {
                Utils.showToast('Please select at least one file', 'error');
                return;
            }

            const $btn = $('#upload-btn');
            $btn.prop('disabled', true).text('Uploading...');

            try {
                const result = await API.ingestFiles(kbId, files);
                Utils.showToast(`${result.total} file(s) queued for ingestion`, 'success');
                $('#upload-files-modal').remove();
                showDocuments(kbId);
            } catch (error) {
                Utils.showToast(error.message, 'error');
                $btn.prop('disabled', false).text('Upload Files');
            }
        });
    }

    return { render };
})();
