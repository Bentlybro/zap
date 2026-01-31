use anyhow::{anyhow, Result};
use std::fs::{File, metadata};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB chunks

/// File metadata for transfer
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub checksum: String,
}

/// Read file metadata
pub async fn get_file_metadata(path: &Path) -> Result<FileMetadata> {
    let metadata = async_fs::metadata(path).await?;
    
    let name = path
        .file_name()
        .ok_or_else(|| anyhow!("Invalid file path"))?
        .to_string_lossy()
        .to_string();
    
    let is_directory = metadata.is_dir();
    let size = if is_directory { 0 } else { metadata.len() };
    
    // For MVP, we'll skip checksum calculation for large files
    let checksum = String::from("tbd");
    
    Ok(FileMetadata {
        name,
        size,
        is_directory,
        checksum,
    })
}

/// File chunker for streaming transfer
pub struct FileChunker {
    file: File,
    chunk_size: usize,
    total_size: u64,
    bytes_read: u64,
}

impl FileChunker {
    /// Create a new file chunker
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let total_size = file.metadata()?.len();
        
        Ok(Self {
            file,
            chunk_size: CHUNK_SIZE,
            total_size,
            bytes_read: 0,
        })
    }
    
    /// Read the next chunk
    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>> {
        if self.bytes_read >= self.total_size {
            return Ok(None);
        }
        
        let mut buffer = vec![0u8; self.chunk_size];
        let bytes_read = self.file.read(&mut buffer)?;
        
        if bytes_read == 0 {
            return Ok(None);
        }
        
        buffer.truncate(bytes_read);
        self.bytes_read += bytes_read as u64;
        Ok(Some(buffer))
    }
    
    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.total_size == 0 {
            return 1.0;
        }
        self.bytes_read as f64 / self.total_size as f64
    }
    
    /// Get total size
    pub fn total_size(&self) -> u64 {
        self.total_size
    }
    
    /// Get bytes read
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }
}

/// File writer for receiving chunks
pub struct FileWriter {
    file: File,
    bytes_written: u64,
    expected_size: u64,
}

impl FileWriter {
    /// Create a new file writer
    pub fn new(path: &Path, expected_size: u64) -> Result<Self> {
        let file = File::create(path)?;
        
        Ok(Self {
            file,
            bytes_written: 0,
            expected_size,
        })
    }
    
    /// Write a chunk
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<()> {
        self.file.write_all(data)?;
        self.bytes_written += data.len() as u64;
        Ok(())
    }
    
    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.expected_size == 0 {
            return 1.0;
        }
        self.bytes_written as f64 / self.expected_size as f64
    }
    
    /// Check if transfer is complete
    pub fn is_complete(&self) -> bool {
        self.bytes_written >= self.expected_size
    }
    
    /// Get bytes written
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
    
    /// Finalize the file
    pub fn finalize(self) -> Result<()> {
        self.file.sync_all()?;
        Ok(())
    }
}

/// Create a tar archive from a directory (for directory transfers)
pub fn create_tar_archive(dir_path: &Path, output_path: &Path) -> Result<()> {
    let tar_file = File::create(output_path)?;
    let mut archive = tar::Builder::new(tar_file);
    
    archive.append_dir_all(".", dir_path)?;
    archive.finish()?;
    
    Ok(())
}

/// Extract a tar archive (for directory transfers)
pub fn extract_tar_archive(archive_path: &Path, output_dir: &Path) -> Result<()> {
    let tar_file = File::open(archive_path)?;
    let mut archive = tar::Archive::new(tar_file);
    
    archive.unpack(output_dir)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_chunker_writer() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, Zap! This is a test file for chunking.";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();
        
        let mut chunker = FileChunker::new(temp_file.path()).unwrap();
        let mut output_file = NamedTempFile::new().unwrap();
        let mut writer = FileWriter::new(output_file.path(), test_data.len() as u64).unwrap();
        
        while let Some(chunk) = chunker.next_chunk().unwrap() {
            writer.write_chunk(&chunk).unwrap();
        }
        
        writer.finalize().unwrap();
        
        let mut result = Vec::new();
        output_file.reopen().unwrap().read_to_end(&mut result).unwrap();
        assert_eq!(result, test_data);
    }
}
