use omspbase_core::error::CoreError;

#[cfg(feature = "webrtc")]
mod imp {
    use super::CoreError;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webrtc::api::APIBuilder;
    use webrtc::peer_connection::configuration::RTCConfiguration;
    use webrtc::peer_connection::RTCPeerConnection;

    pub struct Relay {
        api: webrtc::api::API,
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

        pub async fn register_host(
            &self,
            room_id: &str,
        ) -> Result<RTCPeerConnection, CoreError> {
            let config = RTCConfiguration::default();
            let pc = self
                .api
                .new_peer_connection(config)
                .await
                .map_err(|e| CoreError::PeerConnectionFailure(e.to_string()))?;
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .entry(room_id.to_string())
                .or_insert_with(|| RelaySession {
                    host_peer: None,
                    remote_peers: Vec::new(),
                });
            session.host_peer = Some(pc.clone());
            tracing::info!("Host registered in room {}", room_id);
            Ok(pc)
        }

        pub async fn register_remote(
            &self,
            room_id: &str,
        ) -> Result<RTCPeerConnection, CoreError> {
            let config = RTCConfiguration::default();
            let pc = self
                .api
                .new_peer_connection(config)
                .await
                .map_err(|e| CoreError::PeerConnectionFailure(e.to_string()))?;
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .entry(room_id.to_string())
                .or_insert_with(|| RelaySession {
                    host_peer: None,
                    remote_peers: Vec::new(),
                });
            session.remote_peers.push(pc.clone());
            tracing::info!(
                "Remote registered in room {} (total: {})",
                room_id,
                session.remote_peers.len()
            );
            Ok(pc)
        }

        /// Bridge tracks: bind Host video track to Remote PC.
        /// ponytail: stub for MVP; full track bridging needs live PeerConnections.
        pub async fn bridge_tracks(
            &self,
            _host_pc: &RTCPeerConnection,
            _remote_pc: &RTCPeerConnection,
        ) -> Result<(), CoreError> {
            tracing::info!("WebRTC track bridging (stub) — full implementation pending");
            Ok(())
        }
    }
}

#[cfg(not(feature = "webrtc"))]
mod imp {
    use super::CoreError;

    pub struct Relay;

    impl Relay {
        pub fn new() -> Self {
            tracing::info!("WebRTC relay initialized (stub — signaling-only mode)");
            Relay
        }

        /// ponytail: stub bridge_tracks; requires webrtc feature for real PeerConnection access.
        pub async fn bridge_tracks(
            &self,
            _host_pc: &(),
            _remote_pc: &(),
        ) -> Result<(), CoreError> {
            tracing::debug!("bridge_tracks stub — enable webrtc feature for real relay");
            Ok(())
        }
    }
}

pub use imp::Relay;
