// DataChannel control sender — constructs frames, signs with HMAC, queues for send
// Mirrors Host's control.rs receiver side with ControlFrame enum reused exactly

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::VecDeque;

type HmacSha256 = Hmac<Sha256>;

/// Control frame types sent over DataChannel
/// Identical to Host's ControlFrame — this is the sender side
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlFrame {
    #[serde(rename = "steering")]
    Steering { value: f64 },
    #[serde(rename = "brake")]
    Brake { value: f64 },
    #[serde(rename = "throttle")]
    Throttle { value: f64 },
}

/// Derive HMAC key from label
fn derive_key(label: &[u8]) -> HmacSha256 {
    // ponytail: simple label-as-key derivation, use HKDF if key rotation needed
    HmacSha256::new_from_slice(label).expect("HMAC key derivation should not fail")
}

/// Compute 8-byte HMAC-SHA256 tag for given payload
pub fn compute_hmac_tag(payload: &[u8], key_label: &[u8]) -> [u8; 8] {
    let mut mac = derive_key(key_label);
    mac.update(payload);
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    let mut tag = [0u8; 8];
    tag.copy_from_slice(&code_bytes[..8]);
    tag
}

/// A signed control message ready for DataChannel send
#[derive(Debug, Clone)]
pub struct SignedFrame {
    /// JSON-encoded control frame body
    pub body: String,
    /// 8-byte HMAC tag for the body
    pub tag: [u8; 8],
}

/// Control sender — serializes frames, signs with HMAC, manages send buffer
pub struct ControlSender {
    /// HMAC key label (configurable per remote)
    key_label: Vec<u8>,
    /// Pending signed frames awaiting DataChannel transmit
    buffer: VecDeque<SignedFrame>,
    /// Maximum buffer depth before dropping oldest
    max_depth: usize,
    /// Send rate limiter (Hz)
    rate_hz: u32,
}

impl ControlSender {
    /// Create a new control sender with given HMAC key label and rate limit
    pub fn new(key_label: &str, rate_hz: u32) -> Self {
        tracing::info!(
            "Control sender initialized (HMAC label: {key_label}, rate: {rate_hz} Hz, buffer depth 3)",
            key_label = key_label,
        );
        ControlSender {
            key_label: key_label.as_bytes().to_vec(),
            buffer: VecDeque::with_capacity(4),
            max_depth: 3,
            rate_hz,
        }
    }

    /// Sign a control frame: serialize to JSON, compute HMAC tag
    pub fn sign(&self, frame: &ControlFrame) -> Result<SignedFrame, serde_json::Error> {
        let body = serde_json::to_string(frame)?;
        let tag = compute_hmac_tag(body.as_bytes(), &self.key_label);
        Ok(SignedFrame { body, tag })
    }

    /// Enqueue a frame for sending. Drops oldest if buffer full.
    pub fn enqueue(&mut self, frame: ControlFrame) -> Result<(), serde_json::Error> {
        let signed = self.sign(&frame)?;
        if self.buffer.len() >= self.max_depth {
            self.buffer.pop_front();
            tracing::warn!("Control send buffer full, dropped oldest frame");
        }
        self.buffer.push_back(signed);
        Ok(())
    }

    /// Dequeue next signed frame for DataChannel transmission
    pub fn dequeue(&mut self) -> Option<SignedFrame> {
        self.buffer.pop_front()
    }

    /// Current buffer depth
    pub fn depth(&self) -> usize {
        self.buffer.len()
    }

    /// Rate limit in Hz
    pub fn rate_hz(&self) -> u32 {
        self.rate_hz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_deterministic() {
        let payload = b"{\"type\":\"steering\",\"value\":15.2}";
        let tag1 = compute_hmac_tag(payload, b"omspbase-control");
        let tag2 = compute_hmac_tag(payload, b"omspbase-control");
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn hmac_different_keys_different_tags() {
        let payload = b"{\"type\":\"steering\",\"value\":15.2}";
        let tag1 = compute_hmac_tag(payload, b"key-a");
        let tag2 = compute_hmac_tag(payload, b"key-b");
        assert_ne!(tag1, tag2);
    }

    #[test]
    fn sign_steering_frame() {
        let sender = ControlSender::new("omspbase-control", 30);
        let frame = ControlFrame::Steering { value: 15.2 };
        let signed = sender.sign(&frame).unwrap();
        assert_eq!(signed.body, r#"{"type":"steering","value":15.2}"#);
        assert_eq!(signed.tag.len(), 8);
    }

    #[test]
    fn sign_all_frame_types() {
        let sender = ControlSender::new("test-key", 30);
        for frame in [
            ControlFrame::Steering { value: 1.0 },
            ControlFrame::Brake { value: 0.5 },
            ControlFrame::Throttle { value: 45.0 },
        ] {
            let signed = sender.sign(&frame).unwrap();
            assert!(!signed.body.is_empty());
            assert_eq!(signed.tag.len(), 8);
        }
    }

    #[test]
    fn buffer_drops_oldest() {
        let mut sender = ControlSender::new("test-key", 30);
        sender.enqueue(ControlFrame::Steering { value: 1.0 }).unwrap();
        sender.enqueue(ControlFrame::Brake { value: 0.5 }).unwrap();
        sender.enqueue(ControlFrame::Throttle { value: 10.0 }).unwrap();
        assert_eq!(sender.depth(), 3);

        // This drops the first (steering 1.0)
        sender.enqueue(ControlFrame::Steering { value: 2.0 }).unwrap();
        assert_eq!(sender.depth(), 3);

        // First out should be brake 0.5
        let next = sender.dequeue().unwrap();
        assert!(next.body.contains("brake"));
    }

    #[test]
    fn cross_compatible_with_host_validate() {
        // This test verifies the sender's HMAC can be validated by the Host receiver
        // Re-implement host-side validate inline to avoid crate dependency
        let payload = br#"{"type":"steering","value":15.2}"#;
        let tag = compute_hmac_tag(payload, b"omspbase-control");

        // Host-side validation (same algorithm)
        let mut mac = HmacSha256::new_from_slice(b"omspbase-control").unwrap();
        mac.update(payload);
        let result = mac.finalize();
        let code_bytes = result.into_bytes();
        assert_eq!(&tag[..], &code_bytes[..8]);
    }
}
