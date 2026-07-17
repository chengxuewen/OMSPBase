use std::process;

// Binary depends on omspbase-server lib (same package).
use omspbase_server::config;
use omspbase_server::monitor;
use omspbase_server::relay;
use omspbase_server::signaling;

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

    // Parse config — collect args once for bounds-safe access
    let config_path = {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 3 && args[2] == "--config" {
            args[3].clone()
        } else {
            "/opt/oomspbase/etc/server.conf".to_string()
        }
    };
    let config = match config::load(&config_path) {
        Ok(c) => {
            tracing::info!("Config loaded from {config_path}");
            c
        }
        Err(e) => {
            tracing::warn!("Config {config_path}: {e}, using defaults");
            serde_yaml::from_str(DEFAULT_SERVER_CONFIG).unwrap()
        }
    };// ponytail: fallback to defaults for E2E, add config wizard when needed

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

    // Bind address from omspbase_core config
    let bind_addr = format!("{}:{}", config.listen.host, config.listen.port);

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => {
            tracing::info!("Listening on {}", bind_addr);
            l
        }
        Err(e) => {
            tracing::error!("Failed to bind {}: {}", bind_addr, e);
            process::exit(1);
        }
    };

    // Notify systemd
    tracing::info!("Server ready on {}", bind_addr);

    // Run server with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");
    });

    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }

    tracing::info!("Shutdown complete");
}

/// Default server config for headless/E2E fallback.
const DEFAULT_SERVER_CONFIG: &str = r#"
listen:
  host: "0.0.0.0"
  port: 9800
room_capacity: 10
rate_limit: 100
psk: "omspbase-dev"
"#;
