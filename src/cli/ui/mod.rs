//! UI command - runs UI server with optional API proxy

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use clap::Args;
use reqwest::Client;
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

use crate::config::AppConfig;
use crate::infrastructure::logging;

/// Arguments for the UI command
#[derive(Args, Clone)]
pub struct UiArgs {
    /// API URL to proxy requests to
    #[arg(long, default_value = "http://localhost:3001")]
    pub api_url: String,

    /// Skip proxying - serve static files only
    #[arg(long)]
    pub skip_proxy: bool,

    /// Port to serve UI on (overrides config)
    #[arg(long)]
    pub port: Option<u16>,
}

/// Run the UI server
pub async fn run(args: UiArgs) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::load().unwrap_or_default();
    init_logging(&config);

    let app = create_ui_router(&args);

    let port = args.port.unwrap_or(config.server.port);
    let addr = SocketAddr::from((
        config.server.host.parse::<std::net::IpAddr>()?,
        port,
    ));

    if args.skip_proxy {
        info!("Starting UI server on {} (static files only)", addr);
    } else {
        info!(
            "Starting UI server on {} (proxying /api/* to {})",
            addr, args.api_url
        );
    }

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_logging(config: &AppConfig) {
    logging::init_logging(&logging::LoggingConfig {
        level: config.logging.level.clone(),
        format: config.logging.format.clone(),
    });
}

/// Create UI router with optional API proxy
fn create_ui_router(args: &UiArgs) -> Router {
    let static_service = ServeDir::new("public").fallback(ServeFile::new("public/index.html"));

    if args.skip_proxy {
        // Static files only
        Router::new().nest_service("/", static_service)
    } else {
        // API proxy + static files
        let proxy_state = ProxyState {
            api_url: args.api_url.clone(),
            client: Client::new(),
        };

        Router::new()
            .route("/api/*path", any(proxy_handler))
            .with_state(proxy_state)
            .nest_service("/", static_service)
    }
}

#[derive(Clone)]
struct ProxyState {
    api_url: String,
    client: Client,
}

async fn proxy_handler(
    State(state): State<ProxyState>,
    req: Request<Body>,
) -> impl IntoResponse {
    let path = req.uri().path();

    // Rewrite /api/* to /admin/*
    let target_path = path.replacen("/api", "/admin", 1);
    let target_url = format!("{}{}", state.api_url, target_path);

    // Add query string if present
    let target_url = if let Some(query) = req.uri().query() {
        format!("{}?{}", target_url, query)
    } else {
        target_url
    };

    match forward_request(&state.client, req, &target_url).await {
        Ok(response) => response,
        Err(e) => {
            error!("Proxy error: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                format!("Proxy error: {}", e),
            )
                .into_response()
        }
    }
}

async fn forward_request(
    client: &Client,
    req: Request<Body>,
    target_url: &str,
) -> Result<Response, anyhow::Error> {
    let method = req.method().clone();
    let headers = req.headers().clone();

    // Build the proxied request
    let mut proxy_req = client.request(method, target_url);

    // Copy headers (except host)
    for (key, value) in headers.iter() {
        if key != "host" {
            proxy_req = proxy_req.header(key, value);
        }
    }

    // Copy body
    let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX).await?;

    if !body_bytes.is_empty() {
        proxy_req = proxy_req.body(body_bytes);
    }

    // Send request
    let response = proxy_req.send().await?;

    // Build response
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await?;

    let mut builder = Response::builder().status(status);

    for (key, value) in headers.iter() {
        builder = builder.header(key, value);
    }

    Ok(builder.body(Body::from(body))?)
}
