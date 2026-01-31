use serde::{Deserialize, Serialize};

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Message types exchanged during transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Initial handshake with protocol version
    Hello { version: u8 },
    
    /// SPAKE2 key exchange message
    KeyExchange { data: Vec<u8> },
    
    /// Transfer metadata (encrypted)
    Metadata {
        filename: String,
        size: u64,
        is_directory: bool,
        checksum: String,
    },
    
    /// File chunk (encrypted)
    Chunk {
        index: u64,
        data: Vec<u8>,
    },
    
    /// Request to resume from specific chunk
    Resume { from_chunk: u64 },
    
    /// Transfer complete
    Complete,
    
    /// Error message
    Error { message: String },
    
    /// Acknowledgment
    Ack,
}

impl Message {
    /// Serialize message to bytes
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
    
    /// Deserialize message from bytes
    pub fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(data)?)
    }
}

/// Transfer state for resumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferState {
    pub filename: String,
    pub total_size: u64,
    pub chunks_received: Vec<u64>,
    pub checksum: String,
}
