// WebRTC transport via libwebrtc

#[cfg(feature = "webrtc")]
mod imp {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use webrtc::api::APIBuilder;
    use webrtc::peer_connection::configuration::RTCConfiguration;
    use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
    use webrtc::peer_connection::RTCPeerConnection;
    use webrtc::ice_transport::ice_candidate::RTCIceCandidate;

    pub struct Transport {
        api: webrtc::api::API,
        peer: Arc<Mutex<Option<RTCPeerConnection>>>,
    }

    impl Transport {
        pub fn new() -> Self {
            let api = APIBuilder::new().build();
            tracing::info!("WebRTC transport initialized (libwebrtc)");
            Transport {
                api,
                peer: Arc::new(Mutex::new(None)),
            }
        }

        /// Create a new PeerConnection and store it
        pub async fn create_peer_connection(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let config = RTCConfiguration::default();
            let pc = self.api.new_peer_connection(config).await?;
            let mut peer = self.peer.lock().await;
            *peer = Some(pc);
            tracing::info!("PeerConnection created");
            Ok(())
        }

        /// Create an SDP offer
        pub async fn create_offer(&self) -> Result<RTCSessionDescription, Box<dyn std::error::Error + Send + Sync>> {
            let peer = self.peer.lock().await;
            let pc = peer.as_ref().ok_or("no peer connection")?;
            let offer = pc.create_offer(None).await?;
            pc.set_local_description(offer.clone()).await?;
            tracing::info!("SDP offer created");
            Ok(offer)
        }

        /// Set remote SDP description (answer)
        pub async fn set_remote_description(
            &self,
            desc: RTCSessionDescription,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let peer = self.peer.lock().await;
            let pc = peer.as_ref().ok_or("no peer connection")?;
            pc.set_remote_description(desc).await?;
            tracing::info!("Remote description set");
            Ok(())
        }

        /// Add an ICE candidate from signaling
        pub async fn add_ice_candidate(
            &self,
            candidate: RTCIceCandidate,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let peer = self.peer.lock().await;
            let pc = peer.as_ref().ok_or("no peer connection")?;
            pc.add_ice_candidate(candidate).await?;
            tracing::debug!("ICE candidate added");
            Ok(())
        }
    }
}

#[cfg(not(feature = "webrtc"))]
mod imp {
    pub struct Transport;

    impl Transport {
        pub fn new() -> Self {
            tracing::warn!("WebRTC stub (not compiled)");
            Transport
        }
    }
}

pub use imp::Transport;
