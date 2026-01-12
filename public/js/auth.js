/**
 * Authentication handling with JWT tokens
 */
const Auth = (function() {
    const TOKEN_KEY = 'pmp_admin_token';
    const USER_KEY = 'pmp_admin_user';

    function getToken() {
        return sessionStorage.getItem(TOKEN_KEY);
    }

    function setToken(token) {
        sessionStorage.setItem(TOKEN_KEY, token);
    }

    function clearToken() {
        sessionStorage.removeItem(TOKEN_KEY);
        sessionStorage.removeItem(USER_KEY);
    }

    function getUser() {
        const userJson = sessionStorage.getItem(USER_KEY);

        if (!userJson) return null;
        try {
            return JSON.parse(userJson);
        } catch (e) {
            return null;
        }
    }

    function setUser(user) {
        sessionStorage.setItem(USER_KEY, JSON.stringify(user));
    }

    function isAuthenticated() {
        return !!getToken();
    }

    function showLoginModal() {
        $('#auth-modal').removeClass('hidden');
        $('#app').addClass('hidden');
        $('#username-input').val('').focus();
        $('#password-input').val('');
        $('#login-error').addClass('hidden');
    }

    function hideLoginModal() {
        $('#auth-modal').addClass('hidden');
        $('#app').removeClass('hidden');
    }

    async function login(username, password) {
        const response = await fetch('/auth/login', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ username, password })
        });

        if (!response.ok) {
            let errorMessage = 'Login failed';
            try {
                const error = await response.json();
                errorMessage = error.error?.message || error.message || errorMessage;
            } catch (e) {
                // Ignore JSON parse errors
            }
            throw new Error(errorMessage);
        }

        const data = await response.json();
        setToken(data.token);
        setUser(data.user);

        return data;
    }

    async function logout() {
        try {
            const token = getToken();

            if (token) {
                await fetch('/auth/logout', {
                    method: 'POST',
                    headers: {
                        'Authorization': `Bearer ${token}`
                    }
                });
            }
        } catch (e) {
            // Ignore logout errors
        } finally {
            clearToken();
            showLoginModal();
        }
    }

    function showError(message) {
        $('#login-error').text(message).removeClass('hidden');
    }

    function init() {
        // Handle login form submission
        $('#login-form').on('submit', async function(e) {
            e.preventDefault();
            const username = $('#username-input').val().trim();
            const password = $('#password-input').val();

            if (!username) {
                showError('Please enter a username');
                return;
            }

            if (!password) {
                showError('Please enter a password');
                return;
            }

            const $btn = $(this).find('button[type="submit"]');
            const originalText = $btn.text();
            $btn.prop('disabled', true).text('Logging in...');

            try {
                await login(username, password);
                hideLoginModal();
                // Trigger initial navigation
                App.navigate(window.location.hash.slice(1) || 'dashboard');
            } catch (e) {
                showError(e.message || 'Invalid username or password');
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
        getToken,
        setToken,
        clearToken,
        getUser,
        isAuthenticated,
        showLoginModal,
        hideLoginModal,
        login,
        logout,
        init
    };
})();
