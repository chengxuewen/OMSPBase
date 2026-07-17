use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Host configuration (D114 schema)
#[derive(Debug, Deserialize, Clone)]
pub struct HostConfig {
    pub host: HostSection,
    pub signaling: SignalingSection,
    pub media: MediaSection,
    #[serde(default)]
    pub turn: Option<TurnSection>,
    #[serde(default)]
    pub web: Option<WebSection>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HostSection {
    pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SignalingSection {
    pub ws_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MediaSection {
    pub camera: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub encoder: String,
    #[serde(default = "default_format")]
    pub format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TurnSection {
    pub urls: String,
    pub username: String,
    pub credential: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebSection {
    pub bind: String,
    pub username: String,
    pub password: String,
}

fn default_format() -> String {
    "I420".to_string()
}

/// Load configuration from YAML file
pub fn load<P: AsRef<Path>>(path: P) -> Result<HostConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: HostConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let yaml = r#"
host:
  id: "test-001"
signaling:
  ws_url: "ws://localhost:8080/ws"
media:
  camera: "/dev/video0"
  width: 1280
  height: 720
  fps: 30
  bitrate_kbps: 2000
  encoder: "nvh264enc"
"#;
        let config: HostConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.host.id, "test-001");
        assert_eq!(config.signaling.ws_url, "ws://localhost:8080/ws");
        assert_eq!(config.media.camera, "/dev/video0");
        assert_eq!(config.media.width, 1280);
        assert_eq!(config.media.format, "I420"); // default
        assert!(config.turn.is_none());
        assert!(config.web.is_none());
    }

    #[test]
    fn parse_full_config() {
        let yaml = r#"
host:
  id: "test-002"
signaling:
  ws_url: "wss://server.example.com/ws"
media:
  camera: "/dev/video1"
  width: 1920
  height: 1080
  fps: 15
  bitrate_kbps: 4000
  encoder: "vaapih264enc"
  format: "NV12"
turn:
  urls: "turn:relay.example.com:3478"
  username: "testuser"
  credential: "testpass"
web:
  bind: "0.0.0.0:9800"
  username: "admin"
  password: "secure123"
"#;
        let config: HostConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.host.id, "test-002");
        assert_eq!(config.media.format, "NV12");
        let turn = config.turn.unwrap();
        assert_eq!(turn.urls, "turn:relay.example.com:3478");
        let web = config.web.unwrap();
        assert_eq!(web.bind, "0.0.0.0:9800");
    }
}
