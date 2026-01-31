use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use super::protocol::{RelayMessage, Role};

type Tx = mpsc::UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<String, Peer>>>;

/// Represents a connected peer (sender or receiver)
#[derive(Debug)]
struct Peer {
    role: Role,
    tx: Tx,
    addr: SocketAddr,
}

/// Run the relay server
pub async fn run_relay_server(port: u16) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    
    println!("⚡ Zap Relay Server");
    println!("═══════════════════════════════════════");
    println!("Listening on: {}", addr);
    println!("Relay is blind - all data is encrypted E2E");
    println!();
    
    let peers: PeerMap = Arc::new(Mutex::new(HashMap::new()));
    
    loop {
        let (stream, addr) = listener.accept().await?;
        let peers = peers.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, peers).await {
                eprintln!("Error handling connection from {}: {}", addr, e);
            }
        });
    }
}

async fn handle_connection(stream: TcpStream, addr: SocketAddr, peers: PeerMap) -> Result<()> {
    println!("[{}] New connection", addr);
    
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    // Spawn task to forward messages from channel to websocket
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    
    let mut code_hash: Option<String> = None;
    let mut role: Option<Role> = None;
    
    // Handle incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg? {
            Message::Text(text) => {
                // Handle handshake
                if code_hash.is_none() {
                    match RelayMessage::from_json(&text) {
                        Ok(RelayMessage::Register { role: r, code_hash: ch }) => {
                            println!("[{}] Registered as {:?} with code hash {}", addr, r, &ch[..8]);
                            
                            // Store this peer
                            let peer = Peer {
                                role: r.clone(),
                                tx: tx.clone(),
                                addr,
                            };
                            
                            let mut peers_lock = peers.lock().await;
                            
                            // Check if there's a matching peer
                            if let Some(other_peer) = peers_lock.get(&ch) {
                                // Ensure roles are different
                                if other_peer.role != r {
                                    // Match found! Notify both
                                    println!("[{}] ✓ Matched with {}", addr, other_peer.addr);
                                    
                                    let matched_msg = RelayMessage::Matched.to_json()?;
                                    
                                    // Notify both peers
                                    let _ = tx.send(Message::Text(matched_msg.clone()));
                                    let _ = other_peer.tx.send(Message::Text(matched_msg));
                                    
                                    code_hash = Some(ch.clone());
                                    role = Some(r);
                                } else {
                                    // Same role - error
                                    let error_msg = RelayMessage::Error {
                                        message: "Both peers have the same role".to_string(),
                                    }.to_json()?;
                                    let _ = tx.send(Message::Text(error_msg));
                                    return Ok(());
                                }
                            } else {
                                // No match yet, wait for peer
                                peers_lock.insert(ch.clone(), peer);
                                code_hash = Some(ch);
                                role = Some(r);
                                println!("[{}] Waiting for matching peer...", addr);
                            }
                        }
                        Ok(RelayMessage::Ping) => {
                            let _ = tx.send(Message::Text(RelayMessage::Pong.to_json()?));
                        }
                        _ => {
                            let error_msg = RelayMessage::Error {
                                message: "Expected Register message".to_string(),
                            }.to_json()?;
                            let _ = tx.send(Message::Text(error_msg));
                            return Ok(());
                        }
                    }
                }
            }
            Message::Binary(data) => {
                // After matched, forward binary data to the other peer
                if let Some(ref ch) = code_hash {
                    let peers_lock = peers.lock().await;
                    if let Some(other_peer) = peers_lock.get(ch) {
                        // Only forward if roles are different (the matched peer)
                        if let Some(ref my_role) = role {
                            if &other_peer.role != my_role {
                                let _ = other_peer.tx.send(Message::Binary(data));
                            }
                        }
                    }
                }
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }
    
    // Cleanup
    if let Some(ch) = code_hash {
        let mut peers_lock = peers.lock().await;
        peers_lock.remove(&ch);
        println!("[{}] Disconnected", addr);
    }
    
    forward_task.abort();
    Ok(())
}
