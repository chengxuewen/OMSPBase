//! PeerConnection thin wrapper.
//!
//! With `webrtc-backend` feature: wraps webrtc-rs RTCPeerConnection.
//! Without: stub (returns success, no actual networking).

#[cfg(feature = "webrtc-backend")]
use std::sync::Arc;

use crate::channel::{DataChannel, DataChannelInit};
use crate::sdp::{SdpType, SessionDescription};
use crate::RtcError;

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

pub struct PeerConnectionFactory {
    #[cfg(feature = "webrtc-backend")]
    api: webrtc::api::API,
}

#[cfg(not(feature = "webrtc-backend"))]
impl PeerConnectionFactory {
    pub fn new() -> Self { Self {} }
    pub async fn create_peer_connection(&self, _config: PcConfig) -> Result<PeerConnection, RtcError> {
        tracing::info!("Creating PeerConnection (stub)");
        Ok(PeerConnection {})
    }
}

#[cfg(feature = "webrtc-backend")]
impl PeerConnectionFactory {
    pub fn new() -> Self {
        let api = webrtc::api::APIBuilder::new().build();
        Self { api }
    }

    pub async fn create_peer_connection(&self, config: PcConfig) -> Result<PeerConnection, RtcError> {
        tracing::info!("Creating PeerConnection (webrtc-rs)");
        let mut cfg = webrtc::peer_connection::configuration::RTCConfiguration::default();
        for srv in &config.ice_servers {
            cfg.ice_servers.push(webrtc::ice_transport::ice_server::RTCIceServer {
                urls: srv.urls.clone(),
                username: srv.username.clone(),
                credential: srv.password.clone(),
            });
        }
        let pc = self.api.new_peer_connection(cfg).await
            .map_err(|e| RtcError::PeerConnection(e.to_string()))?;
        Ok(PeerConnection { inner: Some(Arc::new(pc)) })
    }
}

impl Default for PeerConnectionFactory {
    fn default() -> Self { Self::new() }
}

// ── PeerConnection ──

pub struct PeerConnection {
    #[cfg(feature = "webrtc-backend")]
    inner: Option<Arc<webrtc::peer_connection::RTCPeerConnection>>,
}

#[cfg(not(feature = "webrtc-backend"))]
impl PeerConnection {
    pub async fn create_offer(&self, _: &OfferOptions) -> Result<SessionDescription, RtcError> {
        Ok(SessionDescription::new(SdpType::Offer, String::new()))
    }
    pub async fn create_answer(&self, _: &AnswerOptions) -> Result<SessionDescription, RtcError> {
        Ok(SessionDescription::new(SdpType::Answer, String::new()))
    }
    pub async fn set_local_description(&self, _: &SessionDescription) -> Result<(), RtcError> { Ok(()) }
    pub async fn set_remote_description(&self, _: &SessionDescription) -> Result<(), RtcError> { Ok(()) }
    pub async fn add_ice_candidate(&self, _: &IceCandidate) -> Result<(), RtcError> { Ok(()) }
    pub async fn create_data_channel(&self, label: &str, _: DataChannelInit) -> Result<DataChannel, RtcError> {
        Ok(DataChannel { label: label.to_string(), id: 0 })
    }
    pub fn connection_state(&self) -> PeerConnectionState { PeerConnectionState::New }
    pub fn ice_connection_state(&self) -> IceConnectionState { IceConnectionState::New }
    pub fn ice_gathering_state(&self) -> IceGatheringState { IceGatheringState::New }
    pub fn signaling_state(&self) -> SignalingState { SignalingState::Stable }
    pub async fn close(&self) {}
}

#[cfg(feature = "webrtc-backend")]
impl PeerConnection {
    fn inner(&self) -> &Arc<webrtc::peer_connection::RTCPeerConnection> {
        self.inner.as_ref().expect("PeerConnection already closed")
    }

    pub async fn create_offer(&self, options: &OfferOptions) -> Result<SessionDescription, RtcError> {
        let mut opts = webrtc::peer_connection::offer_answer_options::RTCOfferOptions::default();
        if options.ice_restart { opts.ice_restart = true; }
        let sdp = self.inner().create_offer(Some(opts)).await?;
        Ok(SessionDescription { sdp_type: SdpType::Offer, sdp: sdp.sdp })
    }

    pub async fn create_answer(&self, _: &AnswerOptions) -> Result<SessionDescription, RtcError> {
        let sdp = self.inner().create_answer(None).await?;
        Ok(SessionDescription { sdp_type: SdpType::Answer, sdp: sdp.sdp })
    }

    pub async fn set_local_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        let sdp = match desc.sdp_type {
            SdpType::Offer => webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(desc.sdp.clone())?,
            SdpType::Answer => webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(desc.sdp.clone())?,
            _ => return Err(RtcError::Sdp("unsupported SDP type".into())),
        };
        self.inner().set_local_description(sdp).await?;
        Ok(())
    }

    pub async fn set_remote_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        let sdp = match desc.sdp_type {
            SdpType::Offer => webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(desc.sdp.clone())?,
            SdpType::Answer => webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(desc.sdp.clone())?,
            _ => return Err(RtcError::Sdp("unsupported SDP type".into())),
        };
        self.inner().set_remote_description(sdp).await?;
        Ok(())
    }

    pub async fn add_ice_candidate(&self, candidate: &IceCandidate) -> Result<(), RtcError> {
        let c = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
            candidate: candidate.candidate.clone(),
            sdp_mid: candidate.sdp_mid.clone(),
            sdp_mline_index: candidate.sdp_mline_index,
            ..Default::default()
        };
        self.inner().add_ice_candidate(c).await?;
        Ok(())
    }

    pub async fn create_data_channel(&self, label: &str, _: DataChannelInit) -> Result<DataChannel, RtcError> {
        let dc = self.inner().create_data_channel(label, None).await
            .map_err(|e| RtcError::DataChannel(e.to_string()))?;
        Ok(DataChannel::from_webrtc(dc).await)
    }

    pub fn connection_state(&self) -> PeerConnectionState {
        use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::*;
        match self.inner().connection_state() {
            New => PeerConnectionState::New, Connecting => PeerConnectionState::Connecting,
            Connected => PeerConnectionState::Connected, Disconnected => PeerConnectionState::Disconnected,
            Failed => PeerConnectionState::Failed, Closed => PeerConnectionState::Closed,
            _ => PeerConnectionState::New,
        }
    }
    pub fn ice_connection_state(&self) -> IceConnectionState {
        use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::*;
        match self.inner().ice_connection_state() {
            New => IceConnectionState::New, Checking => IceConnectionState::Checking,
            Connected => IceConnectionState::Connected, Completed => IceConnectionState::Completed,
            Failed => IceConnectionState::Failed, Disconnected => IceConnectionState::Disconnected,
            Closed => IceConnectionState::Closed, _ => IceConnectionState::New,
        }
    }
    pub fn ice_gathering_state(&self) -> IceGatheringState {
        use webrtc::ice_transport::ice_gathering_state::RTCIceGatheringState::*;
        match self.inner().ice_gathering_state() {
            Unspecified | New => IceGatheringState::New, Gathering => IceGatheringState::Gathering,
            Complete => IceGatheringState::Complete, _ => IceGatheringState::New,
        }
    }
    pub fn signaling_state(&self) -> SignalingState {
        use webrtc::peer_connection::signaling_state::RTCSignalingState::*;
        match self.inner().signaling_state() {
            Stable => SignalingState::Stable, HaveLocalOffer => SignalingState::HaveLocalOffer,
            Closed => SignalingState::Closed, _ => SignalingState::Stable,
        }
    }

    pub async fn close(&self) {
        if let Some(pc) = &self.inner {
            let _ = pc.close().await;
        }
    }

    /// Set on_data_channel callback. The closure receives the raw RTCDataChannel.
    /// Caller should use DataChannel::from_webrtc to create a wrapper, then spool it.
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
        self.inner().on_data_channel(f);
    }

    /// Set on_ice_candidate callback. None means gathering complete.
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
        self.inner().on_ice_candidate(f);
    }
}

impl Clone for PeerConnection {
    fn clone(&self) -> Self {
        #[cfg(feature = "webrtc-backend")]
        { Self { inner: self.inner.clone() } }
        #[cfg(not(feature = "webrtc-backend"))]
        { Self {} }
    }
}

#[cfg(all(test, not(feature = "webrtc-backend")))]
mod tests {
    use super::*;

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
}
