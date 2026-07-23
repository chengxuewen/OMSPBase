//! webrtc-rs backend — wraps the `webrtc` crate types.
//! Enabled via `backend-webrtc-rs` feature.

use std::sync::{Arc, Mutex};

use super::DcBackend;
use super::PcBackend;
use super::TrackWriteBackend;
use crate::data_channel::{
    RTCDataChannel as PubDataChannel, RTCDataChannelEvent, RTCDataChannelInit,
    RTCDataChannelRx, RTCDataChannelState, RTCDataMessage,
};
use crate::peer_connection::{
    RTCAnswerOptions, RTCIceCandidate, RTCOfferOptions,
    RTCIceConnectionState, RTCIceGatheringState, RTCPeerConnectionState, RTCSignalingState,
};
use crate::sdp::{RTCSdpType, RTCSessionDescription};
use crate::track::{RTCAudioTrackConfig, TrackKind};
use crate::RTCError;
use omspbase_codec::{
    CodecFactory, EncoderConfig, EncoderPreset, Bitrate, CodecId, PixelFormat,
    VideoEncoder, VideoFrame, VideoFormat, Plane,
};

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
        _init: RTCDataChannelInit,
    ) -> Result<PubDataChannel, RTCError> {
        let dc = self
            .inner
            .create_data_channel(label, None)
            .await
            .map_err(|e| RTCError::RTCDataChannel(e.to_string()))?;
        let id = dc.id() as i32;
        Ok(PubDataChannel {
            label: label.to_string(),
            id,
            backend: WebrtcRsDc { inner: dc },
        })
    }
}

impl PcBackend for WebrtcRsPc {
    async fn create_offer(&self, options: &RTCOfferOptions) -> Result<RTCSessionDescription, RTCError> {
        let mut opts = webrtc::peer_connection::offer_answer_options::RTCOfferOptions::default();
        if options.ice_restart {
            opts.ice_restart = true;
        }
        let sdp = self.inner.create_offer(Some(opts)).await?;
        Ok(RTCSessionDescription {
            sdp_type: RTCSdpType::Offer,
            sdp: sdp.sdp,
        })
    }

    async fn create_answer(&self, _: &RTCAnswerOptions) -> Result<RTCSessionDescription, RTCError> {
        let sdp = self.inner.create_answer(None).await?;
        Ok(RTCSessionDescription {
            sdp_type: RTCSdpType::Answer,
            sdp: sdp.sdp,
        })
    }

    async fn set_local_description(&self, desc: &RTCSessionDescription) -> Result<(), RTCError> {
        let sdp = match desc.sdp_type {
            RTCSdpType::Offer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(
                    desc.sdp.clone(),
                )?
            }
            RTCSdpType::Answer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(
                    desc.sdp.clone(),
                )?
            }
            _ => return Err(RTCError::Sdp("unsupported SDP type".into())),
        };
        self.inner.set_local_description(sdp).await?;
        Ok(())
    }

    async fn set_remote_description(&self, desc: &RTCSessionDescription) -> Result<(), RTCError> {
        let sdp = match desc.sdp_type {
            RTCSdpType::Offer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(
                    desc.sdp.clone(),
                )?
            }
            RTCSdpType::Answer => {
                webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(
                    desc.sdp.clone(),
                )?
            }
            _ => return Err(RTCError::Sdp("unsupported SDP type".into())),
        };
        self.inner.set_remote_description(sdp).await?;
        Ok(())
    }

    async fn add_ice_candidate(&self, candidate: &RTCIceCandidate) -> Result<(), RTCError> {
        let c = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
            candidate: candidate.candidate.clone(),
            sdp_mid: candidate.sdp_mid.clone(),
            sdp_mline_index: candidate.sdp_mline_index,
            ..Default::default()
        };
        self.inner.add_ice_candidate(c).await?;
        Ok(())
    }

    fn connection_state(&self) -> RTCPeerConnectionState {
        use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::*;
        match self.inner.connection_state() {
            New => RTCPeerConnectionState::New,
            Connecting => RTCPeerConnectionState::Connecting,
            Connected => RTCPeerConnectionState::Connected,
            Disconnected => RTCPeerConnectionState::Disconnected,
            Failed => RTCPeerConnectionState::Failed,
            Closed => RTCPeerConnectionState::Closed,
            _ => RTCPeerConnectionState::New,
        }
    }

    fn ice_connection_state(&self) -> RTCIceConnectionState {
        use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState::*;
        match self.inner.ice_connection_state() {
            New => RTCIceConnectionState::New,
            Checking => RTCIceConnectionState::Checking,
            Connected => RTCIceConnectionState::Connected,
            Completed => RTCIceConnectionState::Completed,
            Failed => RTCIceConnectionState::Failed,
            Disconnected => RTCIceConnectionState::Disconnected,
            Closed => RTCIceConnectionState::Closed,
            _ => RTCIceConnectionState::New,
        }
    }

    fn ice_gathering_state(&self) -> RTCIceGatheringState {
        use webrtc::ice_transport::ice_gathering_state::RTCIceGatheringState::*;
        match self.inner.ice_gathering_state() {
            Unspecified | New => RTCIceGatheringState::New,
            Gathering => RTCIceGatheringState::Gathering,
            Complete => RTCIceGatheringState::Complete,
        }
    }

    fn signaling_state(&self) -> RTCSignalingState {
        use webrtc::peer_connection::signaling_state::RTCSignalingState::*;
        match self.inner.signaling_state() {
            Stable => RTCSignalingState::Stable,
            HaveLocalOffer => RTCSignalingState::HaveLocalOffer,
            Closed => RTCSignalingState::Closed,
            _ => RTCSignalingState::Stable,
        }
    }

    async fn close(&self) {
        let _ = self.inner.close().await;
    }

    /// Override: store on_track callback (webrtc-rs bridge deferred to Phase 2).
    fn set_on_track(&self, cb: Box<dyn Fn(crate::track::TrackReceiver) + Send + Sync + 'static>) {
        // ponytail: store callback for future webrtc-rs on_track bridge
        // webrtc-rs RTCPeerConnection::on_track provides TrackRemote directly
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
    fn state(&self) -> RTCDataChannelState {
        use webrtc::data_channel::data_channel_state::RTCDataChannelState::*;
        match self.inner.ready_state() {
            Connecting => RTCDataChannelState::Connecting,
            Open => RTCDataChannelState::Open,
            Closing => RTCDataChannelState::Closing,
            Closed => RTCDataChannelState::Closed,
            _ => RTCDataChannelState::Closed,
        }
    }

    async fn send(&self, data: &[u8]) -> Result<(), RTCError> {
        let b = bytes::Bytes::copy_from_slice(data);
        self.inner
            .send(&b)
            .await
            .map(|_| ())
            .map_err(|e| RTCError::RTCDataChannel(e.to_string()))
    }

    async fn send_text(&self, text: &str) -> Result<(), RTCError> {
        self.inner
            .send_text(text)
            .await
            .map(|_| ())
            .map_err(|e| RTCError::RTCDataChannel(e.to_string()))
    }

    async fn spool(&self) -> RTCDataChannelRx {
        let dc = self.inner.clone();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let tx2 = tx.clone();
        dc.on_open(Box::new(move || {
            let _ = tx2.send(RTCDataChannelEvent::Open);
            Box::pin(async {})
        }));
        let tx2 = tx.clone();
        dc.on_close(Box::new(move || {
            let _ = tx2.send(RTCDataChannelEvent::Closed);
            Box::pin(async {})
        }));
        let tx2 = tx.clone();
        dc.on_message(Box::new(move |msg| {
            let data = msg.data.to_vec();
            let _ = tx2.send(RTCDataChannelEvent::Message(RTCDataMessage { data }));
            Box::pin(async {})
        }));
        dc.on_error(Box::new(move |err| {
            let _ = tx.send(RTCDataChannelEvent::Error(err.to_string()));
            Box::pin(async {})
        }));
        RTCDataChannelRx::new(Some(rx))
    }

    async fn close(&mut self) {
        self.inner.close().await.ok();
    }
}

// ── WebrtcRsTrack ──

pub(crate) struct WebrtcRsTrack {
    inner: Option<Arc<webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample>>,
    encoder: Mutex<Option<Box<dyn VideoEncoder>>>,
}

impl WebrtcRsTrack {
    pub(crate) fn new(
        track: Arc<webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample>,
    ) -> Self {
        Self {
            inner: Some(track),
            encoder: Mutex::new(None),
        }
    }

    /// Initialize the H.264 encoder. Called before first write_raw_i420.
    pub(crate) fn init_encoder(&self, width: u32, height: u32) -> Result<(), RTCError> {
        let config = EncoderConfig {
            codec: CodecId::H264,
            format: VideoFormat {
                width,
                height,
                pixel_format: PixelFormat::Yuv420p,
            },
            bitrate: Bitrate::Vbr { target: 2_000_000, max: 4_000_000 },
            fps: omspbase_codec::FrameRate { num: 30, den: 1 },
            preset: EncoderPreset::P1UltraFast,
            gop: 30,
        };
        let factory = CodecFactory::new();
        let mut encoder = factory
            .create_encoder(config.clone(), None)
            .map_err(|e| RTCError::Track(format!("codec: {e}")))?;
        encoder
            .configure(&config)
            .map_err(|e| RTCError::Track(format!("codec configure: {e}")))?;
        *self.encoder.lock().unwrap() = Some(encoder);
        Ok(())
    }
}

impl Default for WebrtcRsTrack {
    fn default() -> Self {
        Self { inner: None, encoder: Mutex::new(None) }
    }
}

impl std::fmt::Debug for WebrtcRsTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcRsTrack").field("has_track", &self.inner.is_some()).finish()
    }
}

impl Clone for WebrtcRsTrack {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), encoder: Mutex::new(None) }
    }
}

impl TrackWriteBackend for WebrtcRsTrack {
    async fn write_frame(
        &self,
        data: &[u8],
        _kind: TrackKind,
        audio_config: Option<&RTCAudioTrackConfig>,
    ) -> Result<(), RTCError> {
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
                .map_err(|e| RTCError::Track(e.to_string()))?;
        }
        Ok(())
    }
    async fn write_raw_i420(
        &self, data: &[u8], width: u32, height: u32,
    ) -> Result<(), RTCError> {
        let y_size = (width * height) as usize;
        let uv_size = ((width / 2) * (height / 2)) as usize;
        if data.len() < y_size + 2 * uv_size {
            return Err(RTCError::Track("I420 data too short".into()));
        }
        let frame = VideoFrame {
            format: VideoFormat { width, height, pixel_format: PixelFormat::Yuv420p },
            planes: vec![
                Plane { data: data[..y_size].to_vec(), stride: width },
                Plane { data: data[y_size..y_size+uv_size].to_vec(), stride: width/2 },
                Plane { data: data[y_size+uv_size..y_size+2*uv_size].to_vec(), stride: width/2 },
            ],
            pts: 0,
            keyframe: false,
        };
        // Push frame under lock, release before writing
        {
            let mut guard = self.encoder.lock().unwrap();
            let enc = guard.as_mut()
                .ok_or_else(|| RTCError::Track("encoder not initialized".into()))?;
            enc.push_frame(&frame)
                .map_err(|e| RTCError::Track(format!("codec push: {e}")))?;
        }
        // Drain packets — acquire/release lock per iteration
        loop {
            let packet = {
                let mut guard = self.encoder.lock().unwrap();
                let enc = guard.as_mut()
                    .ok_or_else(|| RTCError::Track("encoder not initialized".into()))?;
                enc.pull_packet()
                    .map_err(|e| RTCError::Track(format!("codec pull: {e}")))?
            };
            match packet {
                Some(p) => self.write_frame(&p.data, TrackKind::Video, None).await?,
                None => break,
            }
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
        config: crate::peer_connection::RTCConfiguration,
    ) -> Result<WebrtcRsPc, RTCError> {
        tracing::info!("Creating RTCPeerConnection (webrtc-rs)");
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
            .map_err(|e| RTCError::RTCPeerConnection(e.to_string()))?;
        Ok(WebrtcRsPc::new(Arc::new(pc)))
    }
}
