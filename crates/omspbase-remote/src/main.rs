use std::process;

mod config;
mod control;
mod decode;
mod transport;

// ponytail: no direct CoreError use in main — errors handled via config load / pipeline API
use omspbase_core::metrics::CoreMetrics;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    tracing::info!("OMSPBase Remote v{} starting", env!("CARGO_PKG_VERSION"));

    // Parse config path
    let config_path = std::env::args()
        .nth(2)
        .filter(|a| a == "--config")
        .and_then(|_| std::env::args().nth(3))
        .unwrap_or_else(|| "/opt/oomspbase/etc/remote.conf".to_string());

    let config = match config::load(&config_path) {
        Ok(c) => {
            tracing::info!("Config loaded: server={}", c.server.signaling_url);
            c
        }
        Err(e) => {
            tracing::error!("Failed to load config from {}: {e}", config_path);
            process::exit(1);
        }
    };

    // ponytail: media/control params hardcoded for MVP; promote to config when config schema extends
    let display_name = "default";
    let width: u32 = 1280;
    let height: u32 = 720;
    let decoder: Option<&str> = None;
    let hmac_key = "omspbase-control";
    let rate_hz: u32 = 30;

    // Phase 1: Create control sender (signs commands with HMAC before DataChannel send)
    let _control_sender = control::ControlSender::new(hmac_key, rate_hz);

    // Phase 2: Start GStreamer decode pipeline (if enabled and configured)
    let mut pipeline = decode::DecodePipeline::new(display_name, width, height, decoder);
    if let Err(e) = pipeline.start() {
        tracing::error!("Decode pipeline start failed: {e}");
        process::exit(1);
    }

    // Phase 3: Build axum router (health + config + metrics)
    let metrics = std::sync::Arc::new(CoreMetrics::new());
    let metrics_arc = metrics.clone();
    let app = axum::Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/config", axum::routing::get(config_handler))
        .route("/metrics", axum::routing::get(move || {
            let m = metrics_arc.clone();
            async move { m.encode() }
        }));

    let bind_addr = "0.0.0.0:9101";

    let listener = match tokio::net::TcpListener::bind(bind_addr).await {
        Ok(l) => {
            tracing::info!("Config UI listening on {bind_addr}");
            l
        }
        Err(e) => {
            tracing::error!("Failed to bind {bind_addr}: {e}");
            process::exit(1);
        }
    };

    // Phase 5: Connect to server signaling (transport stub)
    let transport = transport::Transport::new(&config.server.signaling_url);
    if let Err(e) = transport.connect().await {
        tracing::error!("Transport connect failed: {e}");
        process::exit(1);
    }
    tracing::info!("Signaling connection placeholder (psk: {psk_len} chars)", psk_len = config.psk.as_deref().unwrap_or("omspbase-dev").len());

    // Report ready: active_connections bumped for startup
    metrics.active_connections.inc();
    tracing::info!("Remote ready (server: {})", config.server.signaling_url);

    // Run server with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");
    });

    if let Err(e) = server.await {
        tracing::error!("Server error: {e}");
    }

    // Stop pipeline
    if let Err(e) = pipeline.stop() {
        tracing::error!("Decode pipeline stop error: {e}");
    }

    metrics.active_connections.dec();
    tracing::info!("Shutdown complete");
}

/// Health check endpoint.
async fn health_handler() -> &'static str {
    "OK"
}

/// Config status endpoint (safe subset, no secrets).
async fn config_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

