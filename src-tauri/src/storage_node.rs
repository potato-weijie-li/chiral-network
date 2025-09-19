use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkUploadRequest {
    pub chunk_hash: String,
    pub chunk_data: Vec<u8>,
    pub file_hash: String,
    pub chunk_index: u32,
    pub payment_tx: Option<String>, // Transaction hash for payment
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkUploadResponse {
    pub success: bool,
    pub chunk_hash: String,
    pub storage_proof: String, // Proof that the chunk is stored
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub node_id: String,
    pub total_capacity: u64,
    pub used_capacity: u64,
    pub available_capacity: u64,
    pub stored_chunks: u64,
    pub uptime: f32,
    pub reputation: f32,
}

pub struct StorageNodeService {
    node_id: String,
    storage_path: PathBuf,
    stored_chunks: Arc<Mutex<HashMap<String, ChunkMetadata>>>, // chunk_hash -> metadata
    total_capacity: u64,
    used_capacity: Arc<Mutex<u64>>,
}

#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub file_hash: String,
    pub chunk_index: u32,
    pub size: u64,
    pub stored_at: u64,
    pub access_count: u64,
}

impl StorageNodeService {
    pub fn new(node_id: String, storage_path: PathBuf, capacity: u64) -> Self {
        // Ensure storage directory exists
        if let Err(e) = std::fs::create_dir_all(&storage_path) {
            error!("Failed to create storage directory: {}", e);
        }

        StorageNodeService {
            node_id,
            storage_path,
            stored_chunks: Arc::new(Mutex::new(HashMap::new())),
            total_capacity: capacity,
            used_capacity: Arc::new(Mutex::new(0)),
        }
    }

    /// Store a chunk from an upload request
    pub async fn store_chunk(&self, request: ChunkUploadRequest) -> Result<ChunkUploadResponse, String> {
        let chunk_size = request.chunk_data.len() as u64;
        
        // Check if we have enough space
        {
            let used = *self.used_capacity.lock().await;
            if used + chunk_size > self.total_capacity {
                return Err("Insufficient storage capacity".to_string());
            }
        }

        // Verify chunk hash
        let computed_hash = self.compute_chunk_hash(&request.chunk_data);
        if computed_hash != request.chunk_hash {
            return Err("Chunk hash verification failed".to_string());
        }

        // Store chunk to disk
        let chunk_path = self.storage_path.join(&request.chunk_hash);
        if let Err(e) = tokio::fs::write(&chunk_path, &request.chunk_data).await {
            return Err(format!("Failed to write chunk to disk: {}", e));
        }

        // Update metadata
        let metadata = ChunkMetadata {
            file_hash: request.file_hash.clone(),
            chunk_index: request.chunk_index,
            size: chunk_size,
            stored_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 0,
        };

        {
            let mut chunks = self.stored_chunks.lock().await;
            chunks.insert(request.chunk_hash.clone(), metadata);
            
            let mut used = self.used_capacity.lock().await;
            *used += chunk_size;
        }

        // Generate storage proof (simple hash-based proof)
        let storage_proof = self.generate_storage_proof(&request.chunk_hash, &request.chunk_data);

        info!("Stored chunk {} ({} bytes) for file {}", 
              request.chunk_hash, chunk_size, request.file_hash);

        Ok(ChunkUploadResponse {
            success: true,
            chunk_hash: request.chunk_hash,
            storage_proof,
            node_id: self.node_id.clone(),
        })
    }

    /// Retrieve a chunk by hash
    pub async fn retrieve_chunk(&self, chunk_hash: String) -> Result<Vec<u8>, String> {
        let chunk_path = self.storage_path.join(&chunk_hash);
        
        match tokio::fs::read(&chunk_path).await {
            Ok(data) => {
                // Update access count
                {
                    let mut chunks = self.stored_chunks.lock().await;
                    if let Some(metadata) = chunks.get_mut(&chunk_hash) {
                        metadata.access_count += 1;
                    }
                }
                
                info!("Retrieved chunk {} ({} bytes)", chunk_hash, data.len());
                Ok(data)
            }
            Err(e) => {
                warn!("Failed to retrieve chunk {}: {}", chunk_hash, e);
                Err(format!("Chunk not found: {}", e))
            }
        }
    }

    /// Verify that a chunk is still stored and accessible
    pub async fn verify_chunk(&self, chunk_hash: String) -> Result<bool, String> {
        let chunk_path = self.storage_path.join(&chunk_hash);
        
        if !chunk_path.exists() {
            return Ok(false);
        }

        // Verify the stored chunk matches its hash
        match tokio::fs::read(&chunk_path).await {
            Ok(data) => {
                let computed_hash = self.compute_chunk_hash(&data);
                Ok(computed_hash == chunk_hash)
            }
            Err(_) => Ok(false),
        }
    }

    /// Get storage statistics for this node
    pub async fn get_storage_stats(&self) -> StorageStats {
        let chunks = self.stored_chunks.lock().await;
        let used = *self.used_capacity.lock().await;

        StorageStats {
            node_id: self.node_id.clone(),
            total_capacity: self.total_capacity,
            used_capacity: used,
            available_capacity: self.total_capacity - used,
            stored_chunks: chunks.len() as u64,
            uptime: 0.99, // Mock uptime
            reputation: 4.5, // Mock reputation
        }
    }

    /// List all stored chunks
    pub async fn list_chunks(&self) -> Vec<String> {
        let chunks = self.stored_chunks.lock().await;
        chunks.keys().cloned().collect()
    }

    /// Delete a chunk
    pub async fn delete_chunk(&self, chunk_hash: String) -> Result<(), String> {
        let chunk_path = self.storage_path.join(&chunk_hash);
        
        // Get chunk size before deletion
        let chunk_size = {
            let chunks = self.stored_chunks.lock().await;
            chunks.get(&chunk_hash).map(|m| m.size).unwrap_or(0)
        };

        // Delete from disk
        if let Err(e) = tokio::fs::remove_file(&chunk_path).await {
            return Err(format!("Failed to delete chunk from disk: {}", e));
        }

        // Update metadata
        {
            let mut chunks = self.stored_chunks.lock().await;
            chunks.remove(&chunk_hash);
            
            let mut used = self.used_capacity.lock().await;
            *used = used.saturating_sub(chunk_size);
        }

        info!("Deleted chunk {} ({} bytes)", chunk_hash, chunk_size);
        Ok(())
    }

    /// Compute SHA-256 hash of chunk data
    fn compute_chunk_hash(&self, data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Generate a storage proof for a chunk
    fn generate_storage_proof(&self, chunk_hash: &str, data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(chunk_hash.as_bytes());
        hasher.update(data);
        hasher.update(self.node_id.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get chunks for a specific file
    pub async fn get_file_chunks(&self, file_hash: String) -> Vec<(String, u32)> {
        let chunks = self.stored_chunks.lock().await;
        chunks
            .iter()
            .filter(|(_, metadata)| metadata.file_hash == file_hash)
            .map(|(chunk_hash, metadata)| (chunk_hash.clone(), metadata.chunk_index))
            .collect()
    }
}