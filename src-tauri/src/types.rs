// Shared type definitions for the application
use crate::analytics;
use crate::dht::DhtService;
use crate::ethereum::GethProcess;
use crate::file_transfer::FileTransferService;
use crate::geth_downloader::GethDownloader;
use crate::keystore::Keystore;
use crate::multi_source_download::MultiSourceDownloadService;
use crate::stream_auth::StreamAuthService;
use crate::webrtc_service::WebRTCService;
use crate::commands::proxy::ProxyNode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAuthToken {
    pub token: String,
    pub proxy_address: String,
    pub expires_at: u64,
    pub created_at: u64,
}

#[derive(Clone, Debug)]
pub struct StreamingUploadSession {
    pub file_name: String,
    pub file_size: u64,
    pub received_chunks: u32,
    pub total_chunks: u32,
    pub hasher: sha2::Sha256,
    pub created_at: std::time::SystemTime,
    pub chunk_cids: Vec<String>,
    pub file_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedTransaction {
    pub id: String,
    pub to_address: String,
    pub amount: f64,
    pub timestamp: u64,
}

pub struct AppState {
    pub geth: Mutex<GethProcess>,
    pub downloader: Arc<GethDownloader>,
    pub miner_address: Mutex<Option<String>>,

    // Wrap in Arc so they can be cloned
    pub active_account: Arc<Mutex<Option<String>>>,
    pub active_account_private_key: Arc<Mutex<Option<String>>>,

    pub rpc_url: Mutex<String>,
    pub dht: Mutex<Option<Arc<DhtService>>>,
    pub file_transfer: Mutex<Option<Arc<FileTransferService>>>,
    pub webrtc: Mutex<Option<Arc<WebRTCService>>>,
    pub multi_source_download: Mutex<Option<Arc<MultiSourceDownloadService>>>,
    pub keystore: Arc<Mutex<Keystore>>,
    pub proxies: Arc<Mutex<Vec<ProxyNode>>>,
    pub privacy_proxies: Arc<Mutex<Vec<String>>>,
    pub file_transfer_pump: Mutex<Option<JoinHandle<()>>>,
    pub multi_source_pump: Mutex<Option<JoinHandle<()>>>,
    pub socks5_proxy_cli: Mutex<Option<String>>,
    pub analytics: Arc<analytics::AnalyticsService>,

    // New fields for transaction queue
    pub transaction_queue: Arc<Mutex<VecDeque<QueuedTransaction>>>,
    pub transaction_processor: Mutex<Option<JoinHandle<()>>>,
    pub processing_transaction: Arc<Mutex<bool>>,

    // New field for streaming upload sessions
    pub upload_sessions: Arc<Mutex<HashMap<String, StreamingUploadSession>>>,

    // Proxy authentication tokens storage
    pub proxy_auth_tokens: Arc<Mutex<HashMap<String, ProxyAuthToken>>>,

    // Stream authentication service
    pub stream_auth: Arc<Mutex<StreamAuthService>>,
}
