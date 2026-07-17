//! DataChannel thin wrapper.
//!
//! Follows webrtc-kit pattern: holds a handle to the webrtc-sys
//! DataChannel C++ object. Provides send/receive + callbacks.

use std::fmt;

/// DataChannel configuration (W3C RTCDataChannelInit).
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
        Self {
            ordered: true,
            max_retransmit_time: None,
            max_retransmits: None,
            protocol: String::new(),
            negotiated: false,
            id: -1,
        }
    }
}

/// DataChannel state (W3C RTCDataChannelState).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataChannelState {
    Connecting,
    Open,
    Closing,
    Closed,
}

/// Received data buffer.
pub struct DataBuffer<'a> {
    pub data: &'a [u8],
    pub binary: bool,
}

// Callback types.
pub type OnDataChannelMessage = Box<dyn FnMut(Vec<u8>) + Send + Sync>;
pub type OnDataChannelStateChange = Box<dyn FnMut(DataChannelState) + Send + Sync>;

/// W3C RTCDataChannel — thin wrapper around webrtc-sys.
///
/// ponytail: hold the webrtc-sys handle directly; add FFI when libwebrtc is linked.
#[derive(Clone)]
pub struct DataChannel {
    pub(crate) label: String,
    pub(crate) id: i32,
}

impl DataChannel {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn state(&self) -> DataChannelState {
        // ponytail: stub — real implementation queries webrtc-sys handle
        DataChannelState::Closed
    }

    pub fn send(&self, data: &[u8], binary: bool) -> Result<(), crate::RtcError> {
        tracing::trace!(len = data.len(), binary, "DataChannel::send");
        // ponytail: stub — real implementation calls webrtc-sys send
        Ok(())
    }

    pub fn send_text(&self, text: &str) -> Result<(), crate::RtcError> {
        self.send(text.as_bytes(), false)
    }

    pub fn close(&self) {
        tracing::trace!("DataChannel::close");
    }

    pub fn on_message(&self, _cb: Option<OnDataChannelMessage>) {
        // ponytail: stub — real implementation registers webrtc-sys callback
    }

    pub fn on_state_change(&self, _cb: Option<OnDataChannelStateChange>) {
        // ponytail: stub — real implementation registers webrtc-sys callback
    }
}

impl fmt::Debug for DataChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataChannel")
            .field("id", &self.id)
            .field("label", &self.label)
            .finish()
    }
}
