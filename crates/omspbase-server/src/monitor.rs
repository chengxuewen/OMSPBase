use crate::signaling::SignalingServer;
use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use omspbase_common::metrics::CoreMetrics;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

// ── Metrics ──────────────────────────────────────────────────────────────────

pub type SharedMetrics = Arc<CoreMetrics>;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct StatsResponse {
    active_rooms: usize,
    connected_peers: usize,
    uptime_seconds: u64,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct MonitorState {
    signaling: SignalingServer,
    metrics: SharedMetrics,
    start_time: Instant,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn monitor_router(signaling: SignalingServer) -> Router {
    let metrics = Arc::new(CoreMetrics::new());
    let start_time = Instant::now();

    Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/stats", get(stats_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(MonitorState {
            signaling,
            metrics,
            start_time,
        })
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health_handler() -> &'static str {
    "OK"
}

async fn ready_handler(State(state): State<MonitorState>) -> (axum::http::StatusCode, &'static str) {
    // Server is ready when there's at least a signaling server (always true after init)
    let _ = &state.signaling;
    (axum::http::StatusCode::OK, "ready")
}

async fn stats_handler(State(state): State<MonitorState>) -> Json<StatsResponse> {
    let active_rooms = state.signaling.room_manager.active_rooms();
    let connected_peers = state.signaling.room_manager.get_peer_count();
    let uptime_seconds = state.start_time.elapsed().as_secs();

    Json(StatsResponse {
        active_rooms,
        connected_peers,
        uptime_seconds,
    })
}

async fn metrics_handler(State(state): State<MonitorState>) -> String {
    // Update gauges from live state
    let connected_peers = state.signaling.room_manager.get_peer_count() as i64;

    state
        .metrics
        .active_connections
        .set(connected_peers);

    // Encode all metrics in Prometheus text format
    state.metrics.encode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::Request;
    use http::StatusCode;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn health_returns_200_ok() {
        let signaling = crate::signaling::SignalingServer::new();
        let app = monitor_router(signaling);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_returns_200() {
        let signaling = crate::signaling::SignalingServer::new();
        let app = monitor_router(signaling);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
