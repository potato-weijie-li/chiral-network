use tauri::command;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};
use directories::UserDirs;

// Inline chunk manager
use sha2::{Sha256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, OsRng};
use std::fs::File;
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

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
}

pub struct ChunkManager {
    chunk_size: usize,
    pub storage_path: PathBuf,
}

impl ChunkManager {
    pub fn new(storage_path: PathBuf) -> Self {
        ChunkManager {
            chunk_size: CHUNK_SIZE,
            storage_path,
        }
    }

    /// Chunks a file into encrypted pieces with proper headers and metadata
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
                compression_type: 0,
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

    fn encrypt_chunk(&self, data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 16])> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce_bytes, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        let mut nonce_array = [0u8; 16];
        nonce_array[..12].copy_from_slice(&nonce_bytes);
        Ok((ciphertext, nonce_array))
    }

    pub fn decrypt_chunk(&self, encrypted_data: &[u8], key: &[u8; 32], nonce: &[u8; 16]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce_bytes = Nonce::from_slice(&nonce[..12]);
        cipher.decrypt(nonce_bytes, encrypted_data)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    fn hash_chunk(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    pub fn hash_file(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path)
            .with_context(|| format!("Failed to open file for hashing: {}", file_path.display()))?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            let bytes_read = file.read(&mut buffer)
                .context("Failed to read file for hashing")?;
            if bytes_read == 0 { break; }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn create_chunk_file(&self, header: &ChunkHeader, metadata: &ChunkMetadata, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        let mut chunk_file = Vec::new();
        chunk_file.extend_from_slice(&header.magic);
        chunk_file.extend_from_slice(&header.version.to_le_bytes());
        chunk_file.extend_from_slice(&header.chunk_index.to_le_bytes());
        chunk_file.extend_from_slice(&header.total_chunks.to_le_bytes());
        chunk_file.extend_from_slice(&header.file_hash);
        chunk_file.extend_from_slice(&header.chunk_hash);
        while chunk_file.len() < HEADER_SIZE { chunk_file.push(0); }
        
        let metadata_start = chunk_file.len();
        chunk_file.extend_from_slice(&metadata.iv);
        chunk_file.push(metadata.compression_type);
        chunk_file.extend_from_slice(&metadata.original_size.to_le_bytes());
        chunk_file.extend_from_slice(&metadata.compressed_size.to_le_bytes());
        chunk_file.extend_from_slice(&metadata.timestamp.to_le_bytes());
        while chunk_file.len() < metadata_start + METADATA_SIZE { chunk_file.push(0); }
        
        chunk_file.extend_from_slice(encrypted_data);
        let checksum = self.hash_chunk(&chunk_file);
        let checksum_bytes = hex::decode(&checksum).context("Failed to decode checksum")?;
        chunk_file.extend_from_slice(&checksum_bytes);
        Ok(chunk_file)
    }

    fn save_chunk(&self, chunk_hash: &str, chunk_data: &[u8]) -> Result<String> {
        let file_path = self.storage_path.join(chunk_hash);
        let mut file = File::create(&file_path)
            .with_context(|| format!("Failed to create chunk file: {}", file_path.display()))?;
        file.write_all(chunk_data).context("Failed to write chunk data")?;
        file.sync_all().context("Failed to sync chunk file")?;
        Ok(chunk_hash.to_string())
    }

    pub fn load_chunk(&self, chunk_hash: &str) -> Result<Vec<u8>> {
        let file_path = self.storage_path.join(chunk_hash);
        fs::read(&file_path)
            .with_context(|| format!("Failed to read chunk file: {}", file_path.display()))
    }

    pub fn validate_chunk(&self, chunk_data: &[u8]) -> Result<bool> {
        if chunk_data.len() < HEADER_SIZE + METADATA_SIZE + CHECKSUM_SIZE { return Ok(false); }
        let data_without_checksum = &chunk_data[..chunk_data.len() - CHECKSUM_SIZE];
        let stored_checksum = &chunk_data[chunk_data.len() - CHECKSUM_SIZE..];
        let calculated_checksum = self.hash_chunk(data_without_checksum);
        let calculated_checksum_bytes = hex::decode(&calculated_checksum)
            .context("Failed to decode calculated checksum")?;
        Ok(stored_checksum == calculated_checksum_bytes)
    }

    pub fn extract_header(&self, chunk_data: &[u8]) -> Result<ChunkHeader> {
        if chunk_data.len() < HEADER_SIZE { return Err(anyhow::anyhow!("Chunk data too small for header")); }
        let mut magic = [0u8; 4]; magic.copy_from_slice(&chunk_data[0..4]);
        if magic != CHUNK_MAGIC { return Err(anyhow::anyhow!("Invalid chunk magic number")); }
        let version = u16::from_le_bytes([chunk_data[4], chunk_data[5]]);
        let chunk_index = u32::from_le_bytes([chunk_data[6], chunk_data[7], chunk_data[8], chunk_data[9]]);
        let total_chunks = u32::from_le_bytes([chunk_data[10], chunk_data[11], chunk_data[12], chunk_data[13]]);
        let mut file_hash = [0u8; 32]; file_hash.copy_from_slice(&chunk_data[14..46]);
        let mut chunk_hash = [0u8; 32]; chunk_hash.copy_from_slice(&chunk_data[46..78]);
        Ok(ChunkHeader { magic, version, chunk_index, total_chunks, file_hash, chunk_hash })
    }

    pub fn extract_metadata(&self, chunk_data: &[u8]) -> Result<ChunkMetadata> {
        if chunk_data.len() < HEADER_SIZE + METADATA_SIZE { return Err(anyhow::anyhow!("Chunk data too small for metadata")); }
        let metadata_start = HEADER_SIZE;
        let metadata_slice = &chunk_data[metadata_start..metadata_start + METADATA_SIZE];
        let mut iv = [0u8; 16]; iv.copy_from_slice(&metadata_slice[0..16]);
        let compression_type = metadata_slice[16];
        let original_size = u64::from_le_bytes([metadata_slice[17], metadata_slice[18], metadata_slice[19], metadata_slice[20], metadata_slice[21], metadata_slice[22], metadata_slice[23], metadata_slice[24]]);
        let compressed_size = u64::from_le_bytes([metadata_slice[25], metadata_slice[26], metadata_slice[27], metadata_slice[28], metadata_slice[29], metadata_slice[30], metadata_slice[31], metadata_slice[32]]);
        let timestamp = u64::from_le_bytes([metadata_slice[33], metadata_slice[34], metadata_slice[35], metadata_slice[36], metadata_slice[37], metadata_slice[38], metadata_slice[39], metadata_slice[40]]);
        Ok(ChunkMetadata { iv, compression_type, original_size, compressed_size, timestamp })
    }

    pub fn extract_encrypted_data(&self, chunk_data: &[u8]) -> Result<Vec<u8>> {
        if chunk_data.len() < HEADER_SIZE + METADATA_SIZE + CHECKSUM_SIZE { return Err(anyhow::anyhow!("Chunk data too small")); }
        let data_start = HEADER_SIZE + METADATA_SIZE;
        let data_end = chunk_data.len() - CHECKSUM_SIZE;
        Ok(chunk_data[data_start..data_end].to_vec())
    }

    pub fn reassemble_file(&self, chunks: &[ChunkInfo], output_path: &Path, encryption_key: &[u8; 32]) -> Result<()> {
        let mut output_file = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.index);
        for chunk_info in sorted_chunks {
            let chunk_data = self.load_chunk(&chunk_info.hash)?;
            if !self.validate_chunk(&chunk_data)? { return Err(anyhow::anyhow!("Chunk validation failed for chunk {}", chunk_info.index)); }
            let metadata = self.extract_metadata(&chunk_data)?;
            let encrypted_data = self.extract_encrypted_data(&chunk_data)?;
            let decrypted_data = self.decrypt_chunk(&encrypted_data, encryption_key, &metadata.iv)?;
            output_file.write_all(&decrypted_data).context("Failed to write decrypted chunk to output file")?;
        }
        output_file.sync_all().context("Failed to sync output file")?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileUploadRequest {
    pub file_path: String,
    pub file_name: String,
    pub encryption_key: Option<String>, // Hex-encoded 32-byte key
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileUploadResponse {
    pub file_hash: String,
    pub chunks: Vec<ChunkInfo>,
    pub total_size: u64,
    pub upload_time: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChunkUploadStatus {
    pub chunk_hash: String,
    pub uploaded: bool,
    pub storage_node_url: Option<String>,
    pub error: Option<String>,
}

fn get_storage_path() -> Result<PathBuf> {
    if let Some(user_dirs) = UserDirs::new() {
        let storage_path = user_dirs.document_dir()
            .unwrap_or_else(|| user_dirs.home_dir())
            .join("ChiralNetwork")
            .join("storage");
        
        fs::create_dir_all(&storage_path)
            .with_context(|| format!("Failed to create storage directory: {}", storage_path.display()))?;
        
        Ok(storage_path)
    } else {
        // Fallback to current directory
        let storage_path = PathBuf::from("./chiral_storage");
        fs::create_dir_all(&storage_path)
            .context("Failed to create fallback storage directory")?;
        Ok(storage_path)
    }
}

fn generate_default_encryption_key() -> [u8; 32] {
    // In a real implementation, this should be derived from user's wallet/identity
    // For now, generate a deterministic key (NOT SECURE FOR PRODUCTION)
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(b"chiral_network_default_key_2024");
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn parse_encryption_key(key_hex: &str) -> Result<[u8; 32]> {
    let key_bytes = hex::decode(key_hex)
        .context("Invalid hex encoding for encryption key")?;
    
    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!("Encryption key must be exactly 32 bytes"));
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    Ok(key)
}

#[command]
pub async fn chunk_file(
    file_path: String,
    encryption_key: Option<String>,
) -> Result<FileUploadResponse, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let chunk_manager = ChunkManager::new(storage_path);
    
    // Parse or generate encryption key
    let key = if let Some(key_hex) = encryption_key {
        parse_encryption_key(&key_hex)
            .map_err(|e| format!("Invalid encryption key: {}", e))?
    } else {
        generate_default_encryption_key()
    };
    
    let file_path_buf = PathBuf::from(&file_path);
    
    // Validate file exists
    if !file_path_buf.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }
    
    // Get file size
    let file_size = fs::metadata(&file_path_buf)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len();
    
    // Chunk the file
    let chunks = chunk_manager.chunk_file(&file_path_buf, &key)
        .map_err(|e| format!("Failed to chunk file: {}", e))?;
    
    // Calculate file hash
    let file_hash = chunk_manager.hash_file(&file_path_buf)
        .map_err(|e| format!("Failed to hash file: {}", e))?;
    
    let upload_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    Ok(FileUploadResponse {
        file_hash,
        chunks,
        total_size: file_size,
        upload_time,
    })
}

#[command]
pub async fn upload_chunk_to_storage_node(
    chunk_hash: String,
    storage_node_url: String,
) -> Result<ChunkUploadStatus, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let chunk_manager = ChunkManager::new(storage_path);
    
    // Load chunk data from local storage
    let chunk_data = chunk_manager.load_chunk(&chunk_hash)
        .map_err(|e| format!("Failed to load chunk {}: {}", chunk_hash, e))?;
    
    // Upload to storage node
    let client = reqwest::Client::new();
    let upload_url = format!("{}/chunks", storage_node_url.trim_end_matches('/'));
    
    let response = client
        .post(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .header("x-chunk-hash", &chunk_hash)
        .body(chunk_data)
        .send()
        .await
        .map_err(|e| format!("Failed to upload chunk to storage node: {}", e))?;
    
    if response.status().is_success() {
        Ok(ChunkUploadStatus {
            chunk_hash,
            uploaded: true,
            storage_node_url: Some(storage_node_url),
            error: None,
        })
    } else {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        
        Ok(ChunkUploadStatus {
            chunk_hash,
            uploaded: false,
            storage_node_url: Some(storage_node_url),
            error: Some(format!("HTTP {}: {}", response.status(), error_text)),
        })
    }
}

#[command]
pub async fn upload_file_chunks(
    file_path: String,
    storage_node_urls: Vec<String>,
    encryption_key: Option<String>,
) -> Result<Vec<ChunkUploadStatus>, String> {
    // First, chunk the file
    let upload_response = chunk_file(file_path, encryption_key).await?;
    
    let mut upload_statuses = Vec::new();
    
    // Upload each chunk to storage nodes
    for chunk in upload_response.chunks {
        // For now, just use the first storage node
        // In a real implementation, you might want to distribute chunks across multiple nodes
        if let Some(storage_node_url) = storage_node_urls.first() {
            let status = upload_chunk_to_storage_node(
                chunk.hash.clone(),
                storage_node_url.clone(),
            ).await?;
            
            upload_statuses.push(status);
        } else {
            upload_statuses.push(ChunkUploadStatus {
                chunk_hash: chunk.hash,
                uploaded: false,
                storage_node_url: None,
                error: Some("No storage nodes available".to_string()),
            });
        }
    }
    
    Ok(upload_statuses)
}

#[command]
pub async fn download_chunk_from_storage_node(
    chunk_hash: String,
    storage_node_url: String,
) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let download_url = format!("{}/chunks/{}", storage_node_url.trim_end_matches('/'), chunk_hash);
    
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download chunk from storage node: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Storage node returned error: {}", response.status()));
    }
    
    let chunk_data = response.bytes().await
        .map_err(|e| format!("Failed to read chunk data: {}", e))?;
    
    // Verify chunk hash
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&chunk_data);
    let calculated_hash = format!("{:x}", hasher.finalize());
    
    if calculated_hash != chunk_hash {
        return Err(format!("Chunk hash mismatch: expected {}, got {}", chunk_hash, calculated_hash));
    }
    
    // Store chunk locally
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let chunk_manager = ChunkManager::new(storage_path);
    
    // Save the chunk (it's already in the proper format with headers)
    let chunk_file_path = chunk_manager.storage_path.join(&chunk_hash);
    fs::write(&chunk_file_path, &chunk_data)
        .map_err(|e| format!("Failed to save chunk locally: {}", e))?;
    
    Ok(true)
}

#[command]
pub async fn reassemble_file(
    file_hash: String,
    output_path: String,
    chunks: Vec<ChunkInfo>,
    encryption_key: Option<String>,
) -> Result<String, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let chunk_manager = ChunkManager::new(storage_path);
    
    // Parse or generate encryption key
    let key = if let Some(key_hex) = encryption_key {
        parse_encryption_key(&key_hex)
            .map_err(|e| format!("Invalid encryption key: {}", e))?
    } else {
        generate_default_encryption_key()
    };
    
    let output_path_buf = PathBuf::from(&output_path);
    
    // Ensure output directory exists
    if let Some(parent) = output_path_buf.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    
    // Reassemble the file
    chunk_manager.reassemble_file(&chunks, &output_path_buf, &key)
        .map_err(|e| format!("Failed to reassemble file: {}", e))?;
    
    // Verify the reassembled file hash
    let reassembled_hash = chunk_manager.hash_file(&output_path_buf)
        .map_err(|e| format!("Failed to hash reassembled file: {}", e))?;
    
    if reassembled_hash != file_hash {
        return Err(format!("File hash mismatch after reassembly: expected {}, got {}", file_hash, reassembled_hash));
    }
    
    Ok(output_path)
}

#[command]
pub async fn get_local_chunks() -> Result<Vec<String>, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let entries = fs::read_dir(&storage_path)
        .map_err(|e| format!("Failed to read storage directory: {}", e))?;
    
    let mut chunks = Vec::new();
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        
        if entry.file_type().map_err(|e| format!("Failed to get file type: {}", e))?.is_file() {
            if let Some(file_name) = entry.file_name().to_str() {
                // Validate that it looks like a chunk hash
                if file_name.len() == 64 && file_name.chars().all(|c| c.is_ascii_hexdigit()) {
                    chunks.push(file_name.to_string());
                }
            }
        }
    }
    
    chunks.sort();
    Ok(chunks)
}

#[command]
pub async fn get_chunk_info(chunk_hash: String) -> Result<Option<ChunkInfo>, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let chunk_manager = ChunkManager::new(storage_path);
    
    match chunk_manager.load_chunk(&chunk_hash) {
        Ok(chunk_data) => {
            // Validate the chunk
            let is_valid = chunk_manager.validate_chunk(&chunk_data)
                .map_err(|e| format!("Failed to validate chunk: {}", e))?;
            
            if !is_valid {
                return Ok(None);
            }
            
            // Extract header information
            let header = chunk_manager.extract_header(&chunk_data)
                .map_err(|e| format!("Failed to extract header: {}", e))?;
            
            let metadata = chunk_manager.extract_metadata(&chunk_data)
                .map_err(|e| format!("Failed to extract metadata: {}", e))?;
            
            let chunk_info = ChunkInfo {
                index: header.chunk_index,
                hash: chunk_hash,
                size: metadata.original_size as usize,
                encrypted_size: chunk_data.len(),
                total_chunks: header.total_chunks,
                file_hash: hex::encode(header.file_hash),
            };
            
            Ok(Some(chunk_info))
        }
        Err(_) => Ok(None),
    }
}

#[command]
pub async fn calculate_file_hash(file_path: String) -> Result<String, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let chunk_manager = ChunkManager::new(storage_path);
    let file_path_buf = PathBuf::from(&file_path);
    
    chunk_manager.hash_file(&file_path_buf)
        .map_err(|e| format!("Failed to hash file: {}", e))
}

// Save temporary file from frontend data
#[command]
pub async fn save_temp_file(file_name: String, file_data: Vec<u8>) -> Result<String, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let temp_dir = storage_path.join("temp");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    // Generate a unique temporary filename
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let temp_file_name = format!("{}_{}", timestamp, file_name);
    let temp_file_path = temp_dir.join(&temp_file_name);
    
    fs::write(&temp_file_path, &file_data)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    
    Ok(temp_file_path.to_string_lossy().to_string())
}

#[command]
pub async fn cleanup_temp_files() -> Result<u32, String> {
    let storage_path = get_storage_path()
        .map_err(|e| format!("Failed to get storage path: {}", e))?;
    
    let temp_dir = storage_path.join("temp");
    
    if !temp_dir.exists() {
        return Ok(0);
    }
    
    let entries = fs::read_dir(&temp_dir)
        .map_err(|e| format!("Failed to read temp directory: {}", e))?;
    
    let mut cleaned_count = 0;
    let now = std::time::SystemTime::now();
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                // Clean up files older than 1 hour
                if let Ok(duration) = now.duration_since(modified) {
                    if duration.as_secs() > 3600 {
                        if fs::remove_file(entry.path()).is_ok() {
                            cleaned_count += 1;
                        }
                    }
                }
            }
        }
    }
    
    Ok(cleaned_count)
}