use rand::Rng;
use std::io;

const PING_MESSAGE: &[u8] = b"PING";

/// WebSocket frame structure
#[derive(Debug, Clone)]
pub struct WebSocketFrame {
    pub fin: bool,
    pub rsv1: bool,
    pub rsv2: bool,
    pub rsv3: bool,
    pub opcode: u8,
    pub masked: bool,
    pub payload_len: u64,
    pub masking_key: Option<[u8; 4]>,
    pub payload: Vec<u8>,
}

impl WebSocketFrame {
    /// Create a text frame with the given payload
    pub fn create_text_frame(payload: &[u8]) -> Vec<u8> {
        Self::create_frame(0x81, payload) // FIN + Text frame
    }

    /// Create a WebSocket frame with the given opcode and payload
    pub fn create_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        let len = payload.len();

        frame.push(opcode);

        // Add length and masking
        if len < 126 {
            frame.push(0x80 | len as u8); // Masked + length
        } else if len < 65536 {
            frame.push(0x80 | 126);
            frame.push((len >> 8) as u8);
            frame.push((len & 0xFF) as u8);
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }

        // Generate and add masking key
        let mut mask = [0u8; 4];
        rand::thread_rng().fill(&mut mask);
        frame.extend_from_slice(&mask);

        // Add masked payload
        let masked_payload: Vec<u8> = payload
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ mask[i % 4])
            .collect();
        frame.extend(masked_payload);

        frame
    }

    /// Parse a WebSocket frame from bytes
    pub fn parse_frame(data: &[u8]) -> Result<Self, io::Error> {
        if data.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Frame too short",
            ));
        }

        let byte1 = data[0];
        let byte2 = data[1];

        let fin = (byte1 & 0x80) != 0;
        let rsv1 = (byte1 & 0x40) != 0;
        let rsv2 = (byte1 & 0x20) != 0;
        let rsv3 = (byte1 & 0x10) != 0;
        let opcode = byte1 & 0x0F;

        let masked = (byte2 & 0x80) != 0;
        let payload_len = (byte2 & 0x7F) as u64;

        let (payload_len, offset) = if payload_len == 126 {
            if data.len() < 4 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Extended length incomplete",
                ));
            }
            let len = u16::from_be_bytes([data[2], data[3]]) as u64;
            (len, 4)
        } else if payload_len == 127 {
            if data.len() < 10 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Extended length incomplete",
                ));
            }
            let len = u64::from_be_bytes([
                data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
            ]);
            (len, 10)
        } else {
            (payload_len, 2)
        };

        let mut offset = offset;
        let masking_key = if masked {
            if data.len() < offset + 4 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Masking key incomplete",
                ));
            }
            let mut key = [0u8; 4];
            key.copy_from_slice(&data[offset..offset + 4]);
            offset += 4;
            Some(key)
        } else {
            None
        };

        if data.len() < offset + payload_len as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Payload incomplete",
            ));
        }

        let mut payload = vec![0u8; payload_len as usize];
        payload.copy_from_slice(&data[offset..offset + payload_len as usize]);

        // Unmask if needed
        if let Some(mask) = masking_key {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[i % 4];
            }
        }

        Ok(WebSocketFrame {
            fin,
            rsv1,
            rsv2,
            rsv3,
            opcode,
            masked,
            payload_len,
            masking_key,
            payload,
        })
    }
}

/// Helper to create PING message as WebSocket frame
pub fn create_ping_frame() -> Vec<u8> {
    WebSocketFrame::create_text_frame(PING_MESSAGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_text_frame() {
        let frame = WebSocketFrame::create_text_frame(b"Hello");

        // Should have at least 2 bytes (header) + 4 (mask) + 5 (payload)
        assert!(frame.len() >= 11);

        // First byte should be 0x81 (FIN + Text frame)
        assert_eq!(frame[0], 0x81);

        // Second byte should have mask bit set
        assert!(frame[1] & 0x80 != 0);
    }

    #[test]
    fn test_ping_frame() {
        let frame = create_ping_frame();

        // Should create a valid frame
        assert!(!frame.is_empty());
        assert_eq!(frame[0], 0x81);
    }

    #[test]
    fn test_parse_frame() {
        let original = b"Test message";
        let frame_data = WebSocketFrame::create_text_frame(original);

        let frame = WebSocketFrame::parse_frame(&frame_data).unwrap();

        assert_eq!(frame.fin, true);
        assert_eq!(frame.opcode, 1); // Text frame
        assert_eq!(frame.payload, original.to_vec());
    }

    #[test]
    fn test_large_frame() {
        let large_payload = vec![0x42u8; 1000];
        let frame_data = WebSocketFrame::create_text_frame(&large_payload);

        let frame = WebSocketFrame::parse_frame(&frame_data).unwrap();

        assert_eq!(frame.payload.len(), 1000);
        assert_eq!(frame.payload, large_payload);
    }
}
