//! Backend abstraction layer for multi-backend WebRTC support.
//!
//! Defines traits (PcBackend, DcBackend, TrackWriteBackend)
//! and compile-time type alias dispatch via cfg gates.
//! Zero dyn overhead — all dispatch is monomorphized.

use crate::channel::{DataChannel, DataChannelRx, DataChannelState};
use crate::peer::{AnswerOptions, IceCandidate, OfferOptions, IceConnectionState, IceGatheringState, PeerConnectionState, SignalingState};
use crate::sdp::SessionDescription;
use crate::stats::RtcStats;
use crate::track::{AudioTrackConfig, TrackKind, TrackReceiver};
use crate::RtcError;

// ── Traits ──

pub(crate) trait PcBackend: Send + Sync + 'static {
    async fn create_offer(&self, options: &OfferOptions) -> Result<SessionDescription, RtcError>;
    async fn create_answer(&self, options: &AnswerOptions) -> Result<SessionDescription, RtcError>;
    async fn set_local_description(&self, desc: &SessionDescription) -> Result<(), RtcError>;
    async fn set_remote_description(&self, desc: &SessionDescription) -> Result<(), RtcError>;
    async fn add_ice_candidate(&self, candidate: &IceCandidate) -> Result<(), RtcError>;
    fn connection_state(&self) -> PeerConnectionState;
    fn ice_connection_state(&self) -> IceConnectionState;
    fn ice_gathering_state(&self) -> IceGatheringState;
    fn signaling_state(&self) -> SignalingState;
    async fn close(&self);

    // ── Default methods (no-op for backends that skip these) ──

    /// Register callback for incoming data channels (receiver side).
    fn set_on_data_channel(&self, _cb: Box<dyn Fn(DataChannel) + Send + Sync + 'static>) {}

    /// Register callback for incoming remote tracks (receiver side).
    fn set_on_track(&self, _cb: Box<dyn Fn(TrackReceiver) + Send + Sync + 'static>) {}

    /// Wait until ICE gathering is complete.
    fn gather_complete(&self) -> Result<(), RtcError> {
        Ok(())
    }

    /// Get structured statistics.
    fn get_stats(&self) -> Vec<RtcStats> {
        vec![]
    }

    /// Add an RTP transceiver for a media type with a given direction.
    // ponytail: String params until enums are justified.
    fn add_transceiver(&self, _media_type: &str, _direction: &str) -> Result<(), RtcError> {
        Err(RtcError::Internal("not supported".into()))
    }
}

pub(crate) trait DcBackend: Send + Sync + 'static {
    fn state(&self) -> DataChannelState;
    async fn send(&self, data: &[u8]) -> Result<(), RtcError>;
    async fn send_text(&self, text: &str) -> Result<(), RtcError>;
    async fn spool(&self) -> DataChannelRx;
    async fn close(&mut self);
}

pub(crate) trait TrackWriteBackend: Send + Sync + 'static {
    async fn write_frame(
        &self,
        data: &[u8],
        kind: TrackKind,
        audio_config: Option<&AudioTrackConfig>,
    ) -> Result<(), RtcError>;
}

// ── Mutual exclusion guard ──

#[cfg(all(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys"))]
compile_error!("Only one backend can be enabled at a time. Choose either backend-webrtc-rs or backend-webrtc-sys.");

// ── Module declarations ──

#[cfg(not(any(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys")))]
pub(crate) mod stub;
#[cfg(feature = "backend-webrtc-rs")]
pub(crate) mod webrtc_rs;
#[cfg(feature = "backend-webrtc-sys")]
pub(crate) mod webrtc_sys;

// ── Type alias dispatch (compile-time, monomorphized) ──

#[cfg(feature = "backend-webrtc-rs")]
pub(crate) type ActivePc = webrtc_rs::WebrtcRsPc;
#[cfg(feature = "backend-webrtc-sys")]
pub(crate) type ActivePc = webrtc_sys::WebrtcSysPc;
#[cfg(not(any(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys")))]
pub(crate) type ActivePc = stub::StubPc;

#[cfg(feature = "backend-webrtc-rs")]
pub(crate) type ActiveDc = webrtc_rs::WebrtcRsDc;
#[cfg(feature = "backend-webrtc-sys")]
pub(crate) type ActiveDc = webrtc_sys::WebrtcSysDc;
#[cfg(not(any(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys")))]
pub(crate) type ActiveDc = stub::StubDc;

#[cfg(feature = "backend-webrtc-rs")]
pub(crate) type ActiveTrack = webrtc_rs::WebrtcRsTrack;
#[cfg(feature = "backend-webrtc-sys")]
pub(crate) type ActiveTrack = webrtc_sys::WebrtcSysTrack;
#[cfg(not(any(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys")))]
pub(crate) type ActiveTrack = stub::StubTrack;

#[cfg(feature = "backend-webrtc-rs")]
pub(crate) type ActiveFactory = webrtc_rs::WebrtcRsFactory;
#[cfg(feature = "backend-webrtc-sys")]
pub(crate) type ActiveFactory = webrtc_sys::WebrtcSysFactory;
#[cfg(not(any(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys")))]
pub(crate) type ActiveFactory = stub::StubFactory;
