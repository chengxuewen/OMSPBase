// Session persistence + crash recovery
// Writes session_state.json every 10s, loads on startup with staleness check

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time;

/// Session state persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub room_id: Option<String>,
    pub peer_fingerprint: Option<String>,
    /// Last known ICE candidates (JSON strings)
    #[serde(default)]
    pub last_ice_candidates: Vec<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState {
            room_id: None,
            peer_fingerprint: None,
            last_ice_candidates: Vec::new(),
        }
    }
}

/// Shared session handle — updated by app, persisted by background task
pub struct Session {
    state: Arc<Mutex<SessionState>>,
    path: PathBuf,
}

impl Session {
    /// Load session from file if stale < 60s, otherwise return fresh
    pub fn load(path: Option<&str>) -> Self {
        let path = PathBuf::from(path.unwrap_or("session_state.json"));

        let state = match std::fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_json::from_str::<SessionState>(&contents) {
                    Ok(s) => {
                        // Check file modification time for staleness
                        if let Ok(meta) = std::fs::metadata(&path) {
                            if let Ok(modified) = meta.modified() {
                                let age = SystemTime::now()
                                    .duration_since(modified)
                                    .unwrap_or(Duration::from_secs(999));
                                if age < Duration::from_secs(60) {
                                    tracing::info!(
                                        "Loaded session: room={:?}, age={}s",
                                        s.room_id,
                                        age.as_secs()
                                    );
                                    s
                                } else {
                                    tracing::info!(
                                        "Session state stale ({}s > 60s), starting fresh",
                                        age.as_secs()
                                    );
                                    SessionState::default()
                                }
                            } else {
                                SessionState::default()
                            }
                        } else {
                            SessionState::default()
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse session state: {}, starting fresh", e);
                        SessionState::default()
                    }
                }
            }
            Err(_) => {
                tracing::info!("No existing session state, starting fresh");
                SessionState::default()
            }
        };

        Session {
            state: Arc::new(Mutex::new(state)),
            path,
        }
    }

    /// Get a mutable reference to update state (caller locks briefly)
    pub async fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut SessionState),
    {
        let mut state = self.state.lock().await;
        f(&mut state);
    }

    /// Start background persistence task — writes every 10 seconds
    pub fn start_persist(&self) -> tokio::task::JoinHandle<()> {
        let state = self.state.clone();
        let path = self.path.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let current = state.lock().await;
                let content = serde_json::to_string_pretty(&*current)
                    .unwrap_or_else(|e| {
                        tracing::error!("Failed to serialize session: {}", e);
                        "{}".to_string()
                    });
                // ponytail: atomic write via temp file + rename
                let tmp_path = path.with_extension("tmp");
                if let Err(e) = std::fs::write(&tmp_path, &content) {
                    tracing::error!("Failed to write session temp file: {}", e);
                    continue;
                }
                if let Err(e) = std::fs::rename(&tmp_path, &path) {
                    tracing::error!("Failed to rename session file: {}", e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn session_serialization() {
        let state = SessionState {
            room_id: Some("room-1".to_string()),
            peer_fingerprint: Some("SHA256:abc123".to_string()),
            last_ice_candidates: vec!["candidate:1".to_string(), "candidate:2".to_string()],
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.room_id.unwrap(), "room-1");
        assert_eq!(parsed.peer_fingerprint.unwrap(), "SHA256:abc123");
        assert_eq!(parsed.last_ice_candidates.len(), 2);
    }

    #[test]
    fn session_default_is_empty() {
        let state = SessionState::default();
        assert!(state.room_id.is_none());
        assert!(state.peer_fingerprint.is_none());
        assert!(state.last_ice_candidates.is_empty());
    }

    #[tokio::test]
    async fn session_update_works() {
        let tmp = NamedTempFile::new().unwrap();
        let session = Session {
            state: Arc::new(Mutex::new(SessionState::default())),
            path: tmp.path().to_path_buf(),
        };

        session.update(|s| {
            s.room_id = Some("room-x".to_string());
            s.peer_fingerprint = Some("fp-x".to_string());
        }).await;

        let state = session.state.lock().await;
        assert_eq!(state.room_id.as_deref(), Some("room-x"));
    }
}
