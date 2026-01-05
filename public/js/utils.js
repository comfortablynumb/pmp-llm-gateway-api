/**
 * Utility functions for the Admin UI
 */
const Utils = (function() {
    /**
     * Escape HTML to prevent XSS
     */
    function escapeHtml(text) {
        if (text === null || text === undefined) return '';
        const div = document.createElement('div');
        div.textContent = String(text);
        return div.innerHTML;
    }

    /**
     * Show a toast notification
     */
    function showToast(message, type = 'info') {
        const container = $('#toast-container');
        const toast = $(`<div class="toast toast-${type}">${escapeHtml(message)}</div>`);
        container.append(toast);

        setTimeout(() => {
            toast.addClass('toast-exit');
            setTimeout(() => toast.remove(), 300);
        }, 3000);
    }

    /**
     * Render loading spinner
     */
    function renderLoading() {
        return `
            <div class="flex items-center justify-center h-64">
                <div class="spinner"></div>
            </div>
        `;
    }

    /**
     * Render error message
     */
    function renderError(message) {
        return `
            <div class="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
                <strong>Error:</strong> ${escapeHtml(message)}
            </div>
        `;
    }

    /**
     * Get form data as object
     */
    function getFormData(form) {
        const formData = new FormData(form);
        const data = {};

        formData.forEach((value, key) => {
            if (value === '') return;

            // Handle nested keys (e.g., config.temperature)
            const keys = key.split('.');
            let obj = data;

            for (let i = 0; i < keys.length - 1; i++) {
                if (!obj[keys[i]]) obj[keys[i]] = {};
                obj = obj[keys[i]];
            }

            // Try to parse numbers
            const lastKey = keys[keys.length - 1];
            const numValue = parseFloat(value);

            if (!isNaN(numValue) && isFinite(numValue)) {
                obj[lastKey] = numValue;
            } else if (value === 'true') {
                obj[lastKey] = true;
            } else if (value === 'false') {
                obj[lastKey] = false;
            } else {
                obj[lastKey] = value;
            }
        });

        // Handle checkboxes (unchecked ones aren't in FormData)
        $(form).find('input[type="checkbox"]').each(function() {
            const name = $(this).attr('name');

            if (name) {
                const keys = name.split('.');
                let obj = data;

                for (let i = 0; i < keys.length - 1; i++) {
                    if (!obj[keys[i]]) obj[keys[i]] = {};
                    obj = obj[keys[i]];
                }

                obj[keys[keys.length - 1]] = $(this).is(':checked');
            }
        });

        return data;
    }

    /**
     * Format date string
     */
    function formatDate(dateStr) {
        if (!dateStr) return 'N/A';

        const date = new Date(dateStr);
        return date.toLocaleString();
    }

    /**
     * Truncate string
     */
    function truncate(str, maxLen = 50) {
        if (!str) return '';
        if (str.length <= maxLen) return str;
        return str.substring(0, maxLen) + '...';
    }

    /**
     * Confirm dialog
     */
    function confirm(message) {
        return window.confirm(message);
    }

    /**
     * Render empty state
     */
    function renderEmpty(message = 'No items found') {
        return `
            <div class="text-center py-12 text-gray-500">
                <p>${escapeHtml(message)}</p>
            </div>
        `;
    }

    /**
     * Debounce function
     */
    function debounce(func, wait) {
        let timeout;

        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }

    return {
        escapeHtml,
        showToast,
        renderLoading,
        renderError,
        getFormData,
        formatDate,
        truncate,
        confirm,
        renderEmpty,
        debounce
    };
})();
