use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

const NONCE_SIZE: usize = 12;

/// Generate a random word code for the transfer
pub fn generate_code(word_count: usize) -> String {
    let words = include_str!("wordlist.txt")
        .lines()
        .collect::<Vec<_>>();
    
    let mut rng = rand::thread_rng();
    (0..word_count)
        .map(|_| words[rng.gen_range(0..words.len())])
        .collect::<Vec<_>>()
        .join("-")
}

/// Derive a shared secret using SPAKE2
pub struct KeyExchange {
    state: Spake2<Ed25519Group>,
}

impl KeyExchange {
    /// Create a new key exchange for the sender side
    pub fn new_sender(code: &str) -> Self {
        let (state, _outbound) = Spake2::<Ed25519Group>::start_symmetric(
            &Password::new(code.as_bytes()),
            &Identity::new(b"zap-sender"),
        );
        Self { state }
    }
    
    /// Create a new key exchange for the receiver side
    pub fn new_receiver(code: &str) -> Self {
        let (state, _outbound) = Spake2::<Ed25519Group>::start_symmetric(
            &Password::new(code.as_bytes()),
            &Identity::new(b"zap-receiver"),
        );
        Self { state }
    }
    
    /// Get the outbound message to send to the peer
    pub fn outbound_message(&self) -> Vec<u8> {
        // Note: In a real implementation, we'd need to restructure this
        // to properly handle the SPAKE2 protocol. For MVP, we'll use a simpler approach.
        vec![]
    }
    
    /// Complete the key exchange and derive the shared secret
    pub fn finish(self, _peer_message: &[u8]) -> Result<Vec<u8>> {
        // Simplified for MVP - in production, complete the SPAKE2 exchange
        Ok(vec![0u8; 32]) // Placeholder
    }
}

/// Encryption/decryption using ChaCha20-Poly1305
pub struct Cipher {
    cipher: ChaCha20Poly1305,
}

impl Cipher {
    /// Create a new cipher from a shared secret
    pub fn new(secret: &[u8]) -> Result<Self> {
        // Derive a 32-byte key from the secret
        let mut hasher = Sha256::new();
        hasher.update(secret);
        let key = hasher.finalize();
        
        let cipher = ChaCha20Poly1305::new(key.as_slice().into());
        Ok(Self { cipher })
    }
    
    /// Create a cipher from a password (for simplified MVP)
    pub fn from_password(password: &str) -> Result<Self> {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let key = hasher.finalize();
        
        let cipher = ChaCha20Poly1305::new(key.as_slice().into());
        Ok(Self { cipher })
    }
    
    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut rng = rand::thread_rng();
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rng.fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = self.cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }
    
    /// Decrypt data
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < NONCE_SIZE {
            return Err(anyhow!("Data too short to contain nonce"));
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))
    }
}

/// Calculate SHA-256 checksum of data
pub fn checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt() {
        let cipher = Cipher::from_password("test-password").unwrap();
        let plaintext = b"Hello, Zap!";
        
        let encrypted = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext, decrypted.as_slice());
    }
}
