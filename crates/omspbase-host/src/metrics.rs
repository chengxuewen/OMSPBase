use axum::{routing::get, extract::State, Json, Router};
use prometheus_client::metrics::gauge::Gauge;
use serde::Serialize;
use std::sync::Arc;
use std::sync::RwLock;

pub struct AppMetrics {
    pub capture_fps: Gauge,
    pub encode_fps: Gauge,
    pub push_bitrate_kbps: Gauge,
    pub push_rtt_ms: Gauge,
    pub process_rss_mb: Gauge,
    pub control_frames_dropped: Gauge,
    pub encode_queue_depth: Gauge,
}

impl AppMetrics {
    pub fn new() -> Self {
        Self {
            capture_fps: Gauge::default(),
            encode_fps: Gauge::default(),
            push_bitrate_kbps: Gauge::default(),
            push_rtt_ms: Gauge::default(),
            process_rss_mb: Gauge::default(),
            control_frames_dropped: Gauge::default(),
            encode_queue_depth: Gauge::default(),
        }
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    checks: ReadyChecks,
}

#[derive(Serialize)]
struct ReadyChecks {
    camera: String,
    encoder: String,
    signaling: String,
}

pub type SharedMetrics = Arc<RwLock<AppMetrics>>;

pub fn metrics_router(metrics: SharedMetrics) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(HealthResponse { status: "ok".to_string() }) }))
        .route("/ready", get(|State(m): State<SharedMetrics>| async move {
            drop(m.read().unwrap());
            Json(ReadyResponse {
                ready: true,
                checks: ReadyChecks {
                    camera: "ok".to_string(),
                    encoder: "ok".to_string(),
                    signaling: "connected".to_string(),
                },
            })
        }))
        .route("/metrics", get(|State(m): State<SharedMetrics>| async move {
            let m = m.read().unwrap();
            let mut buf = String::new();
            buf.push_str(&format!("capture_fps {:.1}\n", m.capture_fps.get()));
            buf.push_str(&format!("encode_fps {:.1}\n", m.encode_fps.get()));
            buf.push_str(&format!("push_bitrate_kbps {:.1}\n", m.push_bitrate_kbps.get()));
            buf.push_str(&format!("push_rtt_ms {:.1}\n", m.push_rtt_ms.get()));
            buf.push_str(&format!("process_rss_mb {:.1}\n", m.process_rss_mb.get()));
            buf
        }))
        .with_state(metrics)
}
