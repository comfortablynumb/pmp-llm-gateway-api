/**
 * Authentication handling
 */
const Auth = (function() {
    const STORAGE_KEY = 'pmp_admin_api_key';

    function getApiKey() {
        return sessionStorage.getItem(STORAGE_KEY);
    }

    function setApiKey(key) {
        sessionStorage.setItem(STORAGE_KEY, key);
    }

    function clearApiKey() {
        sessionStorage.removeItem(STORAGE_KEY);
    }

    function isAuthenticated() {
        return !!getApiKey();
    }

    function showLoginModal() {
        $('#auth-modal').removeClass('hidden');
        $('#app').addClass('hidden');
        $('#api-key-input').val('').focus();
        $('#login-error').addClass('hidden');
    }

    function hideLoginModal() {
        $('#auth-modal').addClass('hidden');
        $('#app').removeClass('hidden');
    }

    async function validateAndStore(apiKey) {
        // Temporarily set key to test it
        setApiKey(apiKey);

        try {
            // Try to fetch models to validate the key has admin access
            await API.listModels();
            hideLoginModal();
            return true;
        } catch (e) {
            clearApiKey();
            return false;
        }
    }

    function showError(message) {
        $('#login-error').text(message).removeClass('hidden');
    }

    function init() {
        // Handle login form submission
        $('#login-form').on('submit', async function(e) {
            e.preventDefault();
            const apiKey = $('#api-key-input').val().trim();

            if (!apiKey) {
                showError('Please enter an API key');
                return;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Validating...');

            try {
                const valid = await validateAndStore(apiKey);

                if (!valid) {
                    showError('Invalid API key or insufficient permissions');
                } else {
                    // Trigger initial navigation
                    App.navigate(window.location.hash.slice(1) || 'dashboard');
                }
            } catch (e) {
                showError('Failed to validate API key');
            } finally {
                $btn.prop('disabled', false).text(originalText);
            }
        });

        // Check authentication on load
        if (!isAuthenticated()) {
            showLoginModal();
        } else {
            hideLoginModal();
        }
    }

    return {
        getApiKey,
        setApiKey,
        clearApiKey,
        isAuthenticated,
        showLoginModal,
        hideLoginModal,
        validateAndStore,
        init
    };
})();
