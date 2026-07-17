use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Remote configuration (mirrors Host schema, focused on receive side)
#[derive(Debug, Deserialize, Clone)]
pub struct RemoteConfig {
    pub remote: RemoteSection,
    pub signaling: SignalingSection,
    pub media: RemoteMediaSection,
    #[serde(default)]
    pub turn: Option<TurnSection>,
    #[serde(default)]
    pub control: Option<ControlSection>,
    #[serde(default)]
    pub web: Option<WebSection>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RemoteSection {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SignalingSection {
    pub ws_url: String,
    #[serde(default = "default_psk")]
    pub psk: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RemoteMediaSection {
    /// Output display (window title or Wayland/X11 surface)
    #[serde(default)]
    pub display: Option<String>,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub fullscreen: bool,
    /// Preferred decoder (e.g., "nvh264dec", "vaapih264dec")
    #[serde(default)]
    pub decoder: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TurnSection {
    pub urls: String,
    pub username: String,
    pub credential: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ControlSection {
    /// HMAC signing key override (default: "omspbase-control")
    #[serde(default = "default_hmac_key")]
    pub hmac_key: String,
    /// Max control send rate in Hz
    #[serde(default = "default_rate_hz")]
    pub rate_hz: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebSection {
    pub bind: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

// --- defaults ---

fn default_psk() -> String {
    "omspbase-dev".to_string()
}
fn default_width() -> u32 {
    1280
}
fn default_height() -> u32 {
    720
}
fn default_format() -> String {
    "BGRA".to_string()
}
fn default_hmac_key() -> String {
    "omspbase-control".to_string()
}
fn default_rate_hz() -> u32 {
    30
}

/// Load configuration from YAML file
pub fn load<P: AsRef<Path>>(path: P) -> Result<RemoteConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: RemoteConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_remote_config() {
        let yaml = r#"
remote:
  id: "remote-001"
signaling:
  ws_url: "ws://server.local:9100/ws"
media: {}
"#;
        let config: RemoteConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.remote.id, "remote-001");
        assert_eq!(config.remote.name, "");
        assert_eq!(config.signaling.ws_url, "ws://server.local:9100/ws");
        assert_eq!(config.signaling.psk, "omspbase-dev");
        assert_eq!(config.media.width, 1280);
        assert_eq!(config.media.height, 720);
        assert_eq!(config.media.format, "BGRA");
        assert!(!config.media.fullscreen);
        assert!(config.control.is_none());
    }

    #[test]
    fn parse_full_remote_config() {
        let yaml = r#"
remote:
  id: "remote-002"
  name: "Operator Station"
signaling:
  ws_url: "wss://server.example.com/ws"
  psk: "secure-key-456"
media:
  display: "HDMI-1"
  width: 1920
  height: 1080
  format: "I420"
  fullscreen: true
  decoder: "nvv4l2decoder"
turn:
  urls: "turn:relay.example.com:3478"
  username: "remote-user"
  credential: "secure-turn"
control:
  hmac_key: "custom-control-key"
  rate_hz: 60
web:
  bind: "0.0.0.0:9101"
  username: "admin"
  password: "secret"
"#;
        let config: RemoteConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.remote.name, "Operator Station");
        assert_eq!(config.signaling.psk, "secure-key-456");
        assert_eq!(config.media.fullscreen, true);
        let ctrl = config.control.unwrap();
        assert_eq!(ctrl.rate_hz, 60);
    }
}
