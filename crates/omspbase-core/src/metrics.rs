//! Prometheus metrics helpers.
//!
//! Provides convenience functions to register standard metrics
//! used by all three components.

use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

/// Common metrics registry shared across all components.
pub struct CoreMetrics {
    registry: Registry,
    pub active_connections: Gauge,
    pub relayed_bytes: Counter<u64>,
    pub signaling_latency_us: Gauge,
    pub error_count: Counter<u64>,
}

impl CoreMetrics {
    /// Create a new metrics instance.
    pub fn new() -> Self {
        let mut registry = Registry::default();

        let active_connections = Gauge::default();
        registry.register(
            "active_connections",
            "Number of active WebSocket/WebRTC connections",
            active_connections.clone(),
        );

        let relayed_bytes = Counter::default();
        registry.register(
            "relayed_bytes",
            "Total bytes relayed (Server only)",
            relayed_bytes.clone(),
        );

        let signaling_latency_us = Gauge::default();
        registry.register(
            "signaling_latency_us",
            "Signaling message latency in microseconds",
            signaling_latency_us.clone(),
        );

        let error_count = Counter::default();
        registry.register(
            "error_count",
            "Total error count by error code",
            error_count.clone(),
        );

        Self {
            registry,
            active_connections,
            relayed_bytes,
            signaling_latency_us,
            error_count,
        }
    }

    /// Encode all metrics in Prometheus text format.
    pub fn encode(&self) -> String {
        let mut buf = String::new();
        encode(&mut buf, &self.registry).expect("metrics encoding infallible");
        buf
    }
}

impl Default for CoreMetrics {
    fn default() -> Self {
        Self::new()
    }
}
