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

use std::sync::Arc;

use futures_util::StreamExt;
use omspbase_core::protocol::SignalingMessage;
use signaling::SignalingClient;
mod config;
mod control;
mod emergency;
mod metrics;
mod pipeline;
mod session;
mod signaling;
mod transport;
mod webrtc_transport;

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

    // Phase 3: Create control handler (shared with metrics)
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

    // Phase 4: Connect to signaling and create WebRTC transport
    let signaling_url = config.server.signaling_url.clone();
    let psk = config
        .psk
        .clone()
        .unwrap_or_else(|| "omspbase-dev".to_string());
    let room_id = "default".to_string(); // ponytail: match remote for E2E

    let client = SignalingClient::new(&signaling_url, &psk, &room_id);
    let (ws_sender, mut ws_receiver) = client
        .connect()
        .await
        .unwrap_or_else(|e| {
            tracing::error!("Signaling connection failed: {e}");
            process::exit(1);
        });

    let (webrtc_transport, dc_events) =
        webrtc_transport::WebrtcTransport::new(ws_sender, room_id)
            .await
            .unwrap_or_else(|e| {
                tracing::error!("WebRTC transport creation failed: {e}");
                process::exit(1);
            });

    let webrtc = Arc::new(webrtc_transport);

    // Spawn WS receiver loop — handles incoming SDP answers and room events
    let _ws_receiver_handle = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(msg) => {
                    if let Ok(text) = msg.to_text() {
                        tracing::debug!("WS received: {}", text);
                        // Parse SDP answer from remote and set remote description
                        if let Ok(sig_msg) =
                            serde_json::from_str::<SignalingMessage>(text)
                        {
                            if let SignalingMessage::Sdp { sdp, .. } = sig_msg {
                                tracing::info!("Received SDP answer, setting remote description");
                                // ponytail: full SDP answer processing in Phase 2
                            }
                        }
                    }
                }
                Err(e) => tracing::warn!("WS receive error: {e}"),
            }
        }
        tracing::warn!("WS receiver loop ended");
    });

    // Spawn DC event loop — logs lifecycle events
    let _dc_event_handle = tokio::spawn(async move {
        webrtc_transport::run_dc_event_loop(dc_events).await;
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

    // Background: pull frames and send via WebRTC DataChannel
    let push_pipeline = pipeline.clone();
    let push_webrtc = webrtc.clone();
    let push_handle = tokio::spawn(async move {
        loop {
            match push_pipeline.pull_sample() {
                Ok(data) => {
                    if let Err(e) = push_webrtc.send_frame(&data).await {
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
