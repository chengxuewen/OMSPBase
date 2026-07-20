//! Stub backend — all operations are no-ops or return defaults.
//! Used when no WebRTC backend feature is enabled (compilation-only mode).

use super::DcBackend;
use super::PcBackend;
use super::TrackWriteBackend;
use crate::channel::{DataChannel, DataChannelInit, DataChannelRx, DataChannelState};
use crate::peer::{
    AnswerOptions, IceCandidate, OfferOptions, PcConfig,
    IceConnectionState, IceGatheringState, PeerConnectionState, SignalingState,
};
use crate::sdp::{SdpType, SessionDescription};
use crate::track::{AudioTrackConfig, TrackKind};
use crate::RtcError;
use std::sync::atomic::{AtomicBool, Ordering};

// ── StubPc ──

#[derive(Debug, Default)]
pub(crate) struct StubPc {
    closed: AtomicBool,
}

impl PcBackend for StubPc {
    async fn create_offer(&self, _: &OfferOptions) -> Result<SessionDescription, RtcError> {
        Ok(SessionDescription::new(SdpType::Offer, String::new()))
    }

    async fn create_answer(&self, _: &AnswerOptions) -> Result<SessionDescription, RtcError> {
        Ok(SessionDescription::new(SdpType::Answer, String::new()))
    }

    async fn set_local_description(&self, _: &SessionDescription) -> Result<(), RtcError> {
        Ok(())
    }

    async fn set_remote_description(&self, _: &SessionDescription) -> Result<(), RtcError> {
        Ok(())
    }

    async fn add_ice_candidate(&self, _: &IceCandidate) -> Result<(), RtcError> {
        Ok(())
    }

    fn connection_state(&self) -> PeerConnectionState {
        if self.closed.load(Ordering::Relaxed) {
            PeerConnectionState::Closed
        } else {
            PeerConnectionState::New
        }
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        if self.closed.load(Ordering::Relaxed) {
            IceConnectionState::Closed
        } else {
            IceConnectionState::New
        }
    }

    fn ice_gathering_state(&self) -> IceGatheringState {
        IceGatheringState::New
    }

    fn signaling_state(&self) -> SignalingState {
        if self.closed.load(Ordering::Relaxed) {
            SignalingState::Closed
        } else {
            SignalingState::Stable
        }
    }

    async fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

// ponytail: manual Clone for AtomicBool-backed struct
impl Clone for StubPc {
    fn clone(&self) -> Self {
        Self {
            closed: AtomicBool::new(self.closed.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(not(feature = "backend-webrtc-rs"))]
impl StubPc {
    pub(crate) async fn create_data_channel(
        &self,
        label: &str,
        _init: DataChannelInit,
    ) -> Result<DataChannel, RtcError> {
        Ok(DataChannel {
            label: label.to_string(),
            id: 0,
            backend: StubDc,
        })
    }
}

// ── StubDc ──

#[derive(Debug, Default, Clone)]
pub(crate) struct StubDc;

impl DcBackend for StubDc {
    fn state(&self) -> DataChannelState {
        DataChannelState::Closed
    }

    async fn send(&self, _: &[u8]) -> Result<(), RtcError> {
        Ok(())
    }

    async fn send_text(&self, _: &str) -> Result<(), RtcError> {
        Ok(())
    }

    async fn spool(&self) -> DataChannelRx {
        DataChannelRx::stub()
    }

    async fn close(&mut self) {}
}

// ── StubTrack ──

#[derive(Debug, Default, Clone)]
pub(crate) struct StubTrack;

impl TrackWriteBackend for StubTrack {
    async fn write_frame(
        &self,
        data: &[u8],
        _kind: TrackKind,
        _audio_config: Option<&AudioTrackConfig>,
    ) -> Result<(), RtcError> {
        tracing::debug!("TrackSender::write_frame (stub): {} bytes", data.len());
        Ok(())
    }
}

// ── StubFactory ──

#[derive(Debug, Default, Clone)]
pub(crate) struct StubFactory;

impl StubFactory {
    pub(crate) async fn create_peer_connection(
        &self,
        _config: PcConfig,
    ) -> Result<StubPc, RtcError> {
        tracing::info!("Creating PeerConnection (stub)");
        Ok(StubPc::default())
    }
}
