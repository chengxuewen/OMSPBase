//! PeerConnection thin wrapper.
//!
//! Follows webrtc-kit pattern: holds a handle to the webrtc-sys
//! PeerConnection C++ object. Provides full SDP/ICE/DataChannel/track API.

use std::fmt;

use crate::channel::{DataChannel, DataChannelInit};
use crate::sdp::{SdpType, SessionDescription};
use crate::track::TrackReceiver;
use crate::RtcError;

// ============================================================================
// Connection state enums (W3C compatible)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceConnectionState {
    New,
    Checking,
    Connected,
    Completed,
    Failed,
    Disconnected,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceGatheringState {
    New,
    Gathering,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalingState {
    Stable,
    HaveLocalOffer,
    HaveLocalPrAnswer,
    HaveRemoteOffer,
    HaveRemotePrAnswer,
    Closed,
}

// ============================================================================
// Configuration types
// ============================================================================

/// ICE server (STUN/TURN).
#[derive(Debug, Clone)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub password: String,
}

/// PeerConnection configuration.
#[derive(Debug, Clone)]
pub struct PcConfig {
    pub ice_servers: Vec<IceServer>,
    pub ice_transport_type: IceTransportsType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceTransportsType {
    Relay,
    NoHost,
    All,
}

impl Default for PcConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec![],
            ice_transport_type: IceTransportsType::All,
        }
    }
}

/// SDP offer options.
#[derive(Debug, Clone, Default)]
pub struct OfferOptions {
    pub ice_restart: bool,
    pub offer_to_receive_audio: bool,
    pub offer_to_receive_video: bool,
}

/// SDP answer options.
#[derive(Debug, Clone, Default)]
pub struct AnswerOptions;

/// ICE candidate received during gathering.
#[derive(Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
}

// ============================================================================
// Callback types (follow webrtc-kit naming)
// ============================================================================

pub type OnIceCandidate = Box<dyn FnMut(IceCandidate) + Send + Sync>;
pub type OnConnectionChange = Box<dyn FnMut(PeerConnectionState) + Send + Sync>;
pub type OnIceConnectionChange = Box<dyn FnMut(IceConnectionState) + Send + Sync>;
pub type OnDataChannel = Box<dyn FnMut(DataChannel) + Send + Sync>;
pub type OnTrack = Box<dyn FnMut(TrackReceiver) + Send + Sync>;

// ============================================================================
// PeerConnectionFactory
// ============================================================================

/// Factory for creating PeerConnections.
///
/// ponytail: holds webrtc-sys PeerConnectionFactory. Add FFI when linking libwebrtc.
#[derive(Clone, Default)]
pub struct PeerConnectionFactory;

impl PeerConnectionFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn create_peer_connection(
        &self,
        _config: PcConfig,
    ) -> Result<PeerConnection, RtcError> {
        tracing::info!("Creating PeerConnection (stub — webrtc-sys FFI pending)");
        Ok(PeerConnection::new())
    }
}

impl fmt::Debug for PeerConnectionFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeerConnectionFactory").finish()
    }
}

// ============================================================================
// PeerConnection
// ============================================================================

/// W3C RTCPeerConnection — thin wrapper around webrtc-sys.
///
/// ponytail: hold the webrtc-sys handle directly; add FFI when libwebrtc is linked.
/// All async methods require a tokio runtime.
#[derive(Clone)]
pub struct PeerConnection;

impl PeerConnection {
    fn new() -> Self {
        Self
    }

    // --- SDP negotiation ---

    pub async fn create_offer(&self, _options: &OfferOptions) -> Result<SessionDescription, RtcError> {
        tracing::trace!("create_offer (stub)");
        Ok(SessionDescription::new(SdpType::Offer, String::new()))
    }

    pub async fn create_answer(&self, _options: &AnswerOptions) -> Result<SessionDescription, RtcError> {
        tracing::trace!("create_answer (stub)");
        Ok(SessionDescription::new(SdpType::Answer, String::new()))
    }

    pub async fn set_local_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        tracing::trace!(sdp_type = %desc.sdp_type, "set_local_description (stub)");
        Ok(())
    }

    pub async fn set_remote_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        tracing::trace!(sdp_type = %desc.sdp_type, "set_remote_description (stub)");
        Ok(())
    }

    // --- ICE ---

    pub async fn add_ice_candidate(&self, _candidate: &IceCandidate) -> Result<(), RtcError> {
        tracing::trace!("add_ice_candidate (stub)");
        Ok(())
    }

    // --- DataChannel ---

    pub fn create_data_channel(
        &self,
        label: &str,
        _init: DataChannelInit,
    ) -> Result<DataChannel, RtcError> {
        tracing::trace!(label, "create_data_channel (stub)");
        Ok(DataChannel {
            label: label.to_string(),
            id: 0,
        })
    }

    // --- State queries ---

    pub fn connection_state(&self) -> PeerConnectionState {
        PeerConnectionState::New
    }

    pub fn ice_connection_state(&self) -> IceConnectionState {
        IceConnectionState::New
    }

    pub fn ice_gathering_state(&self) -> IceGatheringState {
        IceGatheringState::New
    }

    pub fn signaling_state(&self) -> SignalingState {
        SignalingState::Stable
    }

    pub fn close(&self) {
        tracing::trace!("PeerConnection::close");
    }

    // --- Callback registration ---

    pub fn on_ice_candidate(&self, _cb: Option<OnIceCandidate>) {
        tracing::trace!("on_ice_candidate registered (stub)");
    }

    pub fn on_connection_state_change(&self, _cb: Option<OnConnectionChange>) {
        tracing::trace!("on_connection_state_change registered (stub)");
    }

    pub fn on_ice_connection_state_change(&self, _cb: Option<OnIceConnectionChange>) {
        tracing::trace!("on_ice_connection_state_change registered (stub)");
    }

    pub fn on_data_channel(&self, _cb: Option<OnDataChannel>) {
        tracing::trace!("on_data_channel registered (stub)");
    }

    pub fn on_track(&self, _cb: Option<OnTrack>) {
        tracing::trace!("on_track registered (stub)");
    }
}

impl fmt::Debug for PeerConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeerConnection")
            .field("state", &self.connection_state())
            .finish()
    }
}
