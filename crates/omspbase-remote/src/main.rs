use std::process;

mod config;
mod control;
mod decode;

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

    tracing::info!(
        "OMSPBase Remote v{} starting",
        env!("CARGO_PKG_VERSION")
    );

    // Parse config path
    let config_path = std::env::args()
        .nth(2)
        .filter(|a| a == "--config")
        .and_then(|_| std::env::args().nth(3))
        .unwrap_or_else(|| "/opt/oomspbase/etc/remote.conf".to_string());

    let config = match config::load(&config_path) {
        Ok(c) => {
            tracing::info!("Config loaded: remote id={}", c.remote.id);
            c
        }
        Err(e) => {
            tracing::error!("Failed to load config from {}: {}", config_path, e);
            process::exit(1);
        }
    };

    // Phase 1: Create control sender (signs frames with HMAC before DataChannel send)
    let control_config = config.control.as_ref();
    let hmac_key = control_config
        .map(|c| c.hmac_key.as_str())
        .unwrap_or("omspbase-control");
    let rate_hz = control_config
        .map(|c| c.rate_hz)
        .unwrap_or(30);
    let _control_sender = control::ControlSender::new(hmac_key, rate_hz);

    // Phase 2: Start GStreamer decode pipeline (if enabled and configured)
    let display_name = config.media.display.as_deref().unwrap_or("default");
    let mut pipeline = decode::DecodePipeline::new(
        display_name,
        config.media.width,
        config.media.height,
        config.media.decoder.as_deref(),
    );
    if let Err(e) = pipeline.start() {
        tracing::error!("Decode pipeline start failed: {}", e);
        process::exit(1);
    }

    // Phase 3: Build axum router (config UI + health endpoint)
    let app = axum::Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/config", axum::routing::get(config_handler));

    // Phase 4: Connect to Server signaling
    tracing::info!(
        "Connecting to signaling server at {}",
        config.signaling.ws_url
    );
    let _signaling_url = config.signaling.ws_url.clone();
    // ponytail: WebSocket signaling connect deferred — integrate with real WebRTC stack
    tracing::info!("Signaling connection placeholder (psk: {psk_len} chars)", psk_len = config.signaling.psk.len());

    // Determine bind address for config UI
    let bind_addr = config
        .web
        .as_ref()
        .map(|w| w.bind.as_str())
        .unwrap_or("0.0.0.0:9101");

    let listener = match tokio::net::TcpListener::bind(bind_addr).await {
        Ok(l) => {
            tracing::info!("Config UI listening on {}", bind_addr);
            l
        }
        Err(e) => {
            tracing::error!("Failed to bind {}: {}", bind_addr, e);
            process::exit(1);
        }
    };

    tracing::info!("Remote {} ready", config.remote.id);

    // Run server with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");
    });

    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }

    // Stop pipeline
    if let Err(e) = pipeline.stop() {
        tracing::error!("Decode pipeline stop error: {}", e);
    }

    tracing::info!("Shutdown complete");
}

/// Health check endpoint
async fn health_handler() -> &'static str {
    "OK"
}

/// Config status endpoint (safe subset, no secrets)
async fn config_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
