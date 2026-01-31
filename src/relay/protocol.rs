use serde::{Deserialize, Serialize};

/// Relay protocol messages for handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RelayMessage {
    /// Client registers with the relay
    Register {
        role: Role,
        code_hash: String,
    },
    
    /// Relay confirms successful match
    Matched,
    
    /// Error from relay
    Error {
        message: String,
    },
    
    /// Ping/pong for keepalive
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Sender,
    Receiver,
}

impl RelayMessage {
    /// Serialize to JSON string
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
    
    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}

/// Hash a transfer code using BLAKE3
pub fn hash_code(code: &str) -> String {
    let hash = blake3::hash(code.as_bytes());
    hash.to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_code() {
        let code = "alpha-bravo-charlie";
        let hash1 = hash_code(code);
        let hash2 = hash_code(code);
        assert_eq!(hash1, hash2);
        
        let different_hash = hash_code("different-code");
        assert_ne!(hash1, different_hash);
    }
    
    #[test]
    fn test_message_serialization() {
        let msg = RelayMessage::Register {
            role: Role::Sender,
            code_hash: "test123".to_string(),
        };
        
        let json = msg.to_json().unwrap();
        let deserialized = RelayMessage::from_json(&json).unwrap();
        
        match deserialized {
            RelayMessage::Register { role, code_hash } => {
                assert_eq!(role, Role::Sender);
                assert_eq!(code_hash, "test123");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
