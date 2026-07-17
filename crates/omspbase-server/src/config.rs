use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Server configuration (matches config/server.conf template)
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub server: ServerSection,
    #[serde(default)]
    pub web: Option<WebSection>,
    #[serde(default)]
    pub relay: Option<RelaySection>,
    #[serde(default)]
    pub turn: Option<TurnSection>,
    #[serde(default)]
    pub signaling: Option<SignalingSection>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSection {
    pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebSection {
    pub bind: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RelaySection {
    #[serde(default = "default_max_rooms")]
    pub max_rooms: usize,
    #[serde(default = "default_max_remotes_per_room")]
    pub max_remotes_per_room: usize,
    #[serde(default = "default_peer_timeout_secs")]
    pub peer_timeout_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TurnSection {
    pub urls: String,
    pub username: String,
    pub credential: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SignalingSection {
    pub psk: String,
}

fn default_max_rooms() -> usize {
    100
}

fn default_max_remotes_per_room() -> usize {
    50
}

fn default_peer_timeout_secs() -> u64 {
    60
}

/// Load configuration from YAML file
pub fn load<P: AsRef<Path>>(path: P) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: ServerConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let yaml = r#"
server:
  id: "server-001"
web:
  bind: "0.0.0.0:9801"
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.server.id, "server-001");
        assert_eq!(config.web.as_ref().unwrap().bind, "0.0.0.0:9801");
        assert!(config.relay.is_none());
        assert!(config.turn.is_none());
        assert!(config.signaling.is_none());
    }

    #[test]
    fn parse_full_config() {
        let yaml = r#"
server:
  id: "prod-server-001"
web:
  bind: "0.0.0.0:9801"
relay:
  max_rooms: 200
  max_remotes_per_room: 100
  peer_timeout_secs: 120
turn:
  urls: "turn:relay.example.com:3478"
  username: "turnuser"
  credential: "turnpass"
signaling:
  psk: "production-secret"
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.server.id, "prod-server-001");

        let relay = config.relay.unwrap();
        assert_eq!(relay.max_rooms, 200);
        assert_eq!(relay.max_remotes_per_room, 100);
        assert_eq!(relay.peer_timeout_secs, 120);

        let turn = config.turn.unwrap();
        assert_eq!(turn.urls, "turn:relay.example.com:3478");

        let signaling = config.signaling.unwrap();
        assert_eq!(signaling.psk, "production-secret");
    }

    #[test]
    fn default_relay_values() {
        let yaml = r#"
server:
  id: "test"
relay: {}
"#;
        let config: ServerConfig = serde_yaml::from_str(yaml).unwrap();
        let relay = config.relay.unwrap();
        assert_eq!(relay.max_rooms, 100);
        assert_eq!(relay.max_remotes_per_room, 50);
        assert_eq!(relay.peer_timeout_secs, 60);
    }
}
