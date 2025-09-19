use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::fs;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info};
use directories::ProjectDirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRequest {
    pub file_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResponse {
    pub file_data: Vec<u8>,
    pub file_name: String,
    pub file_size: u64,
}

// Simplified file transfer service without complex libp2p request-response
// This provides basic file storage and retrieval functionality

#[derive(Debug)]
pub enum FileTransferCommand {
    UploadFile {
        file_path: String,
        file_name: String,
    },
    DownloadFile {
        file_hash: String,
        output_path: String,
    },
    GetStoredFiles,
}

#[derive(Debug, Clone)]
pub enum FileTransferEvent {
    FileUploaded {
        file_hash: String,
        file_name: String,
    },
    FileDownloaded {
        file_path: String,
    },
    FileNotFound {
        file_hash: String,
    },
    Error {
        message: String,
    },
}

/// File metadata for persistent storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileMetadata {
    hash: String,
    name: String,
    size: u64,
    upload_date: u64, // Unix timestamp
}

/// Get the storage directory path for the application
fn get_storage_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "chiral-network", "chiral-network") {
        let storage_dir = proj_dirs.data_dir().join("files");
        if let Err(e) = fs::create_dir_all(&storage_dir) {
            error!("Failed to create storage directory: {}", e);
        }
        storage_dir
    } else {
        // Fallback to current directory if project dirs are not available
        let fallback = PathBuf::from("./chiral_files");
        if let Err(e) = fs::create_dir_all(&fallback) {
            error!("Failed to create fallback storage directory: {}", e);
        }
        fallback
    }
}

/// Get the metadata file path
fn get_metadata_path() -> PathBuf {
    get_storage_path().join("metadata.json")
}

/// Load file metadata from disk
fn load_file_metadata() -> HashMap<String, FileMetadata> {
    let metadata_path = get_metadata_path();
    if metadata_path.exists() {
        match fs::read_to_string(&metadata_path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<FileMetadata>>(&content) {
                    Ok(metadata_list) => {
                        let mut metadata_map = HashMap::new();
                        for metadata in metadata_list {
                            metadata_map.insert(metadata.hash.clone(), metadata);
                        }
                        info!("Loaded {} file metadata entries from disk", metadata_map.len());
                        metadata_map
                    }
                    Err(e) => {
                        error!("Failed to parse metadata file: {}", e);
                        HashMap::new()
                    }
                }
            }
            Err(e) => {
                error!("Failed to read metadata file: {}", e);
                HashMap::new()
            }
        }
    } else {
        info!("No existing metadata file found, starting with empty storage");
        HashMap::new()
    }
}

/// Save file metadata to disk
fn save_file_metadata(metadata_map: &HashMap<String, FileMetadata>) {
    let metadata_path = get_metadata_path();
    let metadata_list: Vec<FileMetadata> = metadata_map.values().cloned().collect();
    
    match serde_json::to_string_pretty(&metadata_list) {
        Ok(content) => {
            if let Err(e) = fs::write(&metadata_path, content) {
                error!("Failed to save metadata file: {}", e);
            } else {
                info!("Saved {} file metadata entries to disk", metadata_list.len());
            }
        }
        Err(e) => {
            error!("Failed to serialize metadata: {}", e);
        }
    }
}

pub struct FileTransferService {
    cmd_tx: mpsc::Sender<FileTransferCommand>,
    event_rx: Arc<Mutex<mpsc::Receiver<FileTransferEvent>>>,
    stored_files: Arc<Mutex<HashMap<String, (String, Vec<u8>, u64)>>>, // hash -> (name, data, size)
    file_metadata: Arc<Mutex<HashMap<String, FileMetadata>>>, // hash -> metadata
}

impl FileTransferService {
    pub async fn new() -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);
        let stored_files = Arc::new(Mutex::new(HashMap::new()));
        
        // Load existing file metadata from disk
        let file_metadata = Arc::new(Mutex::new(load_file_metadata()));
        
        // Load existing files from disk into memory
        {
            let metadata = file_metadata.lock().await;
            let mut files = stored_files.lock().await;
            
            for (hash, meta) in metadata.iter() {
                let file_path = get_storage_path().join(&hash);
                if file_path.exists() {
                    match fs::read(&file_path) {
                        Ok(data) => {
                            files.insert(hash.clone(), (meta.name.clone(), data, meta.size));
                            info!("Loaded file from disk: {} ({})", meta.name, hash);
                        }
                        Err(e) => {
                            error!("Failed to load file {}: {}", hash, e);
                        }
                    }
                } else {
                    error!("File not found on disk: {} ({})", meta.name, hash);
                }
            }
        }

        // Spawn the file transfer service task
        tokio::spawn(Self::run_file_transfer_service(
            cmd_rx,
            event_tx,
            stored_files.clone(),
            file_metadata.clone(),
        ));

        Ok(FileTransferService {
            cmd_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            stored_files,
            file_metadata,
        })
    }

    async fn run_file_transfer_service(
        mut cmd_rx: mpsc::Receiver<FileTransferCommand>,
        event_tx: mpsc::Sender<FileTransferEvent>,
        stored_files: Arc<Mutex<HashMap<String, (String, Vec<u8>, u64)>>>,
        file_metadata: Arc<Mutex<HashMap<String, FileMetadata>>>,
    ) {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                FileTransferCommand::UploadFile {
                    file_path,
                    file_name,
                } => match Self::handle_upload_file(&file_path, &file_name, &stored_files, &file_metadata).await {
                    Ok(file_hash) => {
                        let _ = event_tx
                            .send(FileTransferEvent::FileUploaded {
                                file_hash: file_hash.clone(),
                                file_name: file_name.clone(),
                            })
                            .await;
                        info!("File uploaded successfully: {} -> {}", file_name, file_hash);
                    }
                    Err(e) => {
                        let error_msg = format!("Upload failed: {}", e);
                        let _ = event_tx
                            .send(FileTransferEvent::Error {
                                message: error_msg.clone(),
                            })
                            .await;
                        error!("File upload failed: {}", error_msg);
                    }
                },
                FileTransferCommand::DownloadFile {
                    file_hash,
                    output_path,
                } => {
                    match Self::handle_download_file(&file_hash, &output_path, &stored_files, &file_metadata).await
                    {
                        Ok(()) => {
                            let _ = event_tx
                                .send(FileTransferEvent::FileDownloaded {
                                    file_path: output_path.clone(),
                                })
                                .await;
                            info!(
                                "File downloaded successfully: {} -> {}",
                                file_hash, output_path
                            );
                        }
                        Err(e) => {
                            let error_msg = format!("Download failed: {}", e);
                            let _ = event_tx
                                .send(FileTransferEvent::Error {
                                    message: error_msg.clone(),
                                })
                                .await;
                            error!("File download failed: {}", error_msg);
                        }
                    }
                }
                FileTransferCommand::GetStoredFiles => {
                    // This could be used to list available files
                    debug!("GetStoredFiles command received");
                }
            }
        }
    }

    async fn handle_upload_file(
        file_path: &str,
        file_name: &str,
        stored_files: &Arc<Mutex<HashMap<String, (String, Vec<u8>, u64)>>>,
        file_metadata: &Arc<Mutex<HashMap<String, FileMetadata>>>,
    ) -> Result<String, String> {
        // Read the file
        let file_data = tokio::fs::read(file_path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Calculate file hash
        let file_hash = Self::calculate_file_hash(&file_data);
        let file_size = file_data.len() as u64;
        let upload_date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Save file to persistent storage
        let storage_path = get_storage_path().join(&file_hash);
        fs::write(&storage_path, &file_data)
            .map_err(|e| format!("Failed to save file to disk: {}", e))?;

        // Store the file in memory for quick access
        {
            let mut files = stored_files.lock().await;
            files.insert(file_hash.clone(), (file_name.to_string(), file_data, file_size));
        }

        // Update metadata and save to disk
        {
            let mut metadata = file_metadata.lock().await;
            metadata.insert(file_hash.clone(), FileMetadata {
                hash: file_hash.clone(),
                name: file_name.to_string(),
                size: file_size,
                upload_date,
            });
            
            // Save metadata to disk
            save_file_metadata(&metadata);
        }

        info!("File saved to persistent storage: {} -> {}", file_name, file_hash);
        Ok(file_hash)
    }

    async fn handle_download_file(
        file_hash: &str,
        output_path: &str,
        stored_files: &Arc<Mutex<HashMap<String, (String, Vec<u8>, u64)>>>,
        _file_metadata: &Arc<Mutex<HashMap<String, FileMetadata>>>,
    ) -> Result<(), String> {
        // Check if we have the file locally
        let (file_name, file_data, _file_size) = {
            let files = stored_files.lock().await;
            files
                .get(file_hash)
                .ok_or_else(|| "File not found locally".to_string())?
                .clone()
        };

        // Write the file to the output path
        tokio::fs::write(output_path, file_data)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        info!("File downloaded: {} -> {}", file_name, output_path);
        Ok(())
    }

    pub fn calculate_file_hash(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    pub async fn upload_file(&self, file_path: String, file_name: String) -> Result<(), String> {
        self.cmd_tx
            .send(FileTransferCommand::UploadFile {
                file_path,
                file_name,
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn download_file(
        &self,
        file_hash: String,
        output_path: String,
    ) -> Result<(), String> {
        self.cmd_tx
            .send(FileTransferCommand::DownloadFile {
                file_hash,
                output_path,
            })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_stored_files(&self) -> Result<Vec<(String, String, u64)>, String> {
        let files = self.stored_files.lock().await;
        Ok(files
            .iter()
            .map(|(hash, (name, _, size))| (hash.clone(), name.clone(), *size))
            .collect())
    }

    pub async fn drain_events(&self, max: usize) -> Vec<FileTransferEvent> {
        let mut events = Vec::new();
        let mut event_rx = self.event_rx.lock().await;

        for _ in 0..max {
            match event_rx.try_recv() {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }

        events
    }

    pub async fn store_file_data(&self, file_hash: String, file_name: String, file_data: Vec<u8>) {
        let file_size = file_data.len() as u64;
        let upload_date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Save file to persistent storage
        let storage_path = get_storage_path().join(&file_hash);
        if let Err(e) = fs::write(&storage_path, &file_data) {
            error!("Failed to save file to disk: {}", e);
        } else {
            info!("File saved to persistent storage: {} -> {}", file_name, file_hash);
        }

        // Store the file in memory for quick access
        {
            let mut stored_files = self.stored_files.lock().await;
            stored_files.insert(file_hash.clone(), (file_name.clone(), file_data, file_size));
        }

        // Update metadata and save to disk
        {
            let mut metadata = self.file_metadata.lock().await;
            metadata.insert(file_hash.clone(), FileMetadata {
                hash: file_hash.clone(),
                name: file_name,
                size: file_size,
                upload_date,
            });
            
            // Save metadata to disk
            save_file_metadata(&metadata);
        }
    }
}
