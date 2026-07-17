use std::process;

mod config;
mod monitor;
mod relay;
mod signaling;

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
        "OMSPBase Server v{} starting",
        env!("CARGO_PKG_VERSION")
    );

    // Parse config
    let config_path = std::env::args()
        .nth(2)
        .filter(|a| a == "--config")
        .and_then(|_| std::env::args().nth(3))
        .unwrap_or_else(|| "/opt/oomspbase/etc/server.conf".to_string());

    let config = match config::load(&config_path) {
        Ok(c) => {
            tracing::info!("Config loaded: {:?}", c);
            c
        }
        Err(e) => {
            tracing::error!("Failed to load config from {}: {}", config_path, e);
            process::exit(1);
        }
    };

    // Create the signaling server (shared state for WebSocket rooms)
    let signaling_server = signaling::SignalingServer::new();

    // Create the WebRTC relay (stub unless `webrtc` feature is enabled)
    let _relay = relay::Relay::new();

    // Build axum router
    let signaling_router = signaling::signaling_router(signaling_server.clone());
    let monitor_router = monitor::monitor_router(signaling_server.clone());

    let app = axum::Router::new()
        .merge(signaling_router)
        .merge(monitor_router);

    // Determine bind address
    let bind_addr = config
        .web
        .as_ref()
        .map(|w| w.bind.as_str())
        .unwrap_or("0.0.0.0:9801");

    let listener = match tokio::net::TcpListener::bind(bind_addr).await {
        Ok(l) => {
            tracing::info!("Listening on {}", bind_addr);
            l
        }
        Err(e) => {
            tracing::error!("Failed to bind {}: {}", bind_addr, e);
            process::exit(1);
        }
    };

    // Notify systemd we're ready
    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]);
    tracing::info!("Server {} ready", config.server.id);

    // Run server with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");
    });

    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }

    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Stopping]);
    tracing::info!("Shutdown complete");
}
