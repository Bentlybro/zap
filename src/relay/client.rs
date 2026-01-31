use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::protocol::{hash_code, RelayMessage, Role};

/// Relay client connection
pub struct RelayConnection {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl RelayConnection {
    /// Connect to a relay server and register
    pub async fn connect(relay_addr: &str, code: &str, role: Role) -> Result<Self> {
        // Ensure the address has ws:// prefix
        let url = if relay_addr.starts_with("ws://") || relay_addr.starts_with("wss://") {
            relay_addr.to_string()
        } else {
            format!("ws://{}", relay_addr)
        };
        
        println!("Connecting to relay: {}", url);
        
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| anyhow!("Failed to connect to relay: {}", e))?;
        
        let mut conn = Self { ws: ws_stream };
        
        // Send registration message
        let code_hash = hash_code(code);
        let register_msg = RelayMessage::Register {
            role,
            code_hash,
        };
        
        conn.send_message(&register_msg).await?;
        
        // Wait for matched response
        loop {
            if let Some(msg) = conn.ws.next().await {
                match msg? {
                    Message::Text(text) => {
                        match RelayMessage::from_json(&text) {
                            Ok(RelayMessage::Matched) => {
                                println!("✓ Matched with peer via relay");
                                return Ok(conn);
                            }
                            Ok(RelayMessage::Error { message }) => {
                                return Err(anyhow!("Relay error: {}", message));
                            }
                            _ => {
                                // Ignore other messages during handshake
                            }
                        }
                    }
                    _ => {}
                }
            } else {
                return Err(anyhow!("Relay connection closed during handshake"));
            }
        }
    }
    
    /// Send a relay protocol message (JSON)
    async fn send_message(&mut self, msg: &RelayMessage) -> Result<()> {
        let json = msg.to_json()?;
        self.ws.send(Message::Text(json)).await?;
        Ok(())
    }
    
    /// Send binary data through relay
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.ws.send(Message::Binary(data.to_vec())).await?;
        Ok(())
    }
    
    /// Receive binary data from relay
    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        loop {
            if let Some(msg) = self.ws.next().await {
                match msg? {
                    Message::Binary(data) => {
                        return Ok(data);
                    }
                    Message::Text(text) => {
                        // Handle control messages
                        if let Ok(relay_msg) = RelayMessage::from_json(&text) {
                            match relay_msg {
                                RelayMessage::Error { message } => {
                                    return Err(anyhow!("Relay error: {}", message));
                                }
                                RelayMessage::Ping => {
                                    self.send_message(&RelayMessage::Pong).await?;
                                }
                                _ => {
                                    // Ignore other control messages
                                }
                            }
                        }
                    }
                    Message::Close(_) => {
                        return Err(anyhow!("Relay connection closed"));
                    }
                    _ => {}
                }
            } else {
                return Err(anyhow!("Relay connection closed"));
            }
        }
    }
    
    /// Close the connection
    pub async fn close(mut self) -> Result<()> {
        self.ws.close(None).await?;
        Ok(())
    }
}
