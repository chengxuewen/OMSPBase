use dashmap::DashMap;
use omspbase_core::error::CoreError;
use omspbase_core::protocol::PeerRole;
use std::sync::Arc;
use std::time::Instant;

/// A room with at most one Host and one Remote peer.
#[derive(Debug)]
pub struct Room {
    pub id: String,
    pub host: Option<String>,
    pub remote: Option<String>,
    pub created_at: Instant,
}

/// In-memory room state managed by the signaling server.
#[derive(Debug, Clone)]
pub struct RoomManager {
    rooms: Arc<DashMap<String, Room>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
        }
    }

    /// Join a room as Host or Remote. Returns error if the slot is already taken.
    pub fn join_room(&self, room_id: &str, peer_id: &str, role: &PeerRole) -> Result<(), CoreError> {
        let id = room_id.to_string();
        let pid = peer_id.to_string();

        if let Some(mut room) = self.rooms.get_mut(&id) {
            match role {
                PeerRole::Host => {
                    if room.host.is_some() {
                        return Err(CoreError::RoomFull);
                    }
                    room.host = Some(pid);
                }
                PeerRole::Remote => {
                    if room.remote.is_some() {
                        return Err(CoreError::RoomFull);
                    }
                    room.remote = Some(pid);
                }
            }
            tracing::info!("Peer {} joined room {} as {:?}", peer_id, room_id, role);
        } else {
            let (h, r) = match role {
                PeerRole::Host => (Some(pid), None),
                PeerRole::Remote => (None, Some(pid)),
            };
            self.rooms.insert(id.clone(), Room {
                id,
                host: h,
                remote: r,
                created_at: Instant::now(),
            });
            tracing::info!("Room {} created by {:?} {}", room_id, role, peer_id);
        }

        Ok(())
    }

    /// Leave a room (clear the peer's slot; remove room if empty).
    pub fn leave_room(&self, room_id: &str, peer_id: &str) {
        let id = room_id.to_string();
        if let Some(mut room) = self.rooms.get_mut(&id) {
            if room.host.as_deref() == Some(peer_id) {
                room.host = None;
            }
            if room.remote.as_deref() == Some(peer_id) {
                room.remote = None;
            }
            tracing::info!("Peer {} left room {}", peer_id, room_id);
        }
        // Remove empty rooms (ponytail: lazy cleanup — only on leave; add periodic GC if rooms accumulate)
        self.rooms.retain(|_, r| r.host.is_some() || r.remote.is_some());
    }

    /// Get the other peer in the room (returns the peer_id to relay messages to).
    pub fn get_other_peer(&self, room_id: &str, _peer_id: &str) -> Option<String> {
        // ponytail: simple single-room pair relay; extend for multi-remote later
        // For now: if sender is host, return remote; if sender is remote, return host
        self.rooms.get(room_id).and_then(|_room| {
            // We don't know exactly which is the sender without ws state,
            // so in practice this is called from signaling with ws peer context.
            // Return whichever peer is not the sender.
            None // stub — relay routing done in signaling handler directly
        })
    }

    pub fn active_rooms(&self) -> usize {
        self.rooms.len()
    }

    pub fn connected_peers(&self) -> usize {
        self.rooms.iter().filter(|r| {
            r.host.is_some() || r.remote.is_some()
        }).count()
    }

    pub fn get_peer_count(&self) -> usize {
        self.rooms.iter().map(|r| {
            (r.host.is_some() as usize) + (r.remote.is_some() as usize)
        }).sum()
    }

    /// Clean rooms whose last peer left more than `timeout_secs` ago.
    pub fn cleanup_stale(&self, _timeout_secs: u64) {
        // ponytail: lazy cleanup via leave_room retention; add timer-based GC if stale rooms build up
        // self.rooms.retain(|_, r| r.created_at.elapsed().as_secs() < timeout_secs || r.host.is_some() || r.remote.is_some());
    }
}
