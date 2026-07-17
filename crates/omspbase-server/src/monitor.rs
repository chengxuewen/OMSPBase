// REST API monitoring endpoints

use crate::signaling::SignalingServer;
use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use prometheus_client::metrics::gauge::Gauge;
use serde::Serialize;
use std::sync::Arc;
use std::sync::RwLock;

// ── Metrics ─────────────────────────────────────────────────────────────────

pub struct AppMetrics {
    pub active_rooms: Gauge,
    pub connected_hosts: Gauge,
    pub connected_remotes: Gauge,
}

impl AppMetrics {
    pub fn new() -> Self {
        Self {
            active_rooms: Gauge::default(),
            connected_hosts: Gauge::default(),
            connected_remotes: Gauge::default(),
        }
    }
}

pub type SharedMetrics = Arc<RwLock<AppMetrics>>;

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[derive(Serialize)]
struct HostListResponse {
    hosts: Vec<String>,
    count: usize,
}

#[derive(Serialize)]
struct RemoteListResponse {
    remotes: Vec<String>,
    count: usize,
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn monitor_router(signaling: SignalingServer) -> Router {
    let metrics = Arc::new(RwLock::new(AppMetrics::new()));

    Router::new()
        .route("/health", get(health_handler))
        .route("/api/hosts", get(hosts_handler))
        .route("/api/remotes", get(remotes_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(MonitorState {
            signaling,
            metrics,
        })
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct MonitorState {
    signaling: SignalingServer,
    metrics: SharedMetrics,
}

// ── Handlers ────────────────────────────────────────────────────────────────

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn hosts_handler(State(state): State<MonitorState>) -> Json<HostListResponse> {
    let hosts = state.signaling.active_hosts();
    let count = hosts.len();
    Json(HostListResponse { hosts, count })
}

async fn remotes_handler(State(state): State<MonitorState>) -> Json<RemoteListResponse> {
    let remotes = state.signaling.active_remotes();
    let count = remotes.len();
    Json(RemoteListResponse { remotes, count })
}

async fn metrics_handler(State(state): State<MonitorState>) -> String {
    // Snapshot live signaling state and update gauges on each scrape
    let (room_count, host_count, remote_count) = state.signaling.metrics_snapshot();
    let m = state.metrics.read().unwrap();

    m.active_rooms.set(room_count as i64);
    m.connected_hosts.set(host_count as i64);
    m.connected_remotes.set(remote_count as i64);

    let mut buf = String::new();
    buf.push_str(&format!("active_rooms {}\n", m.active_rooms.get()));
    buf.push_str(&format!("connected_hosts {}\n", m.connected_hosts.get()));
    buf.push_str(&format!("connected_remotes {}\n", m.connected_remotes.get()));
    buf
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_status() {
        let resp = HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
        };
        assert_eq!(resp.status, "ok");
        assert_eq!(resp.version, "0.1.0");
    }

    #[test]
    fn host_list_empty() {
        let resp = HostListResponse {
            hosts: vec![],
            count: 0,
        };
        assert_eq!(resp.count, 0);
    }

    #[test]
    fn remote_list_with_entries() {
        let resp = RemoteListResponse {
            remotes: vec!["room=a remotes=2".to_string()],
            count: 1,
        };
        assert_eq!(resp.count, 1);
        assert_eq!(resp.remotes.len(), 1);
    }
}