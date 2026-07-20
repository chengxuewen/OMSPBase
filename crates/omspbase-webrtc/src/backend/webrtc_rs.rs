//! webrtc-rs backend — wraps the `webrtc` crate types.
//! Enabled via `backend-webrtc-rs` feature.

use std::sync::Arc;

use super::DcBackend;
use super::PcBackend;
use super::TrackWriteBackend;
use crate::channel::{
    DataChannel as PubDataChannel, DataChannelEvent, DataChannelInit,
    DataChannelRx, DataChannelState, DataMessage,
};
use crate::peer::{
    AnswerOptions, IceCandidate, OfferOptions,
    IceConnectionState, IceGatheringState, PeerConnectionState, SignalingState,
};
use crate::sdp::{SdpType, SessionDescription};
use crate::track::{AudioTrackConfig, TrackKind};
use crate::RtcError;

// ── WebrtcRsPc ──

#[derive(Clone)]
pub(crate) struct WebrtcRsPc {
    inner: Arc<webrtc::peer_connection::RTCPeerConnection>,
}

impl std::fmt::Debug for WebrtcRsPc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcRsPc")
            .field("connection_state", &self.inner.connection_state())
            .finish()
    }
}

impl WebrtcRsPc {
    pub(crate) fn new(inner: Arc<webrtc::peer_connection::RTCPeerConnection>) -> Self {
        Self { inner }
    }

    /// Set on_data_channel callback. Caller should use PubDataChannel's
    /// constructor to wrap the raw RTCDataChannel.
    pub(crate) fn on_data_channel(
        &self,
        f: Box<
            dyn FnMut(
                    Arc<webrtc::data_channel::RTCDataChannel>,
                ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>
                + Send
                + Sync
                + 'static,
        >,
    ) {
        self.inner.on_data_channel(f);
    }

    /// Set on_ice_candidate callback. None means gathering complete.
    pub(crate) fn on_ice_candidate(
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
        self.inner.on_ice_candidate(f);
    }

    pub(crate) async fn create_data_channel(
        &self,
        label: &str,
        _init: DataChannelInit,
    ) -> Result<PubDataChannel, RtcError> {
        let dc = self
            .inner
            .create_data_channel(label, None)
            .await
            .map_err(|e| RtcError::DataChannel(e.to_string()))?;
        let id = dc.id() as i32;
        Ok(PubDataChannel {
            label: label.to_string(),
            id,
            backend: WebrtcRsDc { inner: dc },
        })
    }
}

impl PcBackend for WebrtcRsPc {
    async fn create_offer(&self, options: &OfferOptions) -> Result<SessionDescription, RtcError> {
        let mut opts = webrtc::peer_connection::offer_answer_options::RTCOfferOptions::default();
        if options.ice_restart {
            opts.ice_restart = true;
        }
        let sdp = self.inner.create_offer(Some(opts)).await?;
        Ok(SessionDescription {
            sdp_type: SdpType::Offer,
            sdp: sdp.sdp,
        })
    }

    async fn create_answer(&self, _: &AnswerOptions) -> Result<SessionDescription, RtcError> {
        let sdp = self.inner.create_answer(None).await?;
        Ok(SessionDescription {
            sdp_type: SdpType::Answer,
            sdp: sdp.sdp,
        })
    }

    async fn set_local_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        let sdp = match desc.sdp_type {
            SdpType::Offer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(
                    desc.sdp.clone(),
                )?
            }
            SdpType::Answer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(
                    desc.sdp.clone(),
                )?
            }
            _ => return Err(RtcError::Sdp("unsupported SDP type".into())),
        };
        self.inner.set_local_description(sdp).await?;
        Ok(())
    }

    async fn set_remote_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        let sdp = match desc.sdp_type {
            SdpType::Offer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(
                    desc.sdp.clone(),
                )?
            }
            SdpType::Answer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(
                    desc.sdp.clone(),
                )?
            }
            _ => return Err(RtcError::Sdp("unsupported SDP type".into())),
        };
        self.inner.set_remote_description(sdp).await?;
        Ok(())
    }

    async fn add_ice_candidate(&self, candidate: &IceCandidate) -> Result<(), RtcError> {
        let c = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
            candidate: candidate.candidate.clone(),
            sdp_mid: candidate.sdp_mid.clone(),
            sdp_mline_index: candidate.sdp_mline_index,
            ..Default::default()
        };
        self.inner.add_ice_candidate(c).await?;
        Ok(())
    }

    fn connection_state(&self) -> PeerConnectionState {
        use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::*;
        match self.inner.connection_state() {
            New => PeerConnectionState::New,
            Connecting => PeerConnectionState::Connecting,
            Connected => PeerConnectionState::Connected,
            Disconnected => PeerConnectionState::Disconnected,
            Failed => PeerConnectionState::Failed,
            Closed => PeerConnectionState::Closed,
            _ => PeerConnectionState::New,
        }
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::*;
        match self.inner.ice_connection_state() {
            New => IceConnectionState::New,
            Checking => IceConnectionState::Checking,
            Connected => IceConnectionState::Connected,
            Completed => IceConnectionState::Completed,
            Failed => IceConnectionState::Failed,
            Disconnected => IceConnectionState::Disconnected,
            Closed => IceConnectionState::Closed,
            _ => IceConnectionState::New,
        }
    }

    fn ice_gathering_state(&self) -> IceGatheringState {
        use webrtc::ice_transport::ice_gathering_state::RTCIceGatheringState::*;
        match self.inner.ice_gathering_state() {
            Unspecified | New => IceGatheringState::New,
            Gathering => IceGatheringState::Gathering,
            Complete => IceGatheringState::Complete,
        }
    }

    fn signaling_state(&self) -> SignalingState {
        use webrtc::peer_connection::signaling_state::RTCSignalingState::*;
        match self.inner.signaling_state() {
            Stable => SignalingState::Stable,
            HaveLocalOffer => SignalingState::HaveLocalOffer,
            Closed => SignalingState::Closed,
            _ => SignalingState::Stable,
        }
    }

    async fn close(&self) {
        let _ = self.inner.close().await;
    }
}

// ── WebrtcRsDc ──

#[derive(Clone)]
pub(crate) struct WebrtcRsDc {
    inner: Arc<webrtc::data_channel::RTCDataChannel>,
}

impl std::fmt::Debug for WebrtcRsDc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcRsDc")
            .field("id", &self.inner.id())
            .field("label", &self.inner.label())
            .finish()
    }
}
impl WebrtcRsDc {
    pub(crate) fn new(inner: Arc<webrtc::data_channel::RTCDataChannel>) -> Self {
        Self { inner }
    }
}
impl DcBackend for WebrtcRsDc {
    fn state(&self) -> DataChannelState {
        use webrtc::data_channel::data_channel_state::RTCDataChannelState::*;
        match self.inner.ready_state() {
            Connecting => DataChannelState::Connecting,
            Open => DataChannelState::Open,
            Closing => DataChannelState::Closing,
            Closed => DataChannelState::Closed,
            _ => DataChannelState::Closed,
        }
    }

    async fn send(&self, data: &[u8]) -> Result<(), RtcError> {
        let b = bytes::Bytes::copy_from_slice(data);
        self.inner
            .send(&b)
            .await
            .map(|_| ())
            .map_err(|e| RtcError::DataChannel(e.to_string()))
    }

    async fn send_text(&self, text: &str) -> Result<(), RtcError> {
        self.inner
            .send_text(text)
            .await
            .map(|_| ())
            .map_err(|e| RtcError::DataChannel(e.to_string()))
    }

    async fn spool(&self) -> DataChannelRx {
        let dc = self.inner.clone();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let tx2 = tx.clone();
        dc.on_open(Box::new(move || {
            let _ = tx2.send(DataChannelEvent::Open);
            Box::pin(async {})
        }));
        let tx2 = tx.clone();
        dc.on_close(Box::new(move || {
            let _ = tx2.send(DataChannelEvent::Closed);
            Box::pin(async {})
        }));
        let tx2 = tx.clone();
        dc.on_message(Box::new(move |msg| {
            let data = msg.data.to_vec();
            let _ = tx2.send(DataChannelEvent::Message(DataMessage { data }));
            Box::pin(async {})
        }));
        dc.on_error(Box::new(move |err| {
            let _ = tx.send(DataChannelEvent::Error(err.to_string()));
            Box::pin(async {})
        }));
        DataChannelRx::new(Some(rx))
    }

    async fn close(&mut self) {
        self.inner.close().await.ok();
    }
}

// ── WebrtcRsTrack ──

#[derive(Debug, Clone)]
pub(crate) struct WebrtcRsTrack {
    inner: Option<Arc<webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample>>,
}

impl WebrtcRsTrack {
    pub(crate) fn new(
        track: Arc<webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample>,
    ) -> Self {
        Self {
            inner: Some(track),
        }
    }
}

impl Default for WebrtcRsTrack {
    fn default() -> Self {
        Self { inner: None }
    }
}

impl TrackWriteBackend for WebrtcRsTrack {
    async fn write_frame(
        &self,
        data: &[u8],
        _kind: TrackKind,
        audio_config: Option<&AudioTrackConfig>,
    ) -> Result<(), RtcError> {
        if let Some(ref track) = self.inner {
            // ponytail: audio uses config frame duration, video uses 30fps
            let duration_ms = audio_config
                .map(|c| c.frame_duration_ms())
                .unwrap_or(33);
            let sample = webrtc::media::Sample {
                data: bytes::Bytes::copy_from_slice(data),
                duration: std::time::Duration::from_millis(duration_ms),
                ..Default::default()
            };
            track
                .write_sample(&sample)
                .await
                .map_err(|e| RtcError::Track(e.to_string()))?;
        }
        Ok(())
    }
}

// ── WebrtcRsFactory ──

pub(crate) struct WebrtcRsFactory {
    api: webrtc::api::API,
}

impl std::fmt::Debug for WebrtcRsFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcRsFactory").finish()
    }
}

impl Default for WebrtcRsFactory {
    fn default() -> Self {
        let api = webrtc::api::APIBuilder::new().build();
        Self { api }
    }
}

impl WebrtcRsFactory {
    pub(crate) async fn create_peer_connection(
        &self,
        config: crate::peer::PcConfig,
    ) -> Result<WebrtcRsPc, RtcError> {
        tracing::info!("Creating PeerConnection (webrtc-rs)");
        let mut cfg = webrtc::peer_connection::configuration::RTCConfiguration::default();
        for srv in &config.ice_servers {
            cfg.ice_servers.push(webrtc::ice_transport::ice_server::RTCIceServer {
                urls: srv.urls.clone(),
                username: srv.username.clone(),
                credential: srv.password.clone(),
            });
        }
        let pc = self
            .api
            .new_peer_connection(cfg)
            .await
            .map_err(|e| RtcError::PeerConnection(e.to_string()))?;
        Ok(WebrtcRsPc::new(Arc::new(pc)))
    }
}
