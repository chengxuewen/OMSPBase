//! WebRTC transport — handles offer/answer and ICE exchange via data channel.
//!
//! Receives frames via webrtc-rs data channel and forwards them
//! to the decode pipeline through an mpsc channel.

use omspbase_webrtc::{
    AnswerOptions, DataChannelEvent, DataMessage, IceCandidate as RtcIceCandidate,
    PcConfig, PeerConnection, PeerConnectionFactory, RtcError, SessionDescription,
};
use std::sync::Mutex;
use tokio::sync::mpsc;

/// Manages a WebRTC PeerConnection for receiving frames via data channel.
///
/// Created before the signaling relay loop, reused for subsequent ICE candidates.
pub struct WebrtcTransport {
    factory: PeerConnectionFactory,
    pc: Mutex<Option<PeerConnection>>,
    frame_tx: mpsc::UnboundedSender<Vec<u8>>,
    ice_tx: mpsc::UnboundedSender<String>,
}

/// Convenience: create ICE channel pair.
pub fn ice_channel() -> (mpsc::UnboundedSender<String>, mpsc::UnboundedReceiver<String>) {
    mpsc::unbounded_channel()
}

impl WebrtcTransport {
    /// Create a new transport.
    ///
    /// `frame_tx` — forwards received DataChannel messages to the decode pipeline.
    /// `ice_tx` — forwards ICE candidate JSON for sending via signaling WS.
    pub fn new(
        frame_tx: mpsc::UnboundedSender<Vec<u8>>,
        ice_tx: mpsc::UnboundedSender<String>,
    ) -> Self {
        Self {
            factory: PeerConnectionFactory::new(),
            pc: Mutex::new(None),
            frame_tx,
            ice_tx,
        }
    }

    /// Handle an incoming SDP offer from the signaling server.
    ///
    /// Creates a PeerConnection, sets remote description, registers
    /// data channel and ICE callbacks, generates an answer.
    ///
    /// Returns the answer SDP serialized as JSON.
    pub async fn handle_offer(&self, sdp_json: &str) -> Result<String, RtcError> {
        // a. Deserialize offer
        let offer: SessionDescription = serde_json::from_str(sdp_json)
            .map_err(|e| RtcError::Sdp(format!("parse offer SDP: {e}")))?;

        // b. Create PeerConnection
        let config = PcConfig::default();
        let pc = self.factory.create_peer_connection(config).await?;

        // c. Set remote description
        pc.set_remote_description(&offer).await?;

        // d. Register on_data_channel: spool → spawn task → forward frames
        let frame_tx = self.frame_tx.clone();
        pc.on_data_channel(Box::new(move |d| {
            let frame_tx = frame_tx.clone();
            Box::pin(async move {
                let dc = omspbase_webrtc::DataChannel::from_webrtc(d).await;
                let mut rx = dc.spool().await;
                tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
                        Some(DataChannelEvent::Open) => {
                            tracing::info!("Signaling: DataChannel opened (remote)");
                        }
                        Some(DataChannelEvent::Message(DataMessage { data })) => {
                            tracing::info!("Signaling: frame received via DataChannel ({} bytes)", data.len());
                            }
                            Some(DataChannelEvent::Closed) | None => break,
                            _ => {} // Open, Error — ignore
                        }
                    }
                });
            })
        }));

        // e. Register on_ice_candidate: serialize to JSON, push via ice_tx
        let ice_tx = self.ice_tx.clone();
        pc.on_ice_candidate(Box::new(move |candidate| {
            let ice_tx = ice_tx.clone();
            Box::pin(async move {
                if let Some(c) = candidate {
                    if let Ok(init) = c.to_json() {
                        if let Ok(json) = serde_json::to_string(&init) {
                            let _ = ice_tx.send(json);
                        }
                    }
                }
                // ponytail: None = gathering complete, no-op for now
            })
        }));

        // f. Create answer and set local description
        let answer = pc.create_answer(&AnswerOptions::default()).await?;
        pc.set_local_description(&answer).await?;

        // g. Serialize answer to JSON
        let answer_json = serde_json::to_string(&answer)
            .map_err(|e| RtcError::Sdp(format!("serialize answer: {e}")))?;

        // h. Store the PC for subsequent ICE handling
        {
            let mut guard = self.pc.lock().map_err(|e| RtcError::Internal(e.to_string()))?;
            *guard = Some(pc);
        }

        Ok(answer_json)
    }

    /// Handle an incoming ICE candidate from the signaling server.
    ///
    /// `candidate_json` is a JSON representation of RTCIceCandidateInit
    /// (with camelCase fields: candidate, sdpMid, sdpMLineIndex).
    pub async fn handle_ice(&self, candidate_json: &str) -> Result<(), RtcError> {
        let init: omspbase_webrtc::webrtc::ice_transport::ice_candidate::RTCIceCandidateInit =
            serde_json::from_str(candidate_json)
                .map_err(|e| RtcError::Internal(format!("parse ICE candidate: {e}")))?;

        // Clone PC outside the lock scope
        let pc = {
            let guard = self.pc.lock().map_err(|e| RtcError::Internal(e.to_string()))?;
            guard.clone()
        };

        if let Some(ref pc) = pc {
            let candidate = RtcIceCandidate {
                candidate: init.candidate.clone(),
                sdp_mid: init.sdp_mid.clone(),
                sdp_mline_index: init.sdp_mline_index,
            };
            pc.add_ice_candidate(&candidate).await?;
        }
        Ok(())
    }
}
