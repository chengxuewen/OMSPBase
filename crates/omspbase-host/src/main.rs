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
            tracing::info!("Config loaded: {:?}", c);
            c
        }
        Err(e) => {
            tracing::error!("Failed to load config from {}: {}", config_path, e);
            process::exit(1);
        }
    };

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
        tracing::error!("Pipeline init failed: {}", e);
        process::exit(1);
    }));

    if let Err(e) = pipeline.start() {
        tracing::error!("Pipeline start failed: {}", e);
        process::exit(1);
    }

    // Phase 3: Create WebRTC transport
    let transport = transport::Transport::new();

    // Phase 4: Create control handler (shared with metrics)
    let control_handler = control::ControlHandler::new();
    let frames_dropped = control_handler.frames_dropped.clone();

    // Phase 5: Build combined axum router (metrics + signaling)
    let core_metrics = omspbase_core::metrics::CoreMetrics::new();
    let shared_metrics = std::sync::Arc::new(std::sync::RwLock::new(core_metrics));
    let metrics_router = metrics::metrics_router(shared_metrics.clone());
    let signaling_router = signaling::signaling_router();

    let app = axum::Router::new()
        .merge(metrics_router)
        .merge(signaling_router);

    // Determine bind address
    let bind_addr = config
        .server
        .signaling_url
        .strip_prefix("ws://")
        .or_else(|| config.server.signaling_url.strip_prefix("wss://"))
        .unwrap_or("0.0.0.0:9800");

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
    tracing::info!("Host ready — session id={}", config.capture.source);

    // Phase 6: Emergency UDP listener (background)
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
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
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

    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Stopping]);

    // Stop pipeline
    if let Err(e) = pipeline.stop() {
        tracing::error!("Pipeline stop error: {}", e);
    }

    // Clean up background tasks
    push_handle.abort();
    metrics_updater.abort();
    emergency_handle.abort();
    persist_handle.abort();

    tracing::info!("Shutdown complete");
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
