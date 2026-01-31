use anyhow::Result;
use std::net::SocketAddr;

use crate::network::Connection;
use crate::relay::{RelayConnection, Role};

/// Transport abstraction that works with both direct TCP and relay
pub enum Transport {
    Direct(Connection),
    Relay(RelayConnection),
}

impl Transport {
    /// Create a transport for sending (either listen on TCP or connect to relay)
    pub async fn new_sender(relay_addr: Option<String>, code: &str, port: Option<u16>) -> Result<Self> {
        if let Some(relay) = relay_addr {
            let relay_conn = RelayConnection::connect(&relay, code, Role::Sender).await?;
            Ok(Transport::Relay(relay_conn))
        } else {
            let conn = crate::network::listen(port).await?;
            Ok(Transport::Direct(conn))
        }
    }
    
    /// Create a transport for receiving (either connect to TCP or connect to relay)
    pub async fn new_receiver(
        relay_addr: Option<String>,
        code: &str,
        host: Option<&str>,
        port: Option<u16>,
    ) -> Result<Self> {
        if let Some(relay) = relay_addr {
            let relay_conn = RelayConnection::connect(&relay, code, Role::Receiver).await?;
            Ok(Transport::Relay(relay_conn))
        } else {
            let host = host.ok_or_else(|| anyhow::anyhow!("Host required for direct connection"))?;
            let conn = crate::network::connect(host, port).await?;
            Ok(Transport::Direct(conn))
        }
    }
    
    /// Send data
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        match self {
            Transport::Direct(conn) => conn.send(data).await,
            Transport::Relay(conn) => conn.send(data).await,
        }
    }
    
    /// Receive data
    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        match self {
            Transport::Direct(conn) => conn.receive().await,
            Transport::Relay(conn) => conn.receive().await,
        }
    }
    
    /// Get peer address (only available for direct connections)
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        match self {
            Transport::Direct(conn) => Some(conn.peer_addr()),
            Transport::Relay(_) => None,
        }
    }
}
