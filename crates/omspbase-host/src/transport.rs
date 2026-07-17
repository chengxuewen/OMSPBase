//! WebRTC transport — stub with tracing instrumentation.
//!
//! Real WebRTC connections need a running signaling server (Phase I integration).
//! The Transport struct holds a placeholder for the eventual PeerConnection.

#[cfg(feature = "webrtc")]
mod imp {
    use omspbase_core::error::CoreError;

    pub struct Transport;

    impl Transport {
        pub fn new() -> Self {
            tracing::info!("WebRTC transport initialized (libwebrtc)");
            Transport
        }

        /// Send an encoded H.264 frame to the peer.
        /// Stub: logs the frame size; the real impl pushes bytes via DataChannel.
        pub async fn send_frame(&self, data: &[u8]) -> Result<(), CoreError> {
            tracing::debug!("WebRTC send_frame: {} bytes", data.len());
            Ok(())
        }
    }
}

#[cfg(not(feature = "webrtc"))]
mod imp {
    use omspbase_core::error::CoreError;

    pub struct Transport;

    impl Transport {
        pub fn new() -> Self {
            tracing::warn!("WebRTC stub (not compiled)");
            Transport
        }

        pub async fn send_frame(&self, data: &[u8]) -> Result<(), CoreError> {
            tracing::debug!("WebRTC send_frame (stub): {} bytes", data.len());
            Ok(())
        }
    }
}

pub use imp::Transport;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_new_creates_stub() {
        let _t = Transport::new();
    }

    #[tokio::test]
    async fn transport_send_frame_returns_ok() {
        let t = Transport::new();
        let result = t.send_frame(b"test-frame-data").await;
        assert!(result.is_ok());
    }
}
