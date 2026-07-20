//! W3C RTCRtpParameters types.
//!
//! Ported from webrtc-kit rtc/core.rs.
//! Models RTP codec parameters, encoding parameters, header extensions,
//! RTCP parameters, and the top-level RtpParameters struct.

/// W3C RTCRtpCodecParameters
#[derive(Debug, Clone)]
pub struct RtpCodecParameters {
    pub mime_type: String, // "video/H264", "video/VP8", etc.
    pub payload_type: u8,
    pub clock_rate: u32,
    pub channels: Option<u16>, // for audio
    pub sdp_fmtp_line: Option<String>,
}

/// W3C RTCRtpEncodingParameters
#[derive(Debug, Clone)]
pub struct RtpEncodingParameters {
    pub ssrc: Option<u64>,
    pub active: bool,
    pub max_bitrate: Option<u64>,
    pub max_framerate: Option<f64>,
    pub scale_resolution_down_by: Option<f64>,
    pub rid: Option<String>,
}

impl Default for RtpEncodingParameters {
    fn default() -> Self {
        Self {
            ssrc: None,
            active: true,
            max_bitrate: None,
            max_framerate: None,
            scale_resolution_down_by: None,
            rid: None,
        }
    }
}

/// W3C RTCRtpHeaderExtensionParameters
#[derive(Debug, Clone)]
pub struct RtpHeaderExtensionParameters {
    pub uri: String,
    pub id: u16,
    pub encrypted: bool,
}

/// W3C RTCRtcpParameters
#[derive(Debug, Clone, Default)]
pub struct RtcpParameters {
    pub cname: Option<String>,
    /// When true, indicates reduced-size RTCP.
    #[allow(dead_code)]
    pub reduced_size: bool,
}

/// W3C RTCRtpParameters
#[derive(Debug, Clone, Default)]
pub struct RtpParameters {
    pub transaction_id: String,
    pub codecs: Vec<RtpCodecParameters>,
    pub encodings: Vec<RtpEncodingParameters>,
    pub header_extensions: Vec<RtpHeaderExtensionParameters>,
    pub rtcp: RtcpParameters,
}
