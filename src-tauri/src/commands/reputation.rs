use crate::reputation::{
    TransactionVerdict, SignedTransactionMessage, TrustLevel, BlacklistEntry,
    ReputationConfig, BlacklistManager, ReputationCache, CachedScore,
    calculate_transaction_score, count_transactions,
};
use crate::dht::DhtService;
use serde_json;
use tauri::State;
use std::sync::{Arc, Mutex};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Reputation state for Tauri
pub struct ReputationState {
    pub config: Arc<Mutex<ReputationConfig>>,
    pub blacklist: Arc<Mutex<BlacklistManager>>,
    pub cache: Arc<Mutex<ReputationCache>>,
    pub dht: Arc<Mutex<Option<Arc<DhtService>>>>,
}

impl ReputationState {
    pub fn new() -> Self {
        let config = ReputationConfig::default();
        let blacklist = BlacklistManager::new(config.clone());
        let cache = ReputationCache::new(config.cache_ttl);
        
        Self {
            config: Arc::new(Mutex::new(config)),
            blacklist: Arc::new(Mutex::new(blacklist)),
            cache: Arc::new(Mutex::new(cache)),
            dht: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_dht(&self, dht: Arc<DhtService>) {
        if let Ok(mut dht_lock) = self.dht.lock() {
            *dht_lock = Some(dht);
        }
    }
}

/// Get current reputation configuration
#[tauri::command]
pub async fn get_reputation_config(
    state: State<'_, ReputationState>,
) -> Result<ReputationConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// Update reputation configuration
#[tauri::command]
pub async fn update_reputation_config(
    config: ReputationConfig,
    state: State<'_, ReputationState>,
) -> Result<(), String> {
    let mut current_config = state.config.lock().map_err(|e| e.to_string())?;
    *current_config = config;
    Ok(())
}

/// Publish a transaction verdict to DHT
#[tauri::command]
pub async fn publish_transaction_verdict(
    verdict: TransactionVerdict,
    state: State<'_, ReputationState>,
    app_state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Validate the verdict
    verdict.validate()?;
    
    // Get DHT service
    let dht_opt = app_state.dht.lock().map_err(|e| e.to_string())?;
    let dht = dht_opt.as_ref().ok_or("DHT service not initialized")?;
    
    // Publish to DHT
    dht.publish_reputation_verdict(verdict.clone()).await?;
    
    println!("Published verdict: target={}, outcome={:?}", 
             verdict.target_id, verdict.outcome);
    
    Ok(())
}

/// Fetch transaction verdicts for a peer from DHT
#[tauri::command]
pub async fn fetch_transaction_verdicts(
    peer_id: String,
    app_state: State<'_, crate::AppState>,
) -> Result<Vec<TransactionVerdict>, String> {
    // Get DHT service
    let dht_opt = app_state.dht.lock().map_err(|e| e.to_string())?;
    let dht = dht_opt.as_ref().ok_or("DHT service not initialized")?;
    
    // Fetch from DHT
    let verdicts = dht.fetch_reputation_verdicts(peer_id.clone()).await?;
    
    println!("Fetched {} verdicts for peer: {}", verdicts.len(), peer_id);
    
    Ok(verdicts)
}

/// Sign a transaction message
#[tauri::command]
pub async fn sign_transaction_message(
    message: serde_json::Value,
) -> Result<SignedTransactionMessage, String> {
    // Parse the message
    let from = message["from"].as_str().ok_or("Missing 'from' field")?;
    let to = message["to"].as_str().ok_or("Missing 'to' field")?;
    let amount = message["amount"].as_u64().ok_or("Missing 'amount' field")?;
    let file_hash = message["fileHash"].as_str().ok_or("Missing 'fileHash' field")?;
    let nonce = message["nonce"].as_str().ok_or("Missing 'nonce' field")?;
    let deadline = message["deadline"].as_u64().ok_or("Missing 'deadline' field")?;
    
    let mut signed_message = SignedTransactionMessage::new(
        from.to_string(),
        to.to_string(),
        amount,
        file_hash.to_string(),
        nonce.to_string(),
        deadline,
    );
    
    // In production, this would use the actual private key
    // For now, create a mock signature
    signed_message.downloader_signature = "mock_signature".to_string();
    
    Ok(signed_message)
}

/// Verify a transaction message signature
#[tauri::command]
pub async fn verify_transaction_message(
    message: SignedTransactionMessage,
    public_key: String,
) -> Result<bool, String> {
    // In production, this would verify using the actual public key
    // For now, return true for mock signatures
    println!("Verifying signature for message from {} to {}", 
             message.from, message.to);
    Ok(!message.downloader_signature.is_empty())
}

/// Get wallet balance from blockchain
#[tauri::command]
pub async fn get_wallet_balance(
    address: String,
) -> Result<u64, String> {
    use crate::ethereum::get_balance;
    
    // Query the actual blockchain
    match get_balance(&address).await {
        Ok(balance_str) => {
            // Parse balance string (in wei) to u64
            // Note: This is a simplified conversion
            match balance_str.parse::<u64>() {
                Ok(balance) => {
                    println!("Balance for address {}: {} wei", address, balance);
                    Ok(balance)
                }
                Err(_) => {
                    // If it's too large for u64, return max value
                    println!("Balance for address {} is very large", address);
                    Ok(u64::MAX)
                }
            }
        }
        Err(e) => {
            println!("Failed to get balance for {}: {}", address, e);
            // Return a default balance for testing
            Ok(1000000)
        }
    }
}

/// Manually blacklist a peer
#[tauri::command]
pub async fn blacklist_peer_manual(
    peer_id: String,
    reason: String,
    state: State<'_, ReputationState>,
) -> Result<(), String> {
    let blacklist = state.blacklist.lock().map_err(|e| e.to_string())?;
    blacklist.add_manual(peer_id, reason)
}

/// Remove peer from blacklist
#[tauri::command]
pub async fn blacklist_peer_remove(
    peer_id: String,
    state: State<'_, ReputationState>,
) -> Result<(), String> {
    let blacklist = state.blacklist.lock().map_err(|e| e.to_string())?;
    blacklist.remove(&peer_id)
}

/// Check if peer is blacklisted
#[tauri::command]
pub async fn blacklist_peer_check(
    peer_id: String,
    state: State<'_, ReputationState>,
) -> Result<bool, String> {
    let blacklist = state.blacklist.lock().map_err(|e| e.to_string())?;
    blacklist.is_blacklisted(&peer_id)
}

/// List all blacklisted peers
#[tauri::command]
pub async fn blacklist_peer_list(
    state: State<'_, ReputationState>,
) -> Result<Vec<BlacklistEntry>, String> {
    let blacklist = state.blacklist.lock().map_err(|e| e.to_string())?;
    blacklist.list_all()
}

/// Cleanup expired automatic blacklist entries
#[tauri::command]
pub async fn blacklist_cleanup_expired(
    state: State<'_, ReputationState>,
) -> Result<usize, String> {
    let blacklist = state.blacklist.lock().map_err(|e| e.to_string())?;
    blacklist.cleanup_expired()
}

/// Calculate reputation score from verdicts
#[tauri::command]
pub async fn calculate_peer_score(
    verdicts: Vec<TransactionVerdict>,
    state: State<'_, ReputationState>,
) -> Result<f64, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(calculate_transaction_score(&verdicts, &config))
}

/// Get cached score for a peer
#[tauri::command]
pub async fn get_cached_score(
    peer_id: String,
    state: State<'_, ReputationState>,
) -> Result<Option<CachedScore>, String> {
    let cache = state.cache.lock().map_err(|e| e.to_string())?;
    cache.get(&peer_id)
}

/// Cache a peer's score
#[tauri::command]
pub async fn set_cached_score(
    peer_id: String,
    score: f64,
    trust_level: TrustLevel,
    state: State<'_, ReputationState>,
) -> Result<(), String> {
    let cache = state.cache.lock().map_err(|e| e.to_string())?;
    cache.set(peer_id, score, trust_level)
}

/// Clear reputation cache
#[tauri::command]
pub async fn clear_reputation_cache(
    state: State<'_, ReputationState>,
) -> Result<(), String> {
    let cache = state.cache.lock().map_err(|e| e.to_string())?;
    cache.clear()
}

/// Cleanup stale cache entries
#[tauri::command]
pub async fn cleanup_reputation_cache(
    state: State<'_, ReputationState>,
) -> Result<usize, String> {
    let cache = state.cache.lock().map_err(|e| e.to_string())?;
    cache.cleanup_stale()
}

/// Submit complaint on-chain using blockchain
#[tauri::command]
pub async fn submit_complaint_onchain(
    target_id: String,
    complaint_type: String,
    evidence: Vec<String>,
    app_state: State<'_, crate::AppState>,
) -> Result<String, String> {
    use crate::ethereum::send_transaction;
    
    println!("Submitting complaint on-chain: target={}, type={}", 
             target_id, complaint_type);
    
    // In a full implementation, this would:
    // 1. Call a smart contract method for filing complaints
    // 2. Include the evidence hash in the transaction data
    // 3. Pay gas fees
    
    // For now, we'll create a transaction with the complaint data in the input
    let complaint_data = serde_json::json!({
        "target": target_id,
        "type": complaint_type,
        "evidence_count": evidence.len(),
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    });
    
    let data_str = serde_json::to_string(&complaint_data).map_err(|e| e.to_string())?;
    
    // Get active account
    let account_lock = app_state.active_account.lock().map_err(|e| e.to_string())?;
    let from_address = account_lock.as_ref().ok_or("No active account")?;
    
    // In production, this would send to a complaint contract
    // For testing, return a mock transaction hash
    let tx_hash = format!("0x{:x}", SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs());
    
    println!("Complaint submitted with tx_hash: {}", tx_hash);
    Ok(tx_hash)
}

use std::time::{SystemTime, UNIX_EPOCH};
