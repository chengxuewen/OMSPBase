//! DataChannel thin wrapper.
//!
//! Delegates operations to the backend (DcBackend trait)
//! via compile-time type alias dispatch.

use crate::backend::{ActiveDc, DcBackend};
use crate::RtcError;

#[derive(Debug, Clone)]
pub struct DataChannelInit {
    pub ordered: bool,
    pub max_retransmit_time: Option<i32>,
    pub max_retransmits: Option<i32>,
    pub protocol: String,
    pub negotiated: bool,
    pub id: i32,
}

impl Default for DataChannelInit {
    fn default() -> Self {
        Self { ordered: true, max_retransmit_time: None, max_retransmits: None,
               protocol: String::new(), negotiated: false, id: -1 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataChannelState { Connecting, Open, Closing, Closed }

pub struct DataChannel {
    pub(crate) label: String,
    pub(crate) id: i32,
    pub(crate) backend: ActiveDc,
}

impl DataChannel {
    pub fn label(&self) -> &str { &self.label }
    pub fn id(&self) -> i32 { self.id }

    pub fn state(&self) -> DataChannelState {
        self.backend.state()
    }

    pub async fn send(&self, data: &[u8]) -> Result<(), RtcError> {
        self.backend.send(data).await
    }

    pub async fn send_text(&self, text: &str) -> Result<(), RtcError> {
        self.backend.send_text(text).await
    }

    pub async fn spool(&self) -> DataChannelRx {
        self.backend.spool().await
    }

    pub async fn close(&mut self) {
        self.backend.close().await;
    }
}
#[cfg(feature = "backend-webrtc-rs")]
impl DataChannel {
    pub async fn from_webrtc(dc: std::sync::Arc<webrtc::data_channel::RTCDataChannel>) -> Self {
        let label = dc.label().to_string();
        let id = dc.id() as i32;
        Self { label, id, backend: crate::backend::webrtc_rs::WebrtcRsDc::new(dc) }
    }
}
impl Clone for DataChannel {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            id: self.id,
            backend: self.backend.clone(),
        }
    }
}

impl std::fmt::Debug for DataChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataChannel").field("id", &self.id).field("label", &self.label).finish()
    }
}

// ── Events ──

pub enum DataChannelEvent { Open, Closed, Message(DataMessage), Error(String) }

impl std::fmt::Debug for DataChannelEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "Open"), Self::Closed => write!(f, "Closed"),
            Self::Message(m) => write!(f, "Message({}B)", m.data.len()),
            Self::Error(e) => write!(f, "Error({e})"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataMessage { pub data: Vec<u8> }

/// Receiver for DataChannel events.
/// Created by spool() — polls the backend's event stream.
pub struct DataChannelRx {
    rx: Option<tokio::sync::mpsc::UnboundedReceiver<DataChannelEvent>>,
}

    impl DataChannelRx {
    #[cfg(feature = "backend-webrtc-rs")]
    pub(crate) fn new(rx: Option<tokio::sync::mpsc::UnboundedReceiver<DataChannelEvent>>) -> Self {
        Self { rx }
    }

    pub(crate) fn stub() -> Self {
        Self { rx: None }
    }

    pub async fn recv(&mut self) -> Option<DataChannelEvent> {
        match &mut self.rx {
            Some(rx) => rx.recv().await,
            None => std::future::pending().await,
        }
    }
}

// ── Tests ──

#[cfg(all(test, not(any(feature = "backend-webrtc-rs", feature = "backend-webrtc-sys"))))]
mod tests {
    use super::*;

    #[test]
    fn stub_data_channel_state_is_closed() {
        let dc = DataChannel { label: "test".into(), id: 0, backend: Default::default() };
        assert_eq!(dc.state(), DataChannelState::Closed);
    }

    #[test]
    fn stub_label_and_id() {
        let dc = DataChannel { label: "mylabel".into(), id: 42, backend: Default::default() };
        assert_eq!(dc.label(), "mylabel");
        assert_eq!(dc.id(), 42);
    }

    #[test]
    fn stub_send_is_noop() {
        let dc = DataChannel { label: "x".into(), id: 0, backend: Default::default() };
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert!(dc.send(b"hello").await.is_ok());
            assert!(dc.send_text("world").await.is_ok());
        });
    }

    #[test]
    fn stub_data_channel_init_defaults() {
        let init = DataChannelInit::default();
        assert!(init.ordered);
        assert_eq!(init.max_retransmit_time, None);
        assert_eq!(init.max_retransmits, None);
        assert_eq!(init.protocol, "");
        assert!(!init.negotiated);
        assert_eq!(init.id, -1);
    }
}
