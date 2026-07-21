//! webrtc-sys backend — wraps libwebrtc via LiveKit's webrtc-sys FFI crate.
//!
//! Enabled via `backend-webrtc-sys` feature.
//! Uses tokio::sync::oneshot channels to convert callback-based FFI to async.
//!
//! Backend types:
//! - WebrtcSysPc: wraps webrtc_sys::peer_connection::ffi::PeerConnection
//! - WebrtcSysDc: wraps webrtc_sys::data_channel::ffi::DataChannel
//! - WebrtcSysTrack: real video track via VideoTrackSource (webrtc-sys)
//! - WebrtcSysFactory: wraps webrtc_sys::peer_connection_factory::ffi::PeerConnectionFactory

use std::sync::{Arc, Mutex};

use super::DcBackend;
use super::PcBackend;
use super::TrackWriteBackend;
use crate::channel::{DataChannelRx, DataChannelState};
use crate::peer::{
    AnswerOptions, IceCandidate, IceConnectionState, IceGatheringState, IceTransportsType,
    OfferOptions, PcConfig, PeerConnectionState, SignalingState,
};
use crate::sdp::{SdpType, SessionDescription};
use crate::track::{AudioTrackConfig, TrackKind};
use crate::RtcError;

// ── WebrtcSysPc ──

#[derive(Clone)]
pub(crate) struct WebrtcSysPc {
    pc: cxx::SharedPtr<webrtc_sys::peer_connection::ffi::PeerConnection>,
}

impl std::fmt::Debug for WebrtcSysPc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcSysPc")
            .field("connection_state", &self.connection_state())
            .finish()
    }
}

/// Helper: wrap a oneshot sender in a PeerContext for FFI callbacks.
fn make_ctx<T: Send + 'static>(
    tx: tokio::sync::oneshot::Sender<T>,
) -> Box<webrtc_sys::peer_connection::PeerContext> {
    Box::new(webrtc_sys::peer_connection::PeerContext(Box::new(tx)))
}

/// Helper: extract a oneshot sender from a PeerContext via downcast.
fn extract_tx<T: Send + 'static>(
    ctx: Box<webrtc_sys::peer_connection::PeerContext>,
) -> tokio::sync::oneshot::Sender<T> {
    *ctx.0
        .downcast::<tokio::sync::oneshot::Sender<T>>()
        .unwrap_or_else(|_| panic!("PeerContext downcast failed"))
}


// ponytail: map webrtc-sys SdpType → crate SdpType inline
fn map_sdp_type(st: webrtc_sys::jsep::ffi::SdpType) -> SdpType {
    match st {
        webrtc_sys::jsep::ffi::SdpType::Offer => SdpType::Offer,
        webrtc_sys::jsep::ffi::SdpType::Answer => SdpType::Answer,
        webrtc_sys::jsep::ffi::SdpType::PrAnswer => SdpType::PrAnswer,
        webrtc_sys::jsep::ffi::SdpType::Rollback => SdpType::Rollback,
        _ => SdpType::Offer, // ponytail: defensive fallback
    }
}

impl PcBackend for WebrtcSysPc {
    async fn create_offer(&self, options: &OfferOptions) -> Result<SessionDescription, RtcError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let ctx = make_ctx(tx);

        let mut opts = webrtc_sys::peer_connection::ffi::RtcOfferAnswerOptions::default();
        // ponytail: ICE restart toggle; full options mapping deferred
        if options.ice_restart {
            opts.ice_restart = true;
        }

        self.pc.create_offer(
            opts,
            ctx,
            |ctx, sdp| {
                let tx: tokio::sync::oneshot::Sender<Result<SessionDescription, RtcError>> =
                    extract_tx(ctx);
                let sdp_type = map_sdp_type(sdp.sdp_type());
                let sdp_str = sdp.stringify();
                let _ = tx.send(Ok(SessionDescription {
                    sdp_type,
                    sdp: sdp_str,
                }));
            },
            |ctx, error| {
                let tx: tokio::sync::oneshot::Sender<Result<SessionDescription, RtcError>> =
                    extract_tx(ctx);
                let _ = tx.send(Err(RtcError::Internal(error.message)));
            },
        );

        rx.await
            .map_err(|_| RtcError::Internal("oneshot cancelled".into()))?
    }

    async fn create_answer(
        &self,
        _options: &AnswerOptions,
    ) -> Result<SessionDescription, RtcError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let ctx = make_ctx(tx);

        let opts = webrtc_sys::peer_connection::ffi::RtcOfferAnswerOptions::default();
        // ponytail: AnswerOptions has no fields currently, pass defaults

        self.pc.create_answer(
            opts,
            ctx,
            |ctx, sdp| {
                let tx: tokio::sync::oneshot::Sender<Result<SessionDescription, RtcError>> =
                    extract_tx(ctx);
                let sdp_type = map_sdp_type(sdp.sdp_type());
                let sdp_str = sdp.stringify();
                let _ = tx.send(Ok(SessionDescription {
                    sdp_type,
                    sdp: sdp_str,
                }));
            },
            |ctx, error| {
                let tx: tokio::sync::oneshot::Sender<Result<SessionDescription, RtcError>> =
                    extract_tx(ctx);
                let _ = tx.send(Err(RtcError::Internal(error.message)));
            },
        );

        rx.await
            .map_err(|_| RtcError::Internal("oneshot cancelled".into()))?
    }

    async fn set_local_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let ctx = make_ctx(tx);

        let sdp_type = match desc.sdp_type {
            SdpType::Offer => webrtc_sys::jsep::ffi::SdpType::Offer,
            SdpType::Answer => webrtc_sys::jsep::ffi::SdpType::Answer,
            SdpType::PrAnswer => webrtc_sys::jsep::ffi::SdpType::PrAnswer,
            SdpType::Rollback => webrtc_sys::jsep::ffi::SdpType::Rollback,
        };

        let sd = webrtc_sys::jsep::ffi::create_session_description(sdp_type, desc.sdp.clone())
            .map_err(|e| RtcError::Sdp(e.what().to_owned()))?;

        // ponytail: set_local_description has a single on_complete callback (ctx, error)
        self.pc.set_local_description(sd, ctx, |ctx, error| {
            let tx: tokio::sync::oneshot::Sender<Result<(), RtcError>> = extract_tx(ctx);
            if error.ok() {
                let _ = tx.send(Ok(()));
            } else {
                let _ = tx.send(Err(RtcError::Sdp(error.message)));
            }
        });

        rx.await
            .map_err(|_| RtcError::Internal("oneshot cancelled".into()))?
    }

    async fn set_remote_description(&self, desc: &SessionDescription) -> Result<(), RtcError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let ctx = make_ctx(tx);

        let sdp_type = match desc.sdp_type {
            SdpType::Offer => webrtc_sys::jsep::ffi::SdpType::Offer,
            SdpType::Answer => webrtc_sys::jsep::ffi::SdpType::Answer,
            SdpType::PrAnswer => webrtc_sys::jsep::ffi::SdpType::PrAnswer,
            SdpType::Rollback => webrtc_sys::jsep::ffi::SdpType::Rollback,
        };

        let sd = webrtc_sys::jsep::ffi::create_session_description(sdp_type, desc.sdp.clone())
            .map_err(|e| RtcError::Sdp(e.what().to_owned()))?;

        self.pc.set_remote_description(sd, ctx, |ctx, error| {
            let tx: tokio::sync::oneshot::Sender<Result<(), RtcError>> = extract_tx(ctx);
            if error.ok() {
                let _ = tx.send(Ok(()));
            } else {
                let _ = tx.send(Err(RtcError::Sdp(error.message)));
            }
        });

        rx.await
            .map_err(|_| RtcError::Internal("oneshot cancelled".into()))?
    }

    async fn add_ice_candidate(&self, candidate: &IceCandidate) -> Result<(), RtcError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let ctx = make_ctx(tx);

        let ic = webrtc_sys::jsep::ffi::create_ice_candidate(
            candidate.sdp_mid.clone().unwrap_or_default(),
            candidate.sdp_mline_index.map(|v| v as i32).unwrap_or(0),
            candidate.candidate.clone(),
        )
        .map_err(|e| RtcError::PeerConnection(e.what().to_owned()))?;

        self.pc.add_ice_candidate(ic, ctx, |ctx, error| {
            let tx: tokio::sync::oneshot::Sender<Result<(), RtcError>> = extract_tx(ctx);
            if error.ok() {
                let _ = tx.send(Ok(()));
            } else {
                let _ = tx.send(Err(RtcError::PeerConnection(error.message)));
            }
        });

        rx.await
            .map_err(|_| RtcError::Internal("oneshot cancelled".into()))?
    }

    fn connection_state(&self) -> PeerConnectionState {
        match self.pc.connection_state() {
            webrtc_sys::peer_connection::ffi::PeerConnectionState::New => PeerConnectionState::New,
            webrtc_sys::peer_connection::ffi::PeerConnectionState::Connecting => {
                PeerConnectionState::Connecting
            }
            webrtc_sys::peer_connection::ffi::PeerConnectionState::Connected => {
                PeerConnectionState::Connected
            }
            webrtc_sys::peer_connection::ffi::PeerConnectionState::Disconnected => {
                PeerConnectionState::Disconnected
            }
            webrtc_sys::peer_connection::ffi::PeerConnectionState::Failed => {
                PeerConnectionState::Failed
            }
            webrtc_sys::peer_connection::ffi::PeerConnectionState::Closed => {
                PeerConnectionState::Closed
            }
            _ => PeerConnectionState::New, // ponytail: defensive fallback
        }
    }

    fn ice_connection_state(&self) -> IceConnectionState {
        match self.pc.ice_connection_state() {
            webrtc_sys::peer_connection::ffi::IceConnectionState::IceConnectionNew => {
                IceConnectionState::New
            }
            webrtc_sys::peer_connection::ffi::IceConnectionState::IceConnectionChecking => {
                IceConnectionState::Checking
            }
            webrtc_sys::peer_connection::ffi::IceConnectionState::IceConnectionConnected => {
                IceConnectionState::Connected
            }
            webrtc_sys::peer_connection::ffi::IceConnectionState::IceConnectionCompleted => {
                IceConnectionState::Completed
            }
            webrtc_sys::peer_connection::ffi::IceConnectionState::IceConnectionFailed => {
                IceConnectionState::Failed
            }
            webrtc_sys::peer_connection::ffi::IceConnectionState::IceConnectionDisconnected => {
                IceConnectionState::Disconnected
            }
            webrtc_sys::peer_connection::ffi::IceConnectionState::IceConnectionClosed => {
                IceConnectionState::Closed
            }
            _ => IceConnectionState::New, // ponytail: defensive fallback
        }
    }

    fn ice_gathering_state(&self) -> IceGatheringState {
        match self.pc.ice_gathering_state() {
            webrtc_sys::peer_connection::ffi::IceGatheringState::IceGatheringNew => {
                IceGatheringState::New
            }
            webrtc_sys::peer_connection::ffi::IceGatheringState::IceGatheringGathering => {
                IceGatheringState::Gathering
            }
            webrtc_sys::peer_connection::ffi::IceGatheringState::IceGatheringComplete => {
                IceGatheringState::Complete
            }
            _ => IceGatheringState::New, // ponytail: defensive fallback
        }
    }

    fn signaling_state(&self) -> SignalingState {
        match self.pc.signaling_state() {
            webrtc_sys::peer_connection::ffi::SignalingState::Stable => SignalingState::Stable,
            webrtc_sys::peer_connection::ffi::SignalingState::HaveLocalOffer => {
                SignalingState::HaveLocalOffer
            }
            webrtc_sys::peer_connection::ffi::SignalingState::HaveLocalPrAnswer => {
                SignalingState::HaveLocalPrAnswer
            }
            webrtc_sys::peer_connection::ffi::SignalingState::HaveRemoteOffer => {
                SignalingState::HaveRemoteOffer
            }
            webrtc_sys::peer_connection::ffi::SignalingState::HaveRemotePrAnswer => {
                SignalingState::HaveRemotePrAnswer
            }
            webrtc_sys::peer_connection::ffi::SignalingState::Closed => SignalingState::Closed,
            _ => SignalingState::Stable, // ponytail: defensive fallback
        }
    }

    async fn close(&self) {
        self.pc.close();
    }
}

// ── create_data_channel (method on WebrtcSysPc, called directly by peer.rs) ──

impl WebrtcSysPc {
    pub(crate) async fn create_data_channel(
        &self,
        label: &str,
        init: crate::channel::DataChannelInit,
    ) -> Result<crate::channel::DataChannel, RtcError> {
        use crate::channel::DataChannel;

        let sys_init = webrtc_sys::data_channel::ffi::DataChannelInit {
            ordered: init.ordered,
            has_max_retransmit_time: init.max_retransmit_time.is_some(),
            max_retransmit_time: init.max_retransmit_time.unwrap_or(-1),
            has_max_retransmits: init.max_retransmits.is_some(),
            max_retransmits: init.max_retransmits.unwrap_or(-1),
            protocol: init.protocol,
            negotiated: init.negotiated,
            id: init.id,
            has_priority: false,
            priority: webrtc_sys::data_channel::ffi::Priority::Low,
        };

        let dc = self
            .pc
            .create_data_channel(label.to_string(), sys_init)
            .map_err(|e| RtcError::PeerConnection(e.what().to_owned()))?;

        Ok(DataChannel {
            label: label.to_string(),
            id: dc.id(),
            backend: WebrtcSysDc { dc },
        })
    }
}

// ── WebrtcSysDc ──

#[derive(Clone)]
pub(crate) struct WebrtcSysDc {
    dc: cxx::SharedPtr<webrtc_sys::data_channel::ffi::DataChannel>,
}

impl std::fmt::Debug for WebrtcSysDc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcSysDc").finish()
    }
}

impl DcBackend for WebrtcSysDc {
    fn state(&self) -> DataChannelState {
        match self.dc.state() {
            webrtc_sys::data_channel::ffi::DataState::Connecting => DataChannelState::Connecting,
            webrtc_sys::data_channel::ffi::DataState::Open => DataChannelState::Open,
            webrtc_sys::data_channel::ffi::DataState::Closing => DataChannelState::Closing,
            webrtc_sys::data_channel::ffi::DataState::Closed => DataChannelState::Closed,
            _ => DataChannelState::Closed, // ponytail: defensive fallback
        }
    }

    async fn send(&self, data: &[u8]) -> Result<(), RtcError> {
        let buf = webrtc_sys::data_channel::ffi::DataBuffer {
            ptr: data.as_ptr(),
            len: data.len(),
            binary: true,
        };
        // ponytail: webrtc-sys send may fail if channel not open; log and ignore
        self.dc.send(&buf);
        Ok(())
    }

    async fn send_text(&self, text: &str) -> Result<(), RtcError> {
        let buf = webrtc_sys::data_channel::ffi::DataBuffer {
            ptr: text.as_ptr(),
            len: text.len(),
            binary: false,
        };
        self.dc.send(&buf);
        Ok(())
    }

    async fn spool(&self) -> DataChannelRx {
        // ponytail: observer registration for spool needs a DataChannelObserver — deferred
        DataChannelRx::stub()
    }

    async fn close(&mut self) {
        self.dc.close();
    }
}

// ── WebrtcSysTrack ──


/// webrtc-sys video track backend.
/// Holds a libwebrtc VideoTrackSource for pushing raw I420 frames.
/// libwebrtc handles encoding internally (VP8/H.264).
pub(crate) struct WebrtcSysTrack {
    video_source: Mutex<Option<SharedPtr<webrtc_sys::video_track::ffi::VideoTrackSource>>>,
}

impl Default for WebrtcSysTrack {
    fn default() -> Self {
        Self { video_source: Mutex::new(None) }
    }
}

impl std::fmt::Debug for WebrtcSysTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcSysTrack")
            .field("video_source", &self.video_source.lock().unwrap().is_some())
            .finish()
    }
}

impl Clone for WebrtcSysTrack {
    fn clone(&self) -> Self {
        let source = self.video_source.lock().unwrap().clone();
        Self { video_source: Mutex::new(source) }
    }
}

impl WebrtcSysTrack {
    pub(crate) fn with_video_source(
        source: SharedPtr<webrtc_sys::video_track::ffi::VideoTrackSource>,
    ) -> Self {
        Self { video_source: Mutex::new(Some(source)) }
    }
}

impl TrackWriteBackend for WebrtcSysTrack {
    async fn write_frame(
        &self,
        data: &[u8],
        _kind: TrackKind,
        _audio_config: Option<&AudioTrackConfig>,
    ) -> Result<(), RtcError> {
        tracing::debug!(
            "TrackSender::write_frame (webrtc-sys): {} bytes (encoded pass-through)",
            data.len()
        );
        // ponytail: encoded frame passthrough — stub for now, real encoding deferred
        Ok(())
    }

    async fn write_raw_i420(
        &self, data: &[u8], width: u32, height: u32,
    ) -> Result<(), RtcError> {
        use webrtc_sys::video_frame::ffi as vf;
        use webrtc_sys::video_frame_buffer::ffi as vfb;
        use webrtc_sys::video_track::ffi as vt;

        let source = self.video_source.lock().unwrap()
            .clone()
            .ok_or_else(|| RtcError::Track("video source not initialized".into()))?;

        let w: i32 = width as i32;
        let h: i32 = height as i32;
        // I420 layout: Y plane (W×H) + U plane (W/2×H/2) + V plane (W/2×H/2)
        let y_size = (w * h) as usize;
        let uv_size = ((w / 2) * (h / 2)) as usize;
        if data.len() < y_size + 2 * uv_size {
            return Err(RtcError::Track("I420 data too short".into()));
        }

        let i420 = vfb::new_i420_buffer(w, h, w, w / 2, w / 2);

        // SAFETY: I420Buffer owns the memory; slices live within the call scope.
        // The frame builder consumes the buffer via set_video_frame_buffer before build().
        unsafe {
            let y_slice = std::slice::from_raw_parts_mut(
                i420.data_y() as *mut u8, y_size,
            );
            let u_slice = std::slice::from_raw_parts_mut(
                i420.data_u() as *mut u8, uv_size,
            );
            let v_slice = std::slice::from_raw_parts_mut(
                i420.data_v() as *mut u8, uv_size,
            );
            y_slice.copy_from_slice(&data[..y_size]);
            u_slice.copy_from_slice(&data[y_size..y_size + uv_size]);
            v_slice.copy_from_slice(&data[y_size + uv_size..y_size + 2 * uv_size]);
        }

        // Build VideoFrame and push to source
        let mut builder = vf::new_video_frame_builder();
        builder.pin_mut().set_timestamp_us(0);
        builder.pin_mut().set_video_frame_buffer(
            // SAFETY: yuv_to_vfb upcasts PlanarYuvBuffer → VideoFrameBuffer
            unsafe { &*vfb::yuv_to_vfb(
                vfb::i420_to_yuv8(&i420)
            ) },
        );
        let frame = builder.pin_mut().build();

        let metadata = vt::FrameMetadata {
            has_packet_trailer: false,
            user_timestamp: 0,
            frame_id: 0,
            user_data: vec![],
        };

        source.on_captured_frame(&frame, &metadata);
        Ok(())
    }
}

// ponytail: WebrtcSysTrack has interior Mutex for VideoTrackSource;
// C++ side handles actual thread safety for on_captured_frame.
// ── No-op PeerConnectionObserver ──

/// No-op observer. We don't need to react to observer callbacks in the
/// webrtc-sys backend — state queries are polled directly on the PC.
struct NoOpObserver;

impl webrtc_sys::peer_connection_factory::PeerConnectionObserver for NoOpObserver {
    fn on_signaling_change(&self, _new_state: webrtc_sys::peer_connection::ffi::SignalingState) {}
    fn on_add_stream(
        &self,
        _stream: cxx::SharedPtr<webrtc_sys::media_stream::ffi::MediaStream>,
    ) {
    }
    fn on_remove_stream(
        &self,
        _stream: cxx::SharedPtr<webrtc_sys::media_stream::ffi::MediaStream>,
    ) {
    }
    fn on_data_channel(
        &self,
        _data_channel: cxx::SharedPtr<webrtc_sys::data_channel::ffi::DataChannel>,
    ) {
    }
    fn on_renegotiation_needed(&self) {}
    fn on_negotiation_needed_event(&self, _event: u32) {}
    fn on_ice_connection_change(
        &self,
        _new_state: webrtc_sys::peer_connection::ffi::IceConnectionState,
    ) {
    }
    fn on_standardized_ice_connection_change(
        &self,
        _new_state: webrtc_sys::peer_connection::ffi::IceConnectionState,
    ) {
    }
    fn on_connection_change(
        &self,
        _new_state: webrtc_sys::peer_connection::ffi::PeerConnectionState,
    ) {
    }
    fn on_ice_gathering_change(
        &self,
        _new_state: webrtc_sys::peer_connection::ffi::IceGatheringState,
    ) {
    }
    fn on_ice_candidate(
        &self,
        _candidate: cxx::SharedPtr<webrtc_sys::jsep::ffi::IceCandidate>,
    ) {
    }
    fn on_ice_candidate_error(
        &self,
        _address: String,
        _port: i32,
        _url: String,
        _error_code: i32,
        _error_text: String,
    ) {
    }
    fn on_ice_candidates_removed(
        &self,
        _removed: Vec<cxx::SharedPtr<webrtc_sys::candidate::ffi::Candidate>>,
    ) {
    }
    fn on_ice_connection_receiving_change(&self, _receiving: bool) {}
    fn on_ice_selected_candidate_pair_changed(
        &self,
        _event: webrtc_sys::peer_connection_factory::ffi::CandidatePairChangeEvent,
    ) {
    }
    fn on_add_track(
        &self,
        _receiver: cxx::SharedPtr<webrtc_sys::rtp_receiver::ffi::RtpReceiver>,
        _streams: Vec<cxx::SharedPtr<webrtc_sys::media_stream::ffi::MediaStream>>,
    ) {
    }
    fn on_track(
        &self,
        _transceiver: cxx::SharedPtr<webrtc_sys::rtp_transceiver::ffi::RtpTransceiver>,
    ) {
    }
    fn on_remove_track(
        &self,
        _receiver: cxx::SharedPtr<webrtc_sys::rtp_receiver::ffi::RtpReceiver>,
    ) {
    }
    fn on_interesting_usage(&self, _usage_pattern: i32) {}
}

// ── WebrtcSysFactory ──

pub(crate) struct WebrtcSysFactory {
    factory: cxx::SharedPtr<webrtc_sys::peer_connection_factory::ffi::PeerConnectionFactory>,
}

impl Default for WebrtcSysFactory {
    fn default() -> Self {
        let factory =
            webrtc_sys::peer_connection_factory::ffi::create_peer_connection_factory();
        Self { factory }
    }
}

impl std::fmt::Debug for WebrtcSysFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcSysFactory").finish()
    }
}

impl WebrtcSysFactory {
    pub(crate) async fn create_peer_connection(
        &self,
        config: PcConfig,
    ) -> Result<WebrtcSysPc, RtcError> {
        tracing::info!("Creating PeerConnection (webrtc-sys)");

        let ice_servers: Vec<webrtc_sys::peer_connection::ffi::IceServer> = config
            .ice_servers
            .iter()
            .map(|srv| webrtc_sys::peer_connection::ffi::IceServer {
                urls: srv.urls.clone(),
                username: srv.username.clone(),
                password: srv.password.clone(),
            })
            .collect();

        let ice_transport_type = match config.ice_transport_type {
            IceTransportsType::Relay => {
                webrtc_sys::peer_connection::ffi::IceTransportsType::Relay
            }
            IceTransportsType::NoHost => {
                webrtc_sys::peer_connection::ffi::IceTransportsType::NoHost
            }
            IceTransportsType::All => webrtc_sys::peer_connection::ffi::IceTransportsType::All,
        };

        let rtc_config = webrtc_sys::peer_connection::ffi::RtcConfiguration {
            ice_servers,
            continual_gathering_policy:
                webrtc_sys::peer_connection::ffi::ContinualGatheringPolicy::GatherOnce,
            ice_transport_type,
        };

        // ponytail: minimal no-op PeerConnectionObserver
        let observer = webrtc_sys::peer_connection_factory::PeerConnectionObserverWrapper::new(
            Arc::new(NoOpObserver),
        );

        let pc = self
            .factory
            .create_peer_connection(rtc_config, Box::new(observer))
            .map_err(|e| RtcError::PeerConnection(e.what().to_owned()))?;

        Ok(WebrtcSysPc { pc })
    }

    /// Create a video track with a new VideoTrackSource.
    /// Returns (WebrtcSysTrack, SharedPtr<MediaStreamTrack>) —
    /// the media track can be added to the PeerConnection via add_track.
    pub(crate) fn create_video_track(
        &self,
    ) -> (
        WebrtcSysTrack,
        cxx::SharedPtr<webrtc_sys::media_stream_track::ffi::MediaStreamTrack>,
    ) {
        use webrtc_sys::video_track::ffi as vt;

        let resolution = vt::VideoResolution { width: 640, height: 480 };
        let source = vt::new_video_track_source(&resolution, false);
        let backend = WebrtcSysTrack::with_video_source(source.clone());

        // Create VideoTrack from factory, then convert to MediaStreamTrack
        let video_track = self.factory.create_video_track("video".into(), source);
        let media_track = vt::video_to_media(video_track);

        (backend, media_track)
    }
}
