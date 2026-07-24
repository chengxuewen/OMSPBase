//! mediasoup SFU foundation.
//!
//! Provides `SfuManager` (global), `SfuRoom` (per-room Router + peers),
//! and `SfuPeer` (per-peer transports + producers/consumers).
//!
//! Only compiled when the `sfu-mediasoup` feature is enabled.

#[cfg(feature = "sfu-mediasoup")]
mod imp {
    use dashmap::DashMap;
    use mediasoup::prelude::*;
    use mediasoup::worker_manager::WorkerManager;
    use std::sync::Arc;

    /// Per-peer state: send/recv transports and active producers/consumers.
    pub struct SfuPeer {
        pub send_transport: WebRtcTransport,
        pub recv_transport: WebRtcTransport,
        pub producers: Vec<Producer>,
        pub consumers: Vec<Consumer>,
    }

    /// Per-room SFU state: one Router, all connected peers.
    pub struct SfuRoom {
        pub router: Router,
        pub peers: DashMap<String, Arc<SfuPeer>>,
    }

    /// Global SFU manager — owns WorkerManager, maps room_id → SfuRoom.
    #[allow(dead_code)]
    pub struct SfuManager {
        worker_manager: WorkerManager,
        worker: Worker,
        rooms: DashMap<String, Arc<SfuRoom>>,
    }

    impl SfuManager {
        /// Create a new SfuManager with a single mediasoup Worker.
        ///
        /// The Worker is created immediately — panics if the mediasoup C++
        /// binary cannot be found or spawned.
        pub async fn new() -> Result<Self, String> {
            let worker_manager = WorkerManager::new();
            let worker = worker_manager
                .create_worker(WorkerSettings::default())
                .await
                .map_err(|e| format!("Failed to create mediasoup worker: {e}"))?;
            tracing::info!("mediasoup Worker created (id: {:?})", worker.id());
            Ok(Self {
                worker_manager,
                worker,
                rooms: DashMap::new(),
            })
        }

        /// Get or create a room with its own Router.
        pub async fn get_or_create_room(
            &self,
            room_id: &str,
        ) -> Result<Arc<SfuRoom>, String> {
            if let Some(room) = self.rooms.get(room_id) {
                return Ok(Arc::clone(room.value()));
            }
            let router = self
                .worker
                .create_router(RouterOptions::default())
                .await
                .map_err(|e| format!("Failed to create router for room {room_id}: {e}"))?;
            tracing::info!("Router created for room {} (id: {:?})", room_id, router.id());
            let room = Arc::new(SfuRoom {
                router,
                peers: DashMap::new(),
            });
            self.rooms.insert(room_id.to_string(), Arc::clone(&room));
            Ok(room)
        }

        /// Remove a room and its Router (stops forwarding for all peers).
        pub fn remove_room(&self, room_id: &str) -> bool {
            self.rooms.remove(room_id).is_some()
        }

        /// Number of active rooms.
        pub fn room_count(&self) -> usize {
            self.rooms.len()
        }
    }
}

// ── Stub when sfu-mediasoup is not enabled ──────────────────────────────

#[cfg(not(feature = "sfu-mediasoup"))]
mod imp {
    /// Stub SfuManager — SFU not available.
    pub struct SfuManager;

    impl SfuManager {
        /// Returns an error in non-SFU builds.
        pub async fn new() -> Result<Self, String> {
            Err("sfu-mediasoup feature not enabled".into())
        }
    }

    /// Stub SfuRoom — SFU not available.
    pub struct SfuRoom;

    /// Stub SfuPeer — SFU not available.
    pub struct SfuPeer;
}

pub use imp::{SfuManager, SfuPeer, SfuRoom};
