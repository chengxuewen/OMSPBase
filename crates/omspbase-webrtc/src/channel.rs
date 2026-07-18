//! DataChannel thin wrapper.
//!
//! With `webrtc-backend` feature: wraps webrtc-rs RTCDataChannel.
//! Without: stub (no-op send, Closed state).

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
    #[cfg(feature = "webrtc-backend")]
    inner: Option<std::sync::Arc<webrtc::data_channel::RTCDataChannel>>,
}

// ── Stub ──
#[cfg(not(feature = "webrtc-backend"))]
impl DataChannel {
    pub fn label(&self) -> &str { &self.label }
    pub fn id(&self) -> i32 { self.id }
    pub fn state(&self) -> DataChannelState { DataChannelState::Closed }
    pub async fn send(&self, _: &[u8]) -> Result<(), RtcError> { Ok(()) }
    pub async fn send_text(&self, t: &str) -> Result<(), RtcError> { self.send(t.as_bytes()).await }
    pub async fn close(&mut self) {}
    pub async fn spool(&self) -> DataChannelRx { DataChannelRx::stub() }
}

// ── webrtc-rs backend ──
#[cfg(feature = "webrtc-backend")]
impl DataChannel {
    pub async fn from_webrtc(dc: std::sync::Arc<webrtc::data_channel::RTCDataChannel>) -> Self {
        let id = dc.id() as i32;
        let label = dc.label().to_string();
        Self { label, id, inner: Some(dc) }
    }

    fn inner(&self) -> &std::sync::Arc<webrtc::data_channel::RTCDataChannel> {
        self.inner.as_ref().expect("DataChannel already closed")
    }

    pub fn label(&self) -> &str { &self.label }
    pub fn id(&self) -> i32 { self.id }

    pub fn state(&self) -> DataChannelState {
        use webrtc::data_channel::data_channel_state::RTCDataChannelState::*;
        match self.inner().ready_state() {
            Connecting => DataChannelState::Connecting, Open => DataChannelState::Open,
            Closing => DataChannelState::Closing, Closed => DataChannelState::Closed,
            _ => DataChannelState::Closed,
        }
    }

    pub async fn send(&self, data: &[u8]) -> Result<(), RtcError> {
        let b = bytes::Bytes::copy_from_slice(data);
        self.inner().send(&b).await.map(|_| ()).map_err(|e| RtcError::DataChannel(e.to_string()))
    }

    pub async fn send_text(&self, text: &str) -> Result<(), RtcError> {
        self.inner().send_text(text).await.map(|_| ()).map_err(|e| RtcError::DataChannel(e.to_string()))
    }

    pub async fn spool(&self) -> DataChannelRx {
        let dc = self.inner().clone();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let tx2 = tx.clone();
        dc.on_open(Box::new(move || { let _ = tx2.send(DataChannelEvent::Open); Box::pin(async {}) }));
        let tx2 = tx.clone();
        dc.on_close(Box::new(move || { let _ = tx2.send(DataChannelEvent::Closed); Box::pin(async {}) }));
        let tx2 = tx.clone();
        dc.on_message(Box::new(move |msg| {
            let data = msg.data.to_vec();
            let _ = tx2.send(DataChannelEvent::Message(DataMessage { data }));
            Box::pin(async {})
        }));
        dc.on_error(Box::new(move |err| { let _ = tx.send(DataChannelEvent::Error(err.to_string())); Box::pin(async {}) }));
        DataChannelRx { inner: Some(rx) }
    }

    pub async fn close(&mut self) {
        if let Some(dc) = self.inner.take() {
            dc.close().await.ok();
        }
    }
}

impl Clone for DataChannel {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(), id: self.id,
            #[cfg(feature = "webrtc-backend")]
            inner: self.inner.clone(),
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

pub struct DataChannelRx {
    #[cfg(feature = "webrtc-backend")]
    inner: Option<tokio::sync::mpsc::UnboundedReceiver<DataChannelEvent>>,
}

impl DataChannelRx {
    #[cfg(not(feature = "webrtc-backend"))]
    pub(crate) fn stub() -> Self { Self {} }
    pub async fn recv(&mut self) -> Option<DataChannelEvent> {
        #[cfg(feature = "webrtc-backend")]
        {
            match &mut self.inner {
                Some(rx) => rx.recv().await,
                None => None,
            }
        }
        #[cfg(not(feature = "webrtc-backend"))]
        { std::future::pending().await }
    }
}
