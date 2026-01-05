/**
 * Main application - routing and initialization
 */
const App = (function() {
    const routes = {
        'dashboard': Dashboard,
        'models': Models,
        'prompts': Prompts,
        'api-keys': ApiKeys,
        'workflows': Workflows,
        'credentials': Credentials
    };

    const titles = {
        'dashboard': 'Dashboard',
        'models': 'Models',
        'prompts': 'Prompts',
        'api-keys': 'API Keys',
        'workflows': 'Workflows',
        'credentials': 'Credentials'
    };

    let currentView = null;

    function navigate(route) {
        if (!route) route = 'dashboard';

        // Update URL hash
        if (window.location.hash.slice(1) !== route) {
            window.location.hash = route;
        }

        // Update active nav item
        $('.nav-item').removeClass('active');
        $(`.nav-item[data-route="${route}"]`).addClass('active');

        // Update page title
        $('#page-title').text(titles[route] || 'Dashboard');

        // Load view
        const ViewModule = routes[route];

        if (ViewModule) {
            currentView = route;
            ViewModule.render();
        } else {
            Dashboard.render();
        }
    }

    function init() {
        // Initialize authentication
        Auth.init();

        // Setup navigation click handlers
        $('.nav-item').on('click', function(e) {
            e.preventDefault();
            const route = $(this).data('route');
            navigate(route);
        });

        // Handle hash changes
        $(window).on('hashchange', function() {
            if (Auth.isAuthenticated()) {
                const hash = window.location.hash.slice(1) || 'dashboard';
                navigate(hash);
            }
        });

        // Setup logout
        $('#logout-btn').on('click', function() {
            Auth.clearApiKey();
            Auth.showLoginModal();
        });

        // Initial navigation if authenticated
        if (Auth.isAuthenticated()) {
            const hash = window.location.hash.slice(1) || 'dashboard';
            navigate(hash);
        }
    }

    return { init, navigate };
})();

// Initialize app when DOM is ready
$(document).ready(function() {
    App.init();
});
