use anyhow::{anyhow, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const DEFAULT_PORT: u16 = 9999;
const MESSAGE_SIZE_BYTES: usize = 4;

/// Network connection wrapper
pub struct Connection {
    stream: TcpStream,
    peer_addr: SocketAddr,
}

impl Connection {
    /// Create a new connection from a TCP stream
    pub fn new(stream: TcpStream, peer_addr: SocketAddr) -> Self {
        Self { stream, peer_addr }
    }
    
    /// Get the peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
    
    /// Send a message (length-prefixed)
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        let len = data.len() as u32;
        self.stream.write_all(&len.to_be_bytes()).await?;
        self.stream.write_all(data).await?;
        self.stream.flush().await?;
        Ok(())
    }
    
    /// Receive a message (length-prefixed)
    pub async fn receive(&mut self) -> Result<Vec<u8>> {
        let mut len_bytes = [0u8; MESSAGE_SIZE_BYTES];
        self.stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        if len > 100 * 1024 * 1024 {
            return Err(anyhow!("Message too large: {} bytes", len));
        }
        
        let mut buffer = vec![0u8; len];
        self.stream.read_exact(&mut buffer).await?;
        Ok(buffer)
    }
    
    /// Send raw bytes (for file chunks)
    pub async fn send_raw(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).await?;
        Ok(())
    }
    
    /// Receive raw bytes (for file chunks)
    pub async fn receive_raw(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        self.stream.read_exact(&mut buffer).await?;
        Ok(buffer)
    }
}

/// Start a TCP server and wait for a connection
pub async fn listen(port: Option<u16>) -> Result<Connection> {
    let port = port.unwrap_or(DEFAULT_PORT);
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    
    println!("Listening on {}", addr);
    
    let (stream, peer_addr) = listener.accept().await?;
    Ok(Connection::new(stream, peer_addr))
}

/// Connect to a remote host
pub async fn connect(host: &str, port: Option<u16>) -> Result<Connection> {
    let port = port.unwrap_or(DEFAULT_PORT);
    let addr = format!("{}:{}", host, port);
    
    let stream = TcpStream::connect(&addr).await?;
    let peer_addr = stream.peer_addr()?;
    
    Ok(Connection::new(stream, peer_addr))
}

/// Discover peers on the local network using mDNS (simplified for MVP)
pub async fn discover_mdns(_code: &str) -> Result<Option<SocketAddr>> {
    // For MVP, we'll skip mDNS and require manual connection
    // In a full implementation, we'd use mdns-sd to advertise and discover
    Ok(None)
}

/// Advertise this service on mDNS (simplified for MVP)
pub async fn advertise_mdns(_code: &str, _port: u16) -> Result<()> {
    // For MVP, we'll skip mDNS advertisement
    // In a full implementation, we'd use mdns-sd to advertise the service
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection() {
        let server_handle = tokio::spawn(async {
            let mut conn = listen(Some(19999)).await.unwrap();
            let data = conn.receive().await.unwrap();
            assert_eq!(data, b"test");
            conn.send(b"response").await.unwrap();
        });
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let mut conn = connect("127.0.0.1", Some(19999)).await.unwrap();
        conn.send(b"test").await.unwrap();
        let response = conn.receive().await.unwrap();
        assert_eq!(response, b"response");
        
        server_handle.await.unwrap();
    }
}
