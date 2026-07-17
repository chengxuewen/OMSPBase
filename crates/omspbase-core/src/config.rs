//! Shared config schemas for all OMSPBase components.
//!
//! Each component reads its YAML config file and deserializes into these types.

use serde::{Deserialize, Serialize};

/// Config for omspbase-host (capture + encode + push).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    /// Config schema version for migration compatibility.
    #[serde(default = "default_version")]
    pub version: u8,

    /// Server address (WebSocket signaling + WebRTC ICE).
    pub server: ServerAddress,

    /// Capture source configuration.
    pub capture: CaptureConfig,

    /// Encoding parameters.
    pub encoder: EncoderConfig,

    /// WebRTC configuration.
    pub webrtc: Option<WebRtcPushConfig>,

    /// PSK for signaling auth.
    pub psk: Option<String>,
}

/// Config for omspbase-server (signaling + relay + monitoring).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Config schema version.
    #[serde(default = "default_version")]
    pub version: u8,

    /// Listen address for HTTP/WS server.
    pub listen: ListenConfig,

    /// Room capacity limit.
    #[serde(default = "default_room_capacity")]
    pub room_capacity: usize,

    /// PSK for signaling auth.
    pub psk: Option<String>,

    /// Rate limit (requests per second per connection).
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
}

/// Config for omspbase-remote (pull + decode + control).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// Config schema version.
    #[serde(default = "default_version")]
    pub version: u8,

    /// Server address.
    pub server: ServerAddress,

    /// PSK for signaling auth.
    pub psk: Option<String>,

    /// Render window configuration (platform-specific, optional).
    pub render: Option<RenderConfig>,
}

// --- Sub-types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAddress {
    /// WebSocket signaling URL (e.g., "ws://192.168.1.1:9800/ws").
    pub signaling_url: String,

    /// ICE server addresses (STUN/TURN URIs, optional).
    #[serde(default)]
    pub ice_servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenConfig {
    /// Bind host.
    #[serde(default = "default_host")]
    pub host: String,

    /// HTTP/WS port.
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    /// Capture source type: "screen", "camera", or "test_pattern".
    pub source: String,

    /// Resolution in format "WIDTHxHEIGHT" (e.g., "1280x720").
    #[serde(default = "default_resolution")]
    pub resolution: String,

    /// Frame rate.
    #[serde(default = "default_framerate")]
    pub framerate: u32,

    /// Device path (e.g., /dev/video0 for V4L2). Optional.
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    /// Encoder backend: "nvenc", "videotoolbox", "vaapi", or "software".
    #[serde(default = "default_encoder")]
    pub backend: String,

    /// Target bitrate in kbps.
    #[serde(default = "default_bitrate")]
    pub bitrate_kbps: u32,

    /// Keyframe interval (GOP size in frames).
    #[serde(default = "default_gop")]
    pub keyframe_interval: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRtcPushConfig {
    /// ICE connection timeout in seconds.
    #[serde(default = "default_ice_timeout")]
    pub ice_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    /// Render backend: "auto" (platform default) or explicit like "gtk", "metal", etc.
    #[serde(default = "default_render_backend")]
    pub backend: String,
}

// --- Defaults ---

fn default_version() -> u8 {
    1
}
fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    9800
}
fn default_resolution() -> String {
    "1280x720".into()
}
fn default_framerate() -> u32 {
    30
}
fn default_encoder() -> String {
    "auto".into()
}
fn default_bitrate() -> u32 {
    2000
}
fn default_gop() -> u32 {
    60
}
fn default_room_capacity() -> usize {
    10
}
fn default_rate_limit() -> u32 {
    100
}
fn default_ice_timeout() -> u64 {
    30
}
fn default_render_backend() -> String {
    "auto".into()
}
