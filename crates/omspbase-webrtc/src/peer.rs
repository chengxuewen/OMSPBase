//! PeerConnection thin wrapper.
//!
//! Delegates WebRTC operations to the backend (PcBackend trait)
//! via compile-time type alias dispatch. Track registry and W3C
//! API methods are backend-agnostic and handled directly.

use std::collections::HashMap;
use std::sync::Arc;

use crate::backend::{ActivePc, PcBackend};
use crate::channel::{DataChannel, DataChannelInit};
use crate::sdp::SessionDescription;
use crate::track::{TrackKind, TrackSender, TrackRef};
use crate::RtcError;
use crate::rtp::{RtpReceiver, RtpSender};

// ── Connection state enums ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerConnectionState { New, Connecting, Connected, Disconnected, Failed, Closed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceConnectionState { New, Checking, Connected, Completed, Failed, Disconnected, Closed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceGatheringState { New, Gathering, Complete }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalingState { Stable, HaveLocalOffer, HaveLocalPrAnswer, HaveRemoteOffer, HaveRemotePrAnswer, Closed }

// ── Configuration types ──

#[derive(Debug, Clone)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct PcConfig {
    pub ice_servers: Vec<IceServer>,
    pub ice_transport_type: IceTransportsType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceTransportsType { Relay, NoHost, All }

impl Default for PcConfig {
    fn default() -> Self {
        Self { ice_servers: vec![], ice_transport_type: IceTransportsType::All }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OfferOptions {
    pub ice_restart: bool,
    pub offer_to_receive_audio: bool,
    pub offer_to_receive_video: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AnswerOptions;

#[derive(Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
}

// ── PeerConnectionFactory ──

use crate::backend::ActiveFactory;

pub struct PeerConnectionFactory {
    backend: ActiveFactory,
}

impl PeerConnectionFactory {
    pub fn new() -> Self {
        Self { backend: ActiveFactory::default() }
    }

    pub async fn create_peer_connection(&self, config: PcConfig) -> Result<PeerConnection, RtcError> {
        let pc_backend = self.backend.create_peer_connection(config).await?;
        Ok(PeerConnection {
            backend: pc_backend,
            tracks: Arc::new(std::sync::Mutex::new(HashMap::new())),
            on_track_callback: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// Create a video track with a real VideoTrackSource (webrtc-sys only).
    /// For other backends, returns a stub TrackSender.
    pub fn create_video_track(&self, track_id: &str) -> TrackSender {
        #[cfg(feature = "backend-webrtc-sys")]
        {
            let (backend, _media_track) = self.backend.create_video_track();
            TrackSender { id: track_id.to_string(), kind: TrackKind::Video, audio_config: None, backend }
        }
        #[cfg(not(feature = "backend-webrtc-sys"))]
        {
            TrackSender::new(track_id.to_string(), TrackKind::Video)
        }
    }
}

impl Default for PeerConnectionFactory {
    fn default() -> Self { Self::new() }
}

// ── PeerConnection ──

/// Callback type for onTrack (D146).
type OnTrackCallback = Arc<std::sync::Mutex<Option<Box<dyn Fn(RtpReceiver) + Send + Sync>>>>;

pub struct PeerConnection {
    pub(crate) backend: ActivePc,
    tracks: Arc<std::sync::Mutex<HashMap<String, TrackRef>>>,
    on_track_callback: OnTrackCallback,
}

/// Maximum tracks per PeerConnection (D148).
pub const MAX_TRACKS: usize = 8;

// ── Common methods (both backends) ──

impl PeerConnection {
    pub async fn create_offer(&self, options: &OfferOptions) -> Result<SessionDescription, RtcError> {
        self.backend.create_offer(options).await
    }

    pub async fn create_answer(&self, options: &AnswerOptions) -> Result<SessionDescription, RtcError> {
        self.backend.create_answer(options).await
    }

    pub async fn set_local_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        self.backend.set_local_description(desc).await
    }

    pub async fn set_remote_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        self.backend.set_remote_description(desc).await
    }

    pub async fn add_ice_candidate(&self, candidate: &IceCandidate) -> Result<(), RtcError> {
        self.backend.add_ice_candidate(candidate).await
    }

    pub async fn create_data_channel(&self, label: &str, init: DataChannelInit) -> Result<DataChannel, RtcError> {
        self.backend.create_data_channel(label, init).await
    }

    pub fn connection_state(&self) -> PeerConnectionState {
        self.backend.connection_state()
    }

    pub fn ice_connection_state(&self) -> IceConnectionState {
        self.backend.ice_connection_state()
    }

    pub fn ice_gathering_state(&self) -> IceGatheringState {
        self.backend.ice_gathering_state()
    }

    pub fn signaling_state(&self) -> SignalingState {
        self.backend.signaling_state()
    }

    pub async fn close(&self) {
        self.backend.close().await;
    }

    // ── Track registry (backend-agnostic) ──

    pub fn add_track(&self, track_id: &str, kind: TrackKind) -> Result<String, RtcError> {
        let mut tracks = self.tracks.lock().unwrap();
        if tracks.len() >= MAX_TRACKS {
            return Err(RtcError::Track("max tracks reached".into()));
        }
        let sender = TrackSender::new(track_id.to_string(), kind);
        let id = track_id.to_string();
        tracks.insert(id.clone(), TrackRef::Sender(sender));
        Ok(id)
    }

    pub fn remove_track(&self, track_id: &str) -> Result<(), RtcError> {
        let mut tracks = self.tracks.lock().unwrap();
        tracks.remove(track_id).map(|_| ()).ok_or_else(|| {
            RtcError::Track(format!("track not found: {}", track_id))
        })
    }

    pub fn get_track(&self, track_id: &str) -> Option<TrackRef> {
        let tracks = self.tracks.lock().unwrap();
        tracks.get(track_id).cloned()
    }

    pub fn track_count(&self) -> usize {
        self.tracks.lock().unwrap().len()
    }

    pub fn track_ids(&self) -> Vec<String> {
        self.tracks.lock().unwrap().keys().cloned().collect()
    }
}

// ── webrtc-rs specific callback methods ──

#[cfg(feature = "backend-webrtc-rs")]
impl PeerConnection {
    pub fn on_data_channel(
        &self,
        f: Box<
            dyn FnMut(
                    std::sync::Arc<webrtc::data_channel::RTCDataChannel>,
                ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>
                + Send
                + Sync
                + 'static,
        >,
    ) {
        self.backend.on_data_channel(f);
    }

    pub fn on_ice_candidate(
        &self,
        f: Box<
            dyn FnMut(
                    Option<webrtc::ice_transport::ice_candidate::RTCIceCandidate>,
                ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>
                + Send
                + Sync
                + 'static,
        >,
    ) {
        self.backend.on_ice_candidate(f);
    }
}

// ── W3C API methods (D146) ──

#[allow(non_snake_case)]
impl PeerConnection {
    /// W3C addTrack — adds a sender track and returns the RTCRtpSender.
    pub fn addTrack(&self, track_id: &str, kind: TrackKind) -> Result<RtpSender, RtcError> {
        self.add_track(track_id, kind)?;
        let tr = self.get_track(track_id).ok_or_else(|| RtcError::Track("track disappeared".into()))?;
        Ok(RtpSender::new(tr))
    }

    /// W3C getSenders — returns all sender tracks as RtpSender.
    pub fn getSenders(&self) -> Vec<RtpSender> {
        self.tracks.lock().unwrap().values()
            .filter(|tr| matches!(tr, TrackRef::Sender(_)))
            .map(|tr| RtpSender::new(tr.clone()))
            .collect()
    }

    /// W3C getReceivers — returns all receiver tracks as RtpReceiver.
    pub fn getReceivers(&self) -> Vec<RtpReceiver> {
        self.tracks.lock().unwrap().values()
            .filter(|tr| matches!(tr, TrackRef::Receiver(_)))
            .map(|tr| RtpReceiver::new(tr.clone()))
            .collect()
    }

    /// W3C onTrack — register callback for incoming remote tracks.
    pub fn onTrack<F>(&self, callback: F)
    where F: Fn(RtpReceiver) + Send + Sync + 'static {
        *self.on_track_callback.lock().unwrap() = Some(Box::new(callback));
    }
}

impl Clone for PeerConnection {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            tracks: self.tracks.clone(),
            on_track_callback: Arc::clone(&self.on_track_callback),
        }
    }
}

impl std::fmt::Debug for PeerConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerConnection")
            .field("connection_state", &self.connection_state())
            .field("track_count", &self.track_count())
            .finish()
    }
}

// ── Tests ──

#[cfg(all(test, not(any(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys"))))]
mod tests {
    use super::*;
    use crate::sdp::SdpType;
    use crate::channel::DataChannelInit;

    #[test]
    fn stub_factory_default_creates() {
        let factory = PeerConnectionFactory::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(factory.create_peer_connection(PcConfig::default()));
        assert!(pc.is_ok());
    }

    #[test]
    fn stub_create_offer_returns_sdp_type_offer() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let offer = rt.block_on(pc.create_offer(&OfferOptions::default())).unwrap();
        assert_eq!(offer.sdp_type, SdpType::Offer);
    }

    #[test]
    fn stub_create_answer_returns_sdp_type_answer() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let answer = rt.block_on(pc.create_answer(&AnswerOptions::default())).unwrap();
        assert_eq!(answer.sdp_type, SdpType::Answer);
    }

    #[test]
    fn stub_sdp_operations_are_noop() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let sd = SessionDescription::new(SdpType::Offer, String::new());
        assert!(rt.block_on(pc.set_local_description(&sd)).is_ok());
        assert!(rt.block_on(pc.set_remote_description(&sd)).is_ok());
    }

    #[test]
    fn stub_add_ice_candidate_is_noop() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let ic = IceCandidate { candidate: "candidate:1".into(), sdp_mid: Some("0".into()), sdp_mline_index: Some(0) };
        assert!(rt.block_on(pc.add_ice_candidate(&ic)).is_ok());
    }

    #[test]
    fn stub_create_data_channel_preserves_label() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let dc = rt.block_on(pc.create_data_channel("mychan", DataChannelInit::default())).unwrap();
        assert_eq!(dc.label(), "mychan");
        assert_eq!(dc.id(), 0);
    }

    #[test]
    fn stub_connection_states_are_default() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        assert!(matches!(pc.connection_state(), PeerConnectionState::New));
        assert!(matches!(pc.ice_connection_state(), IceConnectionState::New));
        assert!(matches!(pc.ice_gathering_state(), IceGatheringState::New));
        assert!(matches!(pc.signaling_state(), SignalingState::Stable));
    }

    #[test]
    fn stub_close_is_noop() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        rt.block_on(pc.close());
    }

    #[test]
    fn pc_config_default_is_all_transport() {
        let cfg = PcConfig::default();
        assert!(cfg.ice_servers.is_empty());
        assert_eq!(cfg.ice_transport_type, IceTransportsType::All);
    }

    // ── D148: multi-track registry tests ──

    #[test]
    fn add_track_registers_in_map() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let id = pc.add_track("video-1", TrackKind::Video).unwrap();
        assert_eq!(id, "video-1");
        assert_eq!(pc.track_count(), 1);
        let ids = pc.track_ids();
        assert!(ids.contains(&"video-1".to_string()));
    }

    #[test]
    fn add_track_respects_max() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        for i in 0..8 {
            pc.add_track(&format!("track-{}", i), TrackKind::Video).unwrap();
        }
        assert_eq!(pc.track_count(), 8);
        let err = pc.add_track("overflow", TrackKind::Video).unwrap_err();
        assert!(matches!(err, RtcError::Track(_)));
    }

    #[test]
    fn remove_track_not_found() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let err = pc.remove_track("nonexistent").unwrap_err();
        assert!(matches!(err, RtcError::Track(_)));
    }

    #[test]
    fn remove_track_succeeds() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        pc.add_track("audio-1", TrackKind::Audio).unwrap();
        assert_eq!(pc.track_count(), 1);
        pc.remove_track("audio-1").unwrap();
        assert_eq!(pc.track_count(), 0);
    }

    #[test]
    fn get_track_returns_clone() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        pc.add_track("vid-1", TrackKind::Video).unwrap();
        let tr = pc.get_track("vid-1").unwrap();
        assert_eq!(tr.id(), "vid-1");
        assert_eq!(tr.kind(), TrackKind::Video);
    }

    #[test]
    fn get_track_missing_returns_none() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        assert!(pc.get_track("missing").is_none());
    }

    // ── D146: W3C API tests ──

    #[test]
    fn add_track_w3c_returns_sender() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        let sender = pc.addTrack("video-w3c", TrackKind::Video).unwrap();
        assert_eq!(sender.track_id, "video-w3c");
        assert_eq!(sender.kind, TrackKind::Video);
    }

    #[test]
    fn get_senders_filters_correctly() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        pc.addTrack("vid-1", TrackKind::Video).unwrap();
        pc.addTrack("vid-2", TrackKind::Video).unwrap();
        let senders = pc.getSenders();
        assert_eq!(senders.len(), 2);
    }

    #[test]
    fn get_receivers_empty_initially() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        assert!(pc.getReceivers().is_empty());
    }

    #[test]
    fn on_track_registers_callback() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let pc = rt.block_on(PeerConnectionFactory::new().create_peer_connection(PcConfig::default())).unwrap();
        pc.onTrack(|_receiver| {});
    }
}
