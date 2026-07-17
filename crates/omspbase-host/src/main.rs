use std::process;

mod config;
mod control;
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
                .add_directive("info".parse().unwrap()),
        )
        .init();

    tracing::info!("OMSPBase Host v{} starting", env!("CARGO_PKG_VERSION"));

    // Parse config — collect args once for bounds-safe access
    let config_path = {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 3 && args[2] == "--config" {
            args[3].clone()
        } else {
            "/opt/oomspbase/etc/host.conf".to_string()
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

    // Phase 1: Load or create session state
    let session_state = session::Session::load(None);
    let persist_handle = session_state.start_persist();

    // Phase 2: Start GStreamer pipeline
    let pipeline = pipeline::Pipeline::new(
        &config.media.camera,
        config.media.width,
        config.media.height,
        config.media.fps,
        config.media.bitrate_kbps,
        &config.media.encoder,
    )
    .unwrap_or_else(|e| {
        tracing::error!("Pipeline init failed: {}", e);
        process::exit(1);
    });

    if let Err(e) = pipeline.start() {
        tracing::error!("Pipeline start failed: {}", e);
        process::exit(1);
    }

    // Phase 3: Create WebRTC transport
    let _transport = transport::Transport::new();

    // Phase 4: Create control handler (shared with metrics)
    let control_handler = control::ControlHandler::new();
    let frames_dropped = control_handler.frames_dropped.clone();

    // Phase 5: Build combined axum router (metrics + signaling)
    let metrics = metrics::AppMetrics::new();
    let shared_metrics = std::sync::Arc::new(std::sync::RwLock::new(metrics));
    let metrics_router = metrics::metrics_router(shared_metrics);
    let signaling_router = signaling::signaling_router();

    let app = axum::Router::new()
        .merge(metrics_router)
        .merge(signaling_router);

    // Determine bind address
    let bind_addr = config
        .web
        .as_ref()
        .map(|w| w.bind.as_str())
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
    tracing::info!("Host {} ready", config.host.id);

    // Run server with graceful shutdown
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");
    });

    // Start metrics updater: periodically sync control_frames_dropped
    let metrics_updater = {
        let dropped = frames_dropped;
        let shared = std::sync::Arc::new(std::sync::RwLock::new(()));
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                // ponytail: would sync to shared_metrics here, but metrics router
                // currently reads its own gauge set — for MVP, counter exists
                let _ = dropped.load(std::sync::atomic::Ordering::Relaxed);
                let _ = shared;
            }
        })
    };

    // Run server (blocks until shutdown signal)
    if let Err(e) = server.await {
        tracing::error!("Server error: {}", e);
    }

    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Stopping]);

    // Stop pipeline
    if let Err(e) = pipeline.stop() {
        tracing::error!("Pipeline stop error: {}", e);
    }

    // Clean up background tasks
    metrics_updater.abort();
    persist_handle.abort();

    tracing::info!("Shutdown complete");
}
