//! PSK (Pre-Shared Key) HMAC-SHA256 authentication.
//!
//! Phase 1 MVP: Simple PSK handshake. Client sends HMAC(challenge, psk),
//! Server verifies. Used for WebSocket signaling auth.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::error::CoreError;

type HmacSha256 = Hmac<Sha256>;

/// Authentication result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthResult {
    /// Authentication succeeded.
    Success,
    /// Authentication failed — invalid PSK.
    Denied,
    /// Challenge expired.
    Expired,
}

/// PSK authenticator trait.
///
/// Components implement this trait with their PSK sourcing strategy.
#[async_trait::async_trait]
pub trait PskAuthenticator: Send + Sync {
    /// Verify a signed challenge.
    async fn verify_challenge(&self, challenge: &[u8], signature: &[u8])
        -> Result<AuthResult, CoreError>;
}

/// Simple PSK authenticator that holds the key in memory.
///
/// # Security
/// Phase 1: key from env var or config file. Not meant for multi-tenant production.
/// Phase 2+: integrate with vault / LDAP.
pub struct SimplePskAuth {
    psk: Vec<u8>,
}

impl SimplePskAuth {
    /// Create from a PSK string (base64-encoded or raw).
    pub fn new(psk: impl AsRef<[u8]>) -> Self {
        Self {
            psk: psk.as_ref().to_vec(),
        }
    }

    /// Compute HMAC-SHA256(challenge, psk) → truncated to 8 bytes.
    pub fn sign(&self, challenge: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.psk)
            .expect("HMAC key can be any length");
        mac.update(challenge);
        // ponytail: 8-byte tag is enough for Phase 1 challenge-response; full 32 bytes if collision rate rises
        mac.finalize().into_bytes()[..8].to_vec()
    }
}

#[async_trait::async_trait]
impl PskAuthenticator for SimplePskAuth {
    async fn verify_challenge(
        &self,
        challenge: &[u8],
        signature: &[u8],
    ) -> Result<AuthResult, CoreError> {
        let expected = self.sign(challenge);
        if constant_time_eq(&expected, signature) {
            Ok(AuthResult::Success)
        } else {
            Ok(AuthResult::Denied)
        }
    }
}

/// Constant-time comparison to prevent timing side-channels.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn psk_auth_success() {
        let auth = SimplePskAuth::new("test-secret-key");
        let challenge = b"server-challenge-123";
        let sig = auth.sign(challenge);
        let result = auth.verify_challenge(challenge, &sig).await.unwrap();
        assert_eq!(result, AuthResult::Success);
    }

    #[tokio::test]
    async fn psk_auth_denied_wrong_key() {
        let auth = SimplePskAuth::new("right-key");
        let other = SimplePskAuth::new("wrong-key");
        let challenge = b"challenge";
        let sig = other.sign(challenge); // signed with wrong key
        let result = auth.verify_challenge(challenge, &sig).await.unwrap();
        assert_eq!(result, AuthResult::Denied);
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
