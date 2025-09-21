use sha2::{Sha256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, AeadCore, OsRng};
use std::fs::{File, self};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

// Chunk size constant - 256KB as per spec
const CHUNK_SIZE: usize = 256 * 1024;

// Magic number for chunk headers
const CHUNK_MAGIC: [u8; 4] = [0x43, 0x48, 0x4E, 0x4B]; // "CHNK"
const CHUNK_VERSION: u16 = 0x0001;

// Header size constants
const HEADER_SIZE: usize = 64;
const METADATA_SIZE: usize = 256;
const CHECKSUM_SIZE: usize = 32;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChunkInfo {
    pub index: u32,
    pub hash: String,
    pub size: usize,
    pub encrypted_size: usize,
    pub total_chunks: u32,
    pub file_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChunkHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub file_hash: [u8; 32],
    pub chunk_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChunkMetadata {
    pub iv: [u8; 16],
    pub compression_type: u8,
    pub original_size: u64,
    pub compressed_size: u64,
    pub timestamp: u64,
    // Reserved space: 215 bytes
}

pub struct ChunkManager {
    chunk_size: usize,
    storage_path: PathBuf,
}

impl ChunkManager {
    pub fn new(storage_path: PathBuf) -> Self {
        ChunkManager {
            chunk_size: CHUNK_SIZE,
            storage_path,
        }
    }

    /// Chunks a file into encrypted pieces with proper headers and metadata
    /// Returns chunk information and processes chunks via streaming to handle large files
    pub fn chunk_file(&self, file_path: &Path, encryption_key: &[u8; 32]) -> Result<Vec<ChunkInfo>> {
        let mut file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;

        // Get file size for total chunks calculation
        let file_size = file.metadata()
            .context("Failed to get file metadata")?
            .len();
        
        let total_chunks = ((file_size + self.chunk_size as u64 - 1) / self.chunk_size as u64) as u32;
        
        // Calculate file hash
        let file_hash = self.hash_file(file_path)?;
        let file_hash_bytes = hex::decode(&file_hash)
            .context("Failed to decode file hash")?;
        let mut file_hash_array = [0u8; 32];
        file_hash_array.copy_from_slice(&file_hash_bytes[..32]);

        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; self.chunk_size];
        let mut index = 0;

        // Create storage directory if it doesn't exist
        fs::create_dir_all(&self.storage_path)
            .context("Failed to create storage directory")?;

        loop {
            let bytes_read = file.read(&mut buffer)
                .context("Failed to read chunk from file")?;
            
            if bytes_read == 0 {
                break;
            }

            let chunk_data = &buffer[..bytes_read];
            
            // Hash the original chunk data
            let chunk_hash = self.hash_chunk(chunk_data);
            let chunk_hash_bytes = hex::decode(&chunk_hash)
                .context("Failed to decode chunk hash")?;
            let mut chunk_hash_array = [0u8; 32];
            chunk_hash_array.copy_from_slice(&chunk_hash_bytes[..32]);

            // Encrypt the chunk with a unique nonce
            let (encrypted_data, nonce) = self.encrypt_chunk(chunk_data, encryption_key)?;
            
            // Create chunk header
            let header = ChunkHeader {
                magic: CHUNK_MAGIC,
                version: CHUNK_VERSION,
                chunk_index: index,
                total_chunks,
                file_hash: file_hash_array,
                chunk_hash: chunk_hash_array,
            };

            // Create metadata
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let metadata = ChunkMetadata {
                iv: nonce,
                compression_type: 0, // No compression for now
                original_size: bytes_read as u64,
                compressed_size: encrypted_data.len() as u64,
                timestamp,
            };

            // Write chunk to storage
            let chunk_file_data = self.create_chunk_file(&header, &metadata, &encrypted_data)?;
            let final_hash = self.save_chunk(&chunk_hash, &chunk_file_data)?;

            chunks.push(ChunkInfo {
                index,
                hash: final_hash,
                size: bytes_read,
                encrypted_size: chunk_file_data.len(),
                total_chunks,
                file_hash: file_hash.clone(),
            });

            index += 1;
        }

        Ok(chunks)
    }

    /// Encrypts chunk data using AES-256-GCM with a unique nonce
    fn encrypt_chunk(&self, data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 16])> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        
        // Generate a unique 96-bit nonce for AES-GCM
        let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);
        
        let ciphertext = cipher.encrypt(&nonce_bytes, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        // Convert nonce to 16-byte array (pad with zeros if needed)
        let mut nonce_array = [0u8; 16];
        nonce_array[..12].copy_from_slice(&nonce_bytes);

        Ok((ciphertext, nonce_array))
    }

    /// Decrypts chunk data using AES-256-GCM
    pub fn decrypt_chunk(&self, encrypted_data: &[u8], key: &[u8; 32], nonce: &[u8; 16]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        
        // Extract the 12-byte nonce from the 16-byte array
        let nonce_bytes = Nonce::from_slice(&nonce[..12]);
        
        cipher.decrypt(nonce_bytes, encrypted_data)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    /// Hashes chunk data using SHA-256
    fn hash_chunk(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Hashes entire file using SHA-256 with streaming
    pub fn hash_file(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path)
            .with_context(|| format!("Failed to open file for hashing: {}", file_path.display()))?;
        
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .context("Failed to read file for hashing")?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Creates the complete chunk file with header, metadata, and encrypted data
    fn create_chunk_file(&self, header: &ChunkHeader, metadata: &ChunkMetadata, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        let mut chunk_file = Vec::new();

        // Write header (64 bytes)
        chunk_file.extend_from_slice(&header.magic);
        chunk_file.extend_from_slice(&header.version.to_le_bytes());
        chunk_file.extend_from_slice(&header.chunk_index.to_le_bytes());
        chunk_file.extend_from_slice(&header.total_chunks.to_le_bytes());
        chunk_file.extend_from_slice(&header.file_hash);
        chunk_file.extend_from_slice(&header.chunk_hash);
        
        // Pad header to 64 bytes
        while chunk_file.len() < HEADER_SIZE {
            chunk_file.push(0);
        }

        // Write metadata (256 bytes)
        let metadata_start = chunk_file.len();
        chunk_file.extend_from_slice(&metadata.iv);
        chunk_file.push(metadata.compression_type);
        chunk_file.extend_from_slice(&metadata.original_size.to_le_bytes());
        chunk_file.extend_from_slice(&metadata.compressed_size.to_le_bytes());
        chunk_file.extend_from_slice(&metadata.timestamp.to_le_bytes());
        
        // Pad metadata to 256 bytes
        while chunk_file.len() < metadata_start + METADATA_SIZE {
            chunk_file.push(0);
        }

        // Write encrypted data
        chunk_file.extend_from_slice(encrypted_data);

        // Calculate and append checksum
        let checksum = self.hash_chunk(&chunk_file);
        let checksum_bytes = hex::decode(&checksum)
            .context("Failed to decode checksum")?;
        chunk_file.extend_from_slice(&checksum_bytes);

        Ok(chunk_file)
    }

    /// Saves chunk data to storage and returns the final storage hash
    fn save_chunk(&self, chunk_hash: &str, chunk_data: &[u8]) -> Result<String> {
        // Use the chunk hash as the filename
        let file_path = self.storage_path.join(chunk_hash);
        
        let mut file = File::create(&file_path)
            .with_context(|| format!("Failed to create chunk file: {}", file_path.display()))?;
        
        file.write_all(chunk_data)
            .context("Failed to write chunk data")?;
        
        file.sync_all()
            .context("Failed to sync chunk file")?;

        Ok(chunk_hash.to_string())
    }

    /// Loads a chunk from storage
    pub fn load_chunk(&self, chunk_hash: &str) -> Result<Vec<u8>> {
        let file_path = self.storage_path.join(chunk_hash);
        
        fs::read(&file_path)
            .with_context(|| format!("Failed to read chunk file: {}", file_path.display()))
    }

    /// Validates a chunk file's integrity
    pub fn validate_chunk(&self, chunk_data: &[u8]) -> Result<bool> {
        if chunk_data.len() < HEADER_SIZE + METADATA_SIZE + CHECKSUM_SIZE {
            return Ok(false);
        }

        // Extract and verify checksum
        let data_without_checksum = &chunk_data[..chunk_data.len() - CHECKSUM_SIZE];
        let stored_checksum = &chunk_data[chunk_data.len() - CHECKSUM_SIZE..];
        
        let calculated_checksum = self.hash_chunk(data_without_checksum);
        let calculated_checksum_bytes = hex::decode(&calculated_checksum)
            .context("Failed to decode calculated checksum")?;

        Ok(stored_checksum == calculated_checksum_bytes)
    }

    /// Extracts header from chunk data
    pub fn extract_header(&self, chunk_data: &[u8]) -> Result<ChunkHeader> {
        if chunk_data.len() < HEADER_SIZE {
            return Err(anyhow::anyhow!("Chunk data too small for header"));
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&chunk_data[0..4]);
        
        if magic != CHUNK_MAGIC {
            return Err(anyhow::anyhow!("Invalid chunk magic number"));
        }

        let version = u16::from_le_bytes([chunk_data[4], chunk_data[5]]);
        let chunk_index = u32::from_le_bytes([chunk_data[6], chunk_data[7], chunk_data[8], chunk_data[9]]);
        let total_chunks = u32::from_le_bytes([chunk_data[10], chunk_data[11], chunk_data[12], chunk_data[13]]);
        
        let mut file_hash = [0u8; 32];
        file_hash.copy_from_slice(&chunk_data[14..46]);
        
        let mut chunk_hash = [0u8; 32];
        chunk_hash.copy_from_slice(&chunk_data[46..78]);

        Ok(ChunkHeader {
            magic,
            version,
            chunk_index,
            total_chunks,
            file_hash,
            chunk_hash,
        })
    }

    /// Extracts metadata from chunk data
    pub fn extract_metadata(&self, chunk_data: &[u8]) -> Result<ChunkMetadata> {
        if chunk_data.len() < HEADER_SIZE + METADATA_SIZE {
            return Err(anyhow::anyhow!("Chunk data too small for metadata"));
        }

        let metadata_start = HEADER_SIZE;
        let metadata_slice = &chunk_data[metadata_start..metadata_start + METADATA_SIZE];

        let mut iv = [0u8; 16];
        iv.copy_from_slice(&metadata_slice[0..16]);
        
        let compression_type = metadata_slice[16];
        let original_size = u64::from_le_bytes([
            metadata_slice[17], metadata_slice[18], metadata_slice[19], metadata_slice[20],
            metadata_slice[21], metadata_slice[22], metadata_slice[23], metadata_slice[24],
        ]);
        let compressed_size = u64::from_le_bytes([
            metadata_slice[25], metadata_slice[26], metadata_slice[27], metadata_slice[28],
            metadata_slice[29], metadata_slice[30], metadata_slice[31], metadata_slice[32],
        ]);
        let timestamp = u64::from_le_bytes([
            metadata_slice[33], metadata_slice[34], metadata_slice[35], metadata_slice[36],
            metadata_slice[37], metadata_slice[38], metadata_slice[39], metadata_slice[40],
        ]);

        Ok(ChunkMetadata {
            iv,
            compression_type,
            original_size,
            compressed_size,
            timestamp,
        })
    }

    /// Extracts encrypted data from chunk
    pub fn extract_encrypted_data(&self, chunk_data: &[u8]) -> Result<Vec<u8>> {
        if chunk_data.len() < HEADER_SIZE + METADATA_SIZE + CHECKSUM_SIZE {
            return Err(anyhow::anyhow!("Chunk data too small"));
        }

        let data_start = HEADER_SIZE + METADATA_SIZE;
        let data_end = chunk_data.len() - CHECKSUM_SIZE;
        
        Ok(chunk_data[data_start..data_end].to_vec())
    }

    /// Reassembles chunks back into the original file
    pub fn reassemble_file(&self, chunks: &[ChunkInfo], output_path: &Path, encryption_key: &[u8; 32]) -> Result<()> {
        let mut output_file = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;

        // Sort chunks by index to ensure correct order
        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.index);

        for chunk_info in sorted_chunks {
            // Load chunk data
            let chunk_data = self.load_chunk(&chunk_info.hash)?;
            
            // Validate chunk
            if !self.validate_chunk(&chunk_data)? {
                return Err(anyhow::anyhow!("Chunk validation failed for chunk {}", chunk_info.index));
            }

            // Extract metadata and encrypted data
            let metadata = self.extract_metadata(&chunk_data)?;
            let encrypted_data = self.extract_encrypted_data(&chunk_data)?;

            // Decrypt chunk
            let decrypted_data = self.decrypt_chunk(&encrypted_data, encryption_key, &metadata.iv)?;

            // Write to output file
            output_file.write_all(&decrypted_data)
                .context("Failed to write decrypted chunk to output file")?;
        }

        output_file.sync_all()
            .context("Failed to sync output file")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_chunk_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ChunkManager::new(temp_dir.path().to_path_buf());
        assert_eq!(manager.chunk_size, CHUNK_SIZE);
    }

    #[test]
    fn test_hash_chunk() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ChunkManager::new(temp_dir.path().to_path_buf());
        
        let data = b"Hello, World!";
        let hash = manager.hash_chunk(data);
        
        // SHA-256 of "Hello, World!" should be consistent
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    }

    #[test]
    fn test_encrypt_decrypt_chunk() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ChunkManager::new(temp_dir.path().to_path_buf());
        
        let data = b"Test data for encryption";
        let key = [1u8; 32]; // Test key
        
        let (encrypted, nonce) = manager.encrypt_chunk(data, &key).unwrap();
        let decrypted = manager.decrypt_chunk(&encrypted, &key, &nonce).unwrap();
        
        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_chunk_file_small() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ChunkManager::new(temp_dir.path().to_path_buf());
        
        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"Hello, chunking world!").unwrap();
        
        let key = [2u8; 32];
        let chunks = manager.chunk_file(&test_file, &key).unwrap();
        
        assert_eq!(chunks.len(), 1); // Should be only one chunk for small file
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].size, 22); // Length of test data
    }

    #[test]
    fn test_chunk_file_large() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ChunkManager::new(temp_dir.path().to_path_buf());
        
        // Create a test file larger than chunk size
        let test_data = vec![42u8; CHUNK_SIZE + 1000]; // Slightly larger than one chunk
        let test_file = temp_dir.path().join("large_test.bin");
        fs::write(&test_file, &test_data).unwrap();
        
        let key = [3u8; 32];
        let chunks = manager.chunk_file(&test_file, &key).unwrap();
        
        assert_eq!(chunks.len(), 2); // Should be two chunks
        assert_eq!(chunks[0].size, CHUNK_SIZE);
        assert_eq!(chunks[1].size, 1000);
    }

    #[test]
    fn test_reassemble_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ChunkManager::new(temp_dir.path().to_path_buf());
        
        // Create original test data
        let original_data = b"This is test data for chunking and reassembly!";
        let test_file = temp_dir.path().join("original.txt");
        fs::write(&test_file, original_data).unwrap();
        
        let key = [4u8; 32];
        
        // Chunk the file
        let chunks = manager.chunk_file(&test_file, &key).unwrap();
        
        // Reassemble to a new file
        let output_file = temp_dir.path().join("reassembled.txt");
        manager.reassemble_file(&chunks, &output_file, &key).unwrap();
        
        // Verify the reassembled file matches the original
        let reassembled_data = fs::read(&output_file).unwrap();
        assert_eq!(original_data.to_vec(), reassembled_data);
    }
}