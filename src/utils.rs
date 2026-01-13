use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;

/// Generate random WebSocket key (16 random bytes base64 encoded)
pub fn generate_websocket_key() -> String {
    let mut random_bytes = [0u8; 16];
    rand::thread_rng().fill(&mut random_bytes);
    STANDARD.encode(random_bytes)
}

/// Base64 encode data
pub fn base64_encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// Base64 decode data
pub fn base64_decode(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
    STANDARD.decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_key_generation() {
        let key1 = generate_websocket_key();
        let key2 = generate_websocket_key();

        // Keys should be different
        assert_ne!(key1, key2);

        // Keys should be valid base64 (24 chars for 16 bytes)
        assert_eq!(key1.len(), 24);
        assert_eq!(key2.len(), 24);
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"Hello, World!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();

        assert_eq!(original.to_vec(), decoded);
    }
}
