use sha2::{Sha256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, OsRng};
use std::fs::{File, self, OpenOptions};
use std::io::{Read, Error, Write, BufReader, BufWriter, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use x25519_dalek::PublicKey;
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use fs2::FileExt; // For file locking

// Import the new crypto functions and the bundle struct
use crate::crypto::{encrypt_aes_key, EncryptedAesKeyBundle};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChunkInfo {
    pub index: u32,
    pub hash: String,
    pub size: usize,
    pub encrypted_size: usize,
    pub offset: u64, // Position in original file
}

/// File manifest containing metadata and ordered list of chunks
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileManifest {
    pub version: String,
    pub file_hash: String,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: Option<String>,
    pub chunk_size: usize,
    pub total_chunks: u32,
    pub chunks: Vec<ChunkInfo>,
    pub encryption: Option<EncryptionInfo>,
    pub timestamps: TimestampInfo,
    pub manifest_hash: String, // Self-referential hash for integrity
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptionInfo {
    pub algorithm: String,
    pub encrypted_key_bundle: Option<EncryptedAesKeyBundle>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimestampInfo {
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
}

pub struct ChunkManager {
    chunk_size: usize,
    storage_path: PathBuf,
    chunks_dir: PathBuf,
    manifests_dir: PathBuf,
}

impl ChunkManager {
    pub fn new(storage_path: PathBuf) -> Self {
        let chunks_dir = storage_path.join("chunks");
        let manifests_dir = storage_path.join("manifests");
        
        ChunkManager {
            chunk_size: 1024 * 1024, // 1MB default as specified in the issue
            storage_path,
            chunks_dir,
            manifests_dir,
        }
    }

    /// Set a custom chunk size (must be called before chunking operations)
    pub fn set_chunk_size(&mut self, size: usize) {
        self.chunk_size = size;
    }

    /// Ensure storage directories exist
    fn ensure_storage_dirs(&self) -> Result<(), Error> {
        fs::create_dir_all(&self.chunks_dir)?;
        fs::create_dir_all(&self.manifests_dir)?;
        Ok(())
    }

    /// Create file manifest and store chunks with content-addressed storage
    pub fn store_file_with_manifest(
        &self,
        file_path: &Path,
        recipient_public_key: Option<&PublicKey>,
    ) -> Result<FileManifest, String> {
        self.ensure_storage_dirs().map_err(|e| e.to_string())?;

        // Calculate overall file hash first
        let file_hash = self.hash_file(file_path).map_err(|e| e.to_string())?;
        
        // Generate AES key if encryption is requested
        let (aes_key, encryption_info) = if let Some(pub_key) = recipient_public_key {
            let mut key_bytes = [0u8; 32];
            OsRng.fill_bytes(&mut key_bytes);
            let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
            let encrypted_bundle = encrypt_aes_key(&key_bytes, pub_key)?;
            
            (Some(*key), Some(EncryptionInfo {
                algorithm: "AES-256-GCM".to_string(),
                encrypted_key_bundle: Some(encrypted_bundle),
            }))
        } else {
            (None, None)
        };

        // Process file in chunks with streaming
        let mut file = File::open(file_path).map_err(|e| e.to_string())?;
        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; self.chunk_size];
        let mut index = 0;
        let mut offset = 0u64;

        loop {
            let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 { break; }

            let chunk_data = &buffer[..bytes_read];
            let chunk_hash = self.hash_chunk(chunk_data);
            
            // Check if chunk already exists (deduplication)
            let chunk_path = self.get_chunk_path(&chunk_hash);
            let (encrypted_size, data_to_store) = if chunk_path.exists() {
                // Chunk already exists, just get the size
                let existing_size = fs::metadata(&chunk_path)
                    .map_err(|e| e.to_string())?
                    .len() as usize;
                (existing_size, None)
            } else {
                // New chunk, process and store it
                let data = if let Some(key) = &aes_key {
                    self.encrypt_chunk(chunk_data, key)?
                } else {
                    chunk_data.to_vec()
                };
                let size = data.len();
                (size, Some(data))
            };

            chunks.push(ChunkInfo {
                index,
                hash: chunk_hash.clone(),
                size: bytes_read,
                encrypted_size,
                offset,
            });

            // Store chunk atomically if it's new
            if let Some(data) = data_to_store {
                self.save_chunk_atomic(&chunk_hash, &data)?;
            }

            index += 1;
            offset += bytes_read as u64;
        }

        // Create file manifest
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut manifest = FileManifest {
            version: "1.0".to_string(),
            file_hash: file_hash.clone(),
            file_name: file_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_size: offset,
            mime_type: self.detect_mime_type(file_path),
            chunk_size: self.chunk_size,
            total_chunks: chunks.len() as u32,
            chunks,
            encryption: encryption_info,
            timestamps: TimestampInfo {
                created: now,
                modified: now,
                accessed: now,
            },
            manifest_hash: String::new(), // Will be calculated below
        };

        // Calculate manifest hash (excluding the hash field itself)
        manifest.manifest_hash = self.calculate_manifest_hash(&manifest)?;

        // Save manifest
        self.save_manifest(&file_hash, &manifest)?;

        Ok(manifest)
    }

    /// Reconstruct file from chunks with streaming and verification
    pub fn reconstruct_file(
        &self,
        manifest: &FileManifest,
        output_path: &Path,
        decryption_key: Option<&[u8; 32]>,
    ) -> Result<(), String> {
        // Verify manifest integrity first
        let calculated_hash = self.calculate_manifest_hash(manifest)?;
        if calculated_hash != manifest.manifest_hash {
            return Err("Manifest integrity check failed".to_string());
        }

        // Create output file with locking
        let output_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;
        
        // Lock file for exclusive access
        output_file.lock_exclusive()
            .map_err(|e| format!("Failed to lock output file: {}", e))?;

        let mut writer = BufWriter::new(output_file);
        let mut file_hasher = Sha256::new();

        // Process chunks in order
        for chunk_info in &manifest.chunks {
            let chunk_data = self.load_chunk(&chunk_info.hash)?;
            
            // Decrypt if necessary
            let plain_data = if let Some(key) = decryption_key {
                if manifest.encryption.is_some() {
                    self.decrypt_chunk(&chunk_data, key)?
                } else {
                    chunk_data
                }
            } else if manifest.encryption.is_some() {
                return Err("File is encrypted but no decryption key provided".to_string());
            } else {
                chunk_data
            };

            // Verify chunk integrity
            let calculated_hash = self.hash_chunk(&plain_data);
            if calculated_hash != chunk_info.hash {
                return Err(format!("Chunk {} integrity check failed", chunk_info.index));
            }

            // Verify chunk size matches
            if plain_data.len() != chunk_info.size {
                return Err(format!("Chunk {} size mismatch", chunk_info.index));
            }

            // Write to file and update file hash
            writer.write_all(&plain_data)
                .map_err(|e| format!("Failed to write chunk {}: {}", chunk_info.index, e))?;
            file_hasher.update(&plain_data);
        }

        writer.flush()
            .map_err(|e| format!("Failed to flush output file: {}", e))?;

        // Verify reconstructed file hash
        let final_hash = format!("{:x}", file_hasher.finalize());
        if final_hash != manifest.file_hash {
            return Err("Reconstructed file hash does not match manifest".to_string());
        }

        Ok(())
    }

    /// Load manifest from storage
    pub fn load_manifest(&self, file_hash: &str) -> Result<FileManifest, String> {
        let manifest_path = self.manifests_dir.join(format!("{}.json", file_hash));
        let manifest_data = fs::read_to_string(manifest_path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        
        let manifest: FileManifest = serde_json::from_str(&manifest_data)
            .map_err(|e| format!("Failed to parse manifest: {}", e))?;
        
        // Verify manifest integrity
        let calculated_hash = self.calculate_manifest_hash(&manifest)?;
        if calculated_hash != manifest.manifest_hash {
            return Err("Manifest integrity verification failed".to_string());
        }
        
        Ok(manifest)
    }

    /// Check if all chunks for a file are available locally
    pub fn verify_chunks_available(&self, manifest: &FileManifest) -> Result<Vec<String>, String> {
        let mut missing_chunks = Vec::new();
        
        for chunk_info in &manifest.chunks {
            let chunk_path = self.get_chunk_path(&chunk_info.hash);
            if !chunk_path.exists() {
                missing_chunks.push(chunk_info.hash.clone());
            }
        }
        
        Ok(missing_chunks)
    }

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> Result<StorageStats, String> {
        let chunks_count = self.count_files_in_dir(&self.chunks_dir)?;
        let manifests_count = self.count_files_in_dir(&self.manifests_dir)?;
        
        let chunks_size = self.calculate_dir_size(&self.chunks_dir)?;
        let manifests_size = self.calculate_dir_size(&self.manifests_dir)?;
        
        Ok(StorageStats {
            total_chunks: chunks_count,
            total_manifests: manifests_count,
            chunks_storage_bytes: chunks_size,
            manifests_storage_bytes: manifests_size,
            total_storage_bytes: chunks_size + manifests_size,
        })
    }

    // The function now takes the recipient's public key and returns the encrypted key bundle
    pub fn chunk_and_encrypt_file(
        &self,
        file_path: &Path,
        recipient_public_key: &PublicKey,
    ) -> Result<(Vec<ChunkInfo>, EncryptedAesKeyBundle), String> {
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);

        let mut file = File::open(file_path).map_err(|e| e.to_string())?;
        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; self.chunk_size];
        let mut index = 0;

        loop {
            let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 { break; }

            let chunk_data = &buffer[..bytes_read];
            let chunk_hash = self.hash_chunk(chunk_data);
            
            // The nonce is now prepended to the ciphertext by `encrypt_chunk`
            let encrypted_data_with_nonce = self.encrypt_chunk(chunk_data, &key)?;

            chunks.push(ChunkInfo {
                index,
                hash: chunk_hash.clone(),
                size: bytes_read,
                encrypted_size: encrypted_data_with_nonce.len(),
                offset: (index as usize * self.chunk_size) as u64,
            });

            self.save_chunk(&chunk_hash, &encrypted_data_with_nonce).map_err(|e| e.to_string())?;
            index += 1;
        }

        // Instead of returning the raw key, encrypt it with the recipient's public key
        let encrypted_key_bundle = encrypt_aes_key(&key_bytes, recipient_public_key)?;

        Ok((chunks, encrypted_key_bundle))
    }

    // Helper methods for the new content-addressed storage system

    /// Get the storage path for a chunk based on its hash
    fn get_chunk_path(&self, chunk_hash: &str) -> PathBuf {
        // Use the first 2 characters for subdirectory to avoid too many files in one dir
        let subdir = &chunk_hash[..2.min(chunk_hash.len())];
        let subdir_path = self.chunks_dir.join(subdir);
        subdir_path.join(chunk_hash)
    }

    /// Save a chunk atomically to prevent corruption
    fn save_chunk_atomic(&self, hash: &str, data: &[u8]) -> Result<(), String> {
        let chunk_path = self.get_chunk_path(hash);
        
        // Ensure subdirectory exists
        if let Some(parent) = chunk_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Write to temporary file first, then atomically rename
        let temp_path = chunk_path.with_extension("tmp");
        let temp_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        // Lock the temporary file
        temp_file.lock_exclusive()
            .map_err(|e| format!("Failed to lock temp file: {}", e))?;

        {
            let mut writer = BufWriter::new(&temp_file);
            writer.write_all(data)
                .map_err(|e| format!("Failed to write chunk data: {}", e))?;
            writer.flush()
                .map_err(|e| format!("Failed to flush chunk data: {}", e))?;
        }

        // Atomic rename
        fs::rename(&temp_path, &chunk_path)
            .map_err(|e| format!("Failed to atomically save chunk: {}", e))?;

        Ok(())
    }

    /// Load a chunk from storage
    fn load_chunk(&self, chunk_hash: &str) -> Result<Vec<u8>, String> {
        let chunk_path = self.get_chunk_path(chunk_hash);
        fs::read(&chunk_path)
            .map_err(|e| format!("Failed to load chunk {}: {}", chunk_hash, e))
    }

    /// Save manifest to storage  
    fn save_manifest(&self, file_hash: &str, manifest: &FileManifest) -> Result<(), String> {
        let manifest_path = self.manifests_dir.join(format!("{}.json", file_hash));
        let manifest_json = serde_json::to_string_pretty(manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

        // Write atomically
        let temp_path = manifest_path.with_extension("tmp");
        fs::write(&temp_path, manifest_json)
            .map_err(|e| format!("Failed to write manifest: {}", e))?;
        
        fs::rename(&temp_path, &manifest_path)
            .map_err(|e| format!("Failed to atomically save manifest: {}", e))?;

        Ok(())
    }

    /// Calculate hash of manifest (excluding the hash field itself)
    fn calculate_manifest_hash(&self, manifest: &FileManifest) -> Result<String, String> {
        let mut manifest_copy = manifest.clone();
        manifest_copy.manifest_hash = String::new();
        
        let manifest_json = serde_json::to_string(&manifest_copy)
            .map_err(|e| format!("Failed to serialize manifest for hashing: {}", e))?;
        
        let mut hasher = Sha256::new();
        hasher.update(manifest_json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Detect MIME type from file extension (basic implementation)
    fn detect_mime_type(&self, file_path: &Path) -> Option<String> {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "txt" => "text/plain",
                "pdf" => "application/pdf",
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "mp4" => "video/mp4",
                "zip" => "application/zip",
                "json" => "application/json",
                _ => "application/octet-stream",
            })
            .map(|s| s.to_string())
    }

    /// Count files in a directory
    fn count_files_in_dir(&self, dir: &Path) -> Result<usize, String> {
        if !dir.exists() {
            return Ok(0);
        }
        
        let mut count = 0;
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
            
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            if entry.file_type().map_err(|e| format!("Failed to get file type: {}", e))?.is_file() {
                count += 1;
            } else if entry.file_type().map_err(|e| format!("Failed to get file type: {}", e))?.is_dir() {
                // Recursively count files in subdirectories
                count += self.count_files_in_dir(&entry.path())?;
            }
        }
        
        Ok(count)
    }

    /// Calculate total size of files in a directory
    fn calculate_dir_size(&self, dir: &Path) -> Result<u64, String> {
        if !dir.exists() {
            return Ok(0);
        }
        
        let mut total_size = 0;
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
            
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let metadata = entry.metadata().map_err(|e| format!("Failed to get metadata: {}", e))?;
            
            if metadata.is_file() {
                total_size += metadata.len();
            } else if metadata.is_dir() {
                total_size += self.calculate_dir_size(&entry.path())?;
            }
        }
        
        Ok(total_size)
    }

    /// Decrypt chunk data
    fn decrypt_chunk(&self, encrypted_data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
        if encrypted_data.len() < 12 {
            return Err("Encrypted data too short to contain nonce".to_string());
        }

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))
    }

    // This function now returns the nonce and ciphertext combined for easier storage
    fn encrypt_chunk(&self, data: &[u8], key: &Key<Aes256Gcm>) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // Generate a unique nonce for each chunk

        let ciphertext = cipher.encrypt(&nonce, data).map_err(|e| e.to_string())?;
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    fn hash_chunk(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    // This function now saves the combined [nonce][ciphertext] blob
    fn save_chunk(&self, hash: &str, data_with_nonce: &[u8]) -> Result<(), Error> {
        fs::create_dir_all(&self.storage_path)?;
        fs::write(self.storage_path.join(hash), data_with_nonce)?;
        Ok(())
    }

    pub fn hash_file(&self, file_path: &Path) -> Result<String, Error> {
        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 1024 * 1024]; // 1MB buffer on the heap

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Storage statistics for monitoring
#[derive(Serialize, Deserialize, Debug)]
pub struct StorageStats {
    pub total_chunks: usize,
    pub total_manifests: usize,
    pub chunks_storage_bytes: u64,
    pub manifests_storage_bytes: u64,
    pub total_storage_bytes: u64,
}