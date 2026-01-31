mod cli;
mod crypto;
mod network;
mod protocol;
mod transfer;
mod tui;

use anyhow::Result;
use cli::{Cli, Commands};
use crypto::Cipher;
use network::{connect, listen, Connection};
use protocol::Message;
use std::path::Path;
use std::time::{Duration, Instant};
use transfer::{FileChunker, FileMetadata, FileWriter};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse_args();
    
    match cli.command {
        Commands::Send { path, code, words } => {
            send_file(path, code, words, cli.port, cli.no_tui).await?;
        }
        Commands::Receive { code, output, resume } => {
            receive_file(code, output, cli.port, cli.no_tui, resume).await?;
        }
    }
    
    Ok(())
}

async fn send_file(
    path: Option<std::path::PathBuf>,
    custom_code: Option<String>,
    word_count: usize,
    port: Option<u16>,
    no_tui: bool,
) -> Result<()> {
    // Generate or use custom code
    let code = custom_code.unwrap_or_else(|| crypto::generate_code(word_count));
    
    println!("⚡ Zap - Send File");
    println!("═══════════════════════════════════════");
    println!("Transfer Code: \x1b[1;32m{}\x1b[0m", code);
    println!("Waiting for receiver...");
    println!();
    
    // For MVP, we'll use the path if provided, otherwise error
    let file_path = path.ok_or_else(|| anyhow::anyhow!("File path required for MVP"))?;
    
    // Get file metadata
    let metadata = transfer::get_file_metadata(&file_path).await?;
    println!("File: {} ({} bytes)", metadata.name, metadata.size);
    
    // Wait for connection
    let mut conn = listen(port).await?;
    println!("✓ Connected to {}", conn.peer_addr());
    
    // Send hello
    let hello = Message::Hello { version: protocol::PROTOCOL_VERSION };
    conn.send(&hello.to_bytes()?).await?;
    
    // Receive hello
    let response = conn.receive().await?;
    let response_msg = Message::from_bytes(&response)?;
    match response_msg {
        Message::Hello { version } => {
            if version != protocol::PROTOCOL_VERSION {
                return Err(anyhow::anyhow!("Protocol version mismatch"));
            }
        }
        _ => return Err(anyhow::anyhow!("Expected Hello message")),
    }
    
    println!("✓ Handshake complete");
    
    // Create cipher from code
    let cipher = Cipher::from_password(&code)?;
    
    // Send metadata
    let metadata_msg = Message::Metadata {
        filename: metadata.name.clone(),
        size: metadata.size,
        is_directory: metadata.is_directory,
        checksum: metadata.checksum.clone(),
    };
    let encrypted_metadata = cipher.encrypt(&metadata_msg.to_bytes()?)?;
    conn.send(&encrypted_metadata).await?;
    
    println!("✓ Metadata sent (encrypted)");
    
    // Wait for ack
    let ack = conn.receive().await?;
    let ack_msg = Message::from_bytes(&ack)?;
    match ack_msg {
        Message::Ack => {}
        _ => return Err(anyhow::anyhow!("Expected Ack message")),
    }
    
    // Send file chunks
    println!("Transferring file...");
    let mut chunker = FileChunker::new(&file_path)?;
    let mut chunk_index = 0u64;
    let start_time = Instant::now();
    
    while let Some(chunk) = chunker.next_chunk()? {
        let chunk_msg = Message::Chunk {
            index: chunk_index,
            data: chunk,
        };
        let encrypted_chunk = cipher.encrypt(&chunk_msg.to_bytes()?)?;
        conn.send(&encrypted_chunk).await?;
        
        chunk_index += 1;
        
        // Progress update
        if !no_tui {
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                chunker.bytes_read() as f64 / elapsed
            } else {
                0.0
            };
            tui::print_progress(
                &metadata.name,
                chunker.bytes_read(),
                chunker.total_size(),
                speed,
            );
        }
    }
    
    // Send complete message
    let complete_msg = Message::Complete;
    let encrypted_complete = cipher.encrypt(&complete_msg.to_bytes()?)?;
    conn.send(&encrypted_complete).await?;
    
    println!();
    println!("✓ Transfer complete!");
    
    Ok(())
}

async fn receive_file(
    code: String,
    output: Option<std::path::PathBuf>,
    port: Option<u16>,
    no_tui: bool,
    resume: bool,
) -> Result<()> {
    println!("⚡ Zap - Receive File");
    println!("═══════════════════════════════════════");
    println!("Transfer Code: \x1b[1;32m{}\x1b[0m", code);
    println!("Connecting to sender...");
    println!();
    
    // For MVP, require host to connect to
    // In full version, we'd use mDNS discovery
    println!("Enter sender's IP address (or 'localhost' for local transfer):");
    let mut host = String::new();
    std::io::stdin().read_line(&mut host)?;
    let host = host.trim();
    
    // Connect to sender
    let mut conn = connect(host, port).await?;
    println!("✓ Connected to {}", conn.peer_addr());
    
    // Send hello
    let hello = Message::Hello { version: protocol::PROTOCOL_VERSION };
    conn.send(&hello.to_bytes()?).await?;
    
    // Receive hello
    let response = conn.receive().await?;
    let response_msg = Message::from_bytes(&response)?;
    match response_msg {
        Message::Hello { version } => {
            if version != protocol::PROTOCOL_VERSION {
                return Err(anyhow::anyhow!("Protocol version mismatch"));
            }
        }
        _ => return Err(anyhow::anyhow!("Expected Hello message")),
    }
    
    println!("✓ Handshake complete");
    
    // Create cipher from code
    let cipher = Cipher::from_password(&code)?;
    
    // Receive metadata
    let encrypted_metadata = conn.receive().await?;
    let metadata_bytes = cipher.decrypt(&encrypted_metadata)?;
    let metadata_msg = Message::from_bytes(&metadata_bytes)?;
    
    let (filename, file_size) = match metadata_msg {
        Message::Metadata { filename, size, .. } => {
            println!("✓ Metadata received (encrypted)");
            println!("File: {} ({} bytes)", filename, size);
            (filename, size)
        }
        _ => return Err(anyhow::anyhow!("Expected Metadata message")),
    };
    
    // Send ack
    let ack = Message::Ack;
    conn.send(&ack.to_bytes()?).await?;
    
    // Determine output path
    let output_path = output.unwrap_or_else(|| std::path::PathBuf::from(&filename));
    
    // Create file writer
    let mut writer = FileWriter::new(&output_path, file_size)?;
    println!("Receiving file...");
    let start_time = Instant::now();
    
    // Receive chunks
    loop {
        let encrypted_chunk = conn.receive().await?;
        let chunk_bytes = cipher.decrypt(&encrypted_chunk)?;
        let chunk_msg = Message::from_bytes(&chunk_bytes)?;
        
        match chunk_msg {
            Message::Chunk { data, .. } => {
                writer.write_chunk(&data)?;
                
                // Progress update
                if !no_tui {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        writer.bytes_written() as f64 / elapsed
                    } else {
                        0.0
                    };
                    tui::print_progress(
                        &filename,
                        writer.bytes_written(),
                        file_size,
                        speed,
                    );
                }
            }
            Message::Complete => {
                writer.finalize()?;
                println!();
                println!("✓ Transfer complete!");
                println!("File saved to: {}", output_path.display());
                break;
            }
            Message::Error { message } => {
                return Err(anyhow::anyhow!("Transfer error: {}", message));
            }
            _ => return Err(anyhow::anyhow!("Unexpected message type")),
        }
    }
    
    Ok(())
}
