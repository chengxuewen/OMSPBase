// WebRTC relay — SFU-style media forwarding from Host to Remotes

#[cfg(feature = "webrtc")]
mod imp {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webrtc::api::APIBuilder;
    use webrtc::peer_connection::configuration::RTCConfiguration;
    use webrtc::peer_connection::RTCPeerConnection;

    /// A relay session: one Host peer connection + multiple Remote peer connections
    pub struct Relay {
        api: webrtc::api::API,
        /// Map of room_id → (Host PC, Vec of Remote PCs)
        sessions: Arc<Mutex<HashMap<String, RelaySession>>>,
    }

    struct RelaySession {
        host_peer: Option<RTCPeerConnection>,
        remote_peers: Vec<RTCPeerConnection>,
    }

    impl Relay {
        pub fn new() -> Self {
            let api = APIBuilder::new().build();
            tracing::info!("WebRTC relay initialized (libwebrtc)");
            Relay {
                api,
                sessions: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        /// Register a Host PeerConnection for a room
        pub async fn register_host(&self, room_id: &str) -> Result<RTCPeerConnection, Box<dyn std::error::Error + Send + Sync>> {
            let config = RTCConfiguration::default();
            let pc = self.api.new_peer_connection(config).await?;
            let mut sessions = self.sessions.lock().await;
            let session = sessions.entry(room_id.to_string()).or_insert_with(|| RelaySession {
                host_peer: None,
                remote_peers: Vec::new(),
            });
            session.host_peer = Some(pc.clone());
            tracing::info!("Host registered in room {}", room_id);
            Ok(pc)
        }

        /// Register a Remote PeerConnection for a room
        pub async fn register_remote(&self, room_id: &str) -> Result<RTCPeerConnection, Box<dyn std::error::Error + Send + Sync>> {
            let config = RTCConfiguration::default();
            let pc = self.api.new_peer_connection(config).await?;
            let mut sessions = self.sessions.lock().await;
            let session = sessions.entry(room_id.to_string()).or_insert_with(|| RelaySession {
                host_peer: None,
                remote_peers: Vec::new(),
            });
            session.remote_peers.push(pc.clone());
            tracing::info!("Remote registered in room {} (total: {})", room_id, session.remote_peers.len());
            Ok(pc)
        }
    }
}

#[cfg(not(feature = "webrtc"))]
mod imp {
    pub struct Relay;

    impl Relay {
        pub fn new() -> Self {
            tracing::warn!("WebRTC relay stub (not compiled) — signaling-only mode");
            Relay
        }
    }
}

pub use imp::Relay;
