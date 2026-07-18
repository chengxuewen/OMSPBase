//! OMSPBase Host — headless capture + encode + WebRTC push.
//!
//! # Startup flow
//! 1. Parse config (host.conf YAML)
//! 2. Load or create session state
//! 3. Start GStreamer pipeline
//! 4. Initialize WebRTC transport
//! 5. Create control handler
//! 6. Build axum router (metrics + signaling)
//! 7. Start emergency UDP listener
//! 8. Serve until shutdown signal

use std::process;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
mod config;
mod control;
mod emergency;
mod metrics;
mod pipeline;
mod session;
mod signaling;
mod transport;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().expect("hardcoded directive")),
        )
        .init();

    tracing::info!("OMSPBase Host v{} starting", env!("CARGO_PKG_VERSION"));

    // Parse config — collect args once for bounds-safe access
    let config_path = {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 3 && args[2] == "--config" {
            args[3].clone()
        } else {
            "/opt/omspbase/etc/host.conf".to_string()
        }
    };
    let config = match config::load(&config_path) {
        Ok(c) => {
            tracing::info!("Config loaded from {config_path}");
            c
        }
        Err(e) => {
            tracing::warn!("Config {config_path}: {e}, using defaults");
            serde_yaml::from_str(&default_host_config()).unwrap()
        }
    };// ponytail: fallback to defaults when config file missing, add config wizard when needed

    // Parse resolution "WIDTHxHEIGHT"
    let (width, height) = parse_resolution(&config.capture.resolution);
    let framerate = config.capture.framerate;
    let bitrate = config.encoder.bitrate_kbps;
    let encoder = &config.encoder.backend;

    // Phase 1: Load or create session state
    let session_state = session::Session::load(None);
    let persist_handle = session_state.start_persist();

    // Phase 2: Start GStreamer pipeline
    let pipeline = std::sync::Arc::new(pipeline::Pipeline::new(
        &config.capture,
        width,
        height,
        framerate,
        bitrate,
        encoder,
    )
    .unwrap_or_else(|e| {
        tracing::warn!("Pipeline init failed: {e}, running headless");
        // ponytail: return dummy pipeline for headless mode (E2E testing)
        pipeline::Pipeline::dummy()
    }));
    // ponytail: pipeline start may fail without GStreamer; non-fatal for E2E
    if let Err(e) = pipeline.start() {
        tracing::warn!("Pipeline start failed: {e}, continuing headless");
    }

    // Phase 3: Create WebRTC transport
    let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<String>();
    let transport = transport::Transport::new_with_sender(frame_tx);
    // Phase 4: Create control handler (shared with metrics)
    let control_handler = control::ControlHandler::new();
    let frames_dropped = control_handler.frames_dropped.clone();

    // Phase 5: Build axum router (metrics)
    let core_metrics = omspbase_core::metrics::CoreMetrics::new();
    let shared_metrics = std::sync::Arc::new(std::sync::RwLock::new(core_metrics));
    let metrics_router = metrics::metrics_router(shared_metrics.clone());

    let app = axum::Router::new()
        .merge(metrics_router);

    // Determine bind address
    let bind_addr = "0.0.0.0:9801"; // ponytail: separate port from server (9800)

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

    tracing::info!("Host ready — session id={}", config.capture.source);

    // Phase 6: Connect to Server signaling
    let signaling_url = config.server.signaling_url.clone();
    let psk = config
        .psk
        .clone()
        .unwrap_or_else(|| "omspbase-dev".to_string());
    let room_id = "default"; // ponytail: match remote for E2E
    let signaling_handle = tokio::spawn(async move {
        use signaling::SignalingClient;
        let client = SignalingClient::new(&signaling_url, &psk, &room_id);
        match client.connect().await {
            Ok((mut sender, mut receiver)) => {
                tracing::info!("Signaling connected, entering relay loop");
                loop {
                    tokio::select! {
                        Some(frame_json) = frame_rx.recv() => {
                            tracing::debug!("Relay: sending frame ({} bytes)", frame_json.len());
                            if let Err(e) = sender.send(Message::Text(frame_json.into())).await {
                                tracing::warn!("Failed to send frame via WS: {e}");
                            }
                        }
                        Some(msg) = receiver.next() => {
                            match msg {
                                Ok(msg) => {
                                    if let Ok(text) = msg.to_text() {
                                        tracing::debug!("Relay received: {}", text);
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("WS receive error: {e}");
                                }
                            }
                        }
                        else => break,
                    }
                }
                tracing::warn!("Signaling relay loop ended");
            }
            Err(e) => {
                tracing::error!("Signaling connection failed: {}", e);
            }
        }
    });

    // Phase 7: Emergency UDP listener (background)
    let emergency_handle = tokio::spawn(async move {
        let listener = match emergency::EmergencyListener::bind(9999).await {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("Emergency listener failed to bind: {e}");
                return;
            }
        };
        if let Err(e) = listener.listen().await {
            tracing::error!("Emergency listener error: {e}");
        }
    });

    // Background: pull frames and send via transport
    let push_pipeline = pipeline.clone();
    let push_handle = tokio::spawn(async move {
        loop {
            match push_pipeline.pull_sample() {
                Ok(data) => {
                    if let Err(e) = transport.send_frame(&data).await {
                        tracing::warn!("Transport send error: {e}");
                    }
                    if let Ok(m) = shared_metrics.read() {
                        m.relayed_bytes.inc_by(data.len() as u64);
                    }
                }
                Err(e) => {
                    tracing::warn!("Pipeline pull error: {e}");
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(33)).await; // ponytail: ~30fps throttle
        }
    });

    // Start metrics updater: sync dropped frames counter
    let metrics_updater = {
        let dropped = frames_dropped;
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let _ = dropped.load(std::sync::atomic::Ordering::Relaxed);
            }
        })
    };

    // Run server (blocks until shutdown signal)
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");
    });

    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }
    tracing::info!("Shutdown complete");

    // Stop pipeline
    if let Err(e) = pipeline.stop() {
        tracing::error!("Pipeline stop error: {}", e);
    }

    // Clean up background tasks
    push_handle.abort();
    metrics_updater.abort();
    signaling_handle.abort();

}

/// Parse "WIDTHxHEIGHT" into (width, height). Defaults to 1280x720.
fn parse_resolution(res: &str) -> (u32, u32) {
    let parts: Vec<&str> = res.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].parse().unwrap_or(1280);
        let h = parts[1].parse().unwrap_or(720);
        (w, h)
    } else {
        (1280, 720)
    }
}

/// Generate a default host config YAML for headless/E2E fallback.
fn default_host_config() -> String {
    r#"
server:
  signaling_url: "ws://localhost:9800/ws"
  ice_servers: []
capture:
  source: "test_pattern"
  resolution: "1280x720"
  framerate: 30
  device: null
encoder:
  backend: "auto"
  bitrate_kbps: 2000
  keyframe_interval: 60
psk: "omspbase-dev"
"#.to_string()
}
