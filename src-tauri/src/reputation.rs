use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// CORE REPUTATION TYPES (Transaction-Backed System)
// ============================================================================

/// Transaction verdicts published to DHT to summarize an issuer's view of a
/// particular on-chain transaction involving `target_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VerdictOutcome {
    Good,
    Disputed,
    Bad,
}

/// TransactionVerdict is the core reputation primitive. Peers publish these
/// to the DHT after completing (or failing to complete) a file transfer transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionVerdict {
    /// Peer whose reputation is being updated
    pub target_id: String,
    /// Blockchain transaction hash (or null for non-payment complaints)
    pub tx_hash: Option<String>,
    /// Outcome: good, bad, or disputed
    pub outcome: VerdictOutcome,
    /// Optional plain-text metadata (≤ 1 KB)
    pub details: Option<String>,
    /// Optional metric label (defaults to "transaction")
    pub metric: Option<String>,
    /// Unix timestamp when verdict was issued
    pub issued_at: u64,
    /// Peer ID of the issuer
    pub issuer_id: String,
    /// Monotonic counter per issuer to prevent duplicates
    pub issuer_seq_no: u64,
    /// Hex-encoded signature over the canonical signable payload
    pub issuer_sig: String,
    /// Optional on-chain receipt pointer
    pub tx_receipt: Option<String>,
    /// Optional evidence blobs (critical for non-payment complaints)
    pub evidence_blobs: Option<Vec<String>>,
}

/// Container for storing multiple verdicts for a peer in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationRecord {
    /// Peer whose reputation this records
    pub target_id: String,
    /// All verdicts for this peer
    pub verdicts: Vec<TransactionVerdict>,
    /// Last updated timestamp
    pub last_updated: u64,
}

impl ReputationRecord {
    pub fn new(target_id: String) -> Self {
        Self {
            target_id,
            verdicts: Vec::new(),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn add_verdict(&mut self, verdict: TransactionVerdict) {
        // Check for duplicates (same issuer and seq_no)
        let is_duplicate = self.verdicts.iter().any(|v| {
            v.issuer_id == verdict.issuer_id && v.issuer_seq_no == verdict.issuer_seq_no
        });

        if !is_duplicate {
            self.verdicts.push(verdict);
            self.last_updated = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }
}

impl TransactionVerdict {
    /// Basic validation performed client-side before accepting a verdict.
    pub fn validate(&self) -> Result<(), String> {
        if self.issuer_id.is_empty() {
            return Err("issuer_id missing".into());
        }
        if self.target_id.is_empty() {
            return Err("target_id missing".into());
        }
        if self.issuer_id == self.target_id {
            return Err("issuer_id must not equal target_id".into());
        }
        // Details should be ≤ 1 KB
        if let Some(ref details) = self.details {
            if details.len() > 1024 {
                return Err("details exceed 1 KB limit".into());
            }
        }
        Ok(())
    }

    /// Compute the DHT key for this target's transaction verdicts: H(target_id || "tx-rep")
    pub fn dht_key_for_target(target_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(target_id.as_bytes());
        hasher.update(b"tx-rep");
        hex::encode(hasher.finalize())
    }

    /// Sign this verdict using the provided signing key
    pub fn sign_with(
        &mut self,
        signing_key: &SigningKey,
        issuer_id: &str,
        issuer_seq_no: u64,
    ) -> Result<(), String> {
        self.issuer_id = issuer_id.to_string();
        self.issuer_seq_no = issuer_seq_no;

        // Build deterministic signable payload with explicit field order
        let signable = serde_json::json!({
            "target_id": self.target_id,
            "tx_hash": self.tx_hash,
            "outcome": &self.outcome,
            "details": self.details,
            "metric": self.metric,
            "issued_at": self.issued_at,
            "issuer_id": self.issuer_id,
            "issuer_seq_no": self.issuer_seq_no,
            "tx_receipt": self.tx_receipt,
            "evidence_blobs": self.evidence_blobs,
        });

        let serialized = serde_json::to_vec(&signable).map_err(|e| e.to_string())?;
        let signature = signing_key.sign(&serialized);
        self.issuer_sig = hex::encode(signature.to_bytes());
        Ok(())
    }

    /// Verify the signature on this verdict
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> Result<bool, String> {
        let signable = serde_json::json!({
            "target_id": self.target_id,
            "tx_hash": self.tx_hash,
            "outcome": &self.outcome,
            "details": self.details,
            "metric": self.metric,
            "issued_at": self.issued_at,
            "issuer_id": self.issuer_id,
            "issuer_seq_no": self.issuer_seq_no,
            "tx_receipt": self.tx_receipt,
            "evidence_blobs": self.evidence_blobs,
        });

        let serialized = serde_json::to_vec(&signable).map_err(|e| e.to_string())?;
        let signature_bytes = hex::decode(&self.issuer_sig).map_err(|e| e.to_string())?;
        
        if signature_bytes.len() != 64 {
            return Err("invalid signature length".into());
        }
        
        let mut sig_array: [u8; 64] = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = Signature::from_bytes(&sig_array);

        Ok(verifying_key.verify(&serialized, &signature).is_ok())
    }
}

/// Signed transaction message - the payment promise sent off-chain during handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransactionMessage {
    /// Downloader's blockchain address
    pub from: String,
    /// Seeder's blockchain address
    pub to: String,
    /// Payment amount
    pub amount: u64,
    /// File hash being transferred
    pub file_hash: String,
    /// Unique identifier to prevent replay attacks
    pub nonce: String,
    /// Unix timestamp deadline for transfer completion
    pub deadline: u64,
    /// Cryptographic signature from downloader
    pub downloader_signature: String,
}

impl SignedTransactionMessage {
    /// Create a new signed transaction message
    pub fn new(
        from: String,
        to: String,
        amount: u64,
        file_hash: String,
        nonce: String,
        deadline: u64,
    ) -> Self {
        Self {
            from,
            to,
            amount,
            file_hash,
            nonce,
            deadline,
            downloader_signature: String::new(),
        }
    }

    /// Sign the message with downloader's private key
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<(), String> {
        let signable = serde_json::json!({
            "from": self.from,
            "to": self.to,
            "amount": self.amount,
            "file_hash": self.file_hash,
            "nonce": self.nonce,
            "deadline": self.deadline,
        });

        let serialized = serde_json::to_vec(&signable).map_err(|e| e.to_string())?;
        let signature = signing_key.sign(&serialized);
        self.downloader_signature = hex::encode(signature.to_bytes());
        Ok(())
    }

    /// Verify the signature on this message
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> Result<bool, String> {
        let signable = serde_json::json!({
            "from": self.from,
            "to": self.to,
            "amount": self.amount,
            "file_hash": self.file_hash,
            "nonce": self.nonce,
            "deadline": self.deadline,
        });

        let serialized = serde_json::to_vec(&signable).map_err(|e| e.to_string())?;
        let signature_bytes = hex::decode(&self.downloader_signature).map_err(|e| e.to_string())?;
        
        if signature_bytes.len() != 64 {
            return Err("invalid signature length".into());
        }
        
        let mut sig_array: [u8; 64] = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = Signature::from_bytes(&sig_array);

        Ok(verifying_key.verify(&serialized, &signature).is_ok())
    }
}

/// Trust levels based on transaction score ranges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TrustLevel {
    Unknown,  // 0.0 - 0.2
    Low,      // 0.2 - 0.4
    Medium,   // 0.4 - 0.6
    High,     // 0.6 - 0.8
    Trusted,  // 0.8 - 1.0
}

impl TrustLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.8 {
            TrustLevel::Trusted
        } else if score >= 0.6 {
            TrustLevel::High
        } else if score >= 0.4 {
            TrustLevel::Medium
        } else if score >= 0.2 {
            TrustLevel::Low
        } else {
            TrustLevel::Unknown
        }
    }

    pub fn min_score(&self) -> f64 {
        match self {
            TrustLevel::Trusted => 0.8,
            TrustLevel::High => 0.6,
            TrustLevel::Medium => 0.4,
            TrustLevel::Low => 0.2,
            TrustLevel::Unknown => 0.0,
        }
    }

    pub fn max_score(&self) -> f64 {
        match self {
            TrustLevel::Trusted => 1.0,
            TrustLevel::High => 0.8,
            TrustLevel::Medium => 0.6,
            TrustLevel::Low => 0.4,
            TrustLevel::Unknown => 0.2,
        }
    }
}

/// Blacklist entry for misbehaving peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistEntry {
    pub peer_id: String,
    pub reason: String,
    pub blacklisted_at: u64,
    pub is_automatic: bool,
    pub evidence: Option<Vec<String>>,
}

/// Configuration for reputation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Blocks required before transaction counts for reputation
    pub confirmation_threshold: u64,
    /// Max duration to keep verdict pending (seconds)
    pub confirmation_timeout: u64,
    /// Transactions needed to reach max base score
    pub maturity_threshold: u64,
    /// Half-life for time decay (days), 0 = disabled
    pub decay_half_life: u64,
    /// Duration to keep accepted verdicts (days)
    pub retention_period: u64,
    /// Max bytes in details field
    pub max_verdict_size: usize,
    /// Duration to cache scores locally (seconds)
    pub cache_ttl: u64,
    /// Blacklist mode: manual, automatic, hybrid
    pub blacklist_mode: String,
    /// Enable automatic blacklisting
    pub blacklist_auto_enabled: bool,
    /// Score threshold for auto-blacklist (0.0-1.0)
    pub blacklist_score_threshold: f64,
    /// Bad verdicts needed for auto-blacklist
    pub blacklist_bad_verdicts_threshold: u32,
    /// Blacklist retention period (days)
    pub blacklist_retention: u64,
    /// Default deadline for signed messages (seconds)
    pub payment_deadline_default: u64,
    /// Grace period after deadline (seconds)
    pub payment_grace_period: u64,
    /// Min balance as multiple of file price
    pub min_balance_multiplier: f64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            confirmation_threshold: 12,
            confirmation_timeout: 3600,
            maturity_threshold: 100,
            decay_half_life: 90,
            retention_period: 90,
            max_verdict_size: 1024,
            cache_ttl: 600,
            blacklist_mode: "hybrid".to_string(),
            blacklist_auto_enabled: true,
            blacklist_score_threshold: 0.2,
            blacklist_bad_verdicts_threshold: 3,
            blacklist_retention: 30,
            payment_deadline_default: 3600,
            payment_grace_period: 1800,
            min_balance_multiplier: 1.2,
        }
    }
}

// ============================================================================
// REPUTATION SCORING
// ============================================================================

/// Calculate weighted reputation score from verdicts
pub fn calculate_transaction_score(verdicts: &[TransactionVerdict], config: &ReputationConfig) -> f64 {
    if verdicts.is_empty() {
        return 0.0;
    }

    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;

    for verdict in verdicts {
        let value = match verdict.outcome {
            VerdictOutcome::Good => 1.0,
            VerdictOutcome::Disputed => 0.5,
            VerdictOutcome::Bad => 0.0,
        };

        let weight = if config.decay_half_life > 0 {
            calculate_time_decay_weight(verdict.issued_at, config.decay_half_life)
        } else {
            1.0
        };

        weighted_sum += value * weight;
        total_weight += weight;
    }

    if total_weight > 0.0 {
        weighted_sum / total_weight
    } else {
        0.0
    }
}

/// Calculate time decay weight using exponential decay
fn calculate_time_decay_weight(issued_at: u64, half_life_days: u64) -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let age_seconds = now.saturating_sub(issued_at);
    let age_days = age_seconds as f64 / 86400.0;
    let half_life = half_life_days as f64;
    
    0.5_f64.powf(age_days / half_life)
}

/// Count successful vs failed transactions
pub fn count_transactions(verdicts: &[TransactionVerdict]) -> (u32, u32) {
    let mut successful = 0;
    let mut failed = 0;

    for verdict in verdicts {
        match verdict.outcome {
            VerdictOutcome::Good => successful += 1,
            VerdictOutcome::Bad => failed += 1,
            VerdictOutcome::Disputed => {
                // Disputed counts as half success
                // This is a simplification; could be more nuanced
            }
        }
    }

    (successful, failed)
}

// ============================================================================
// BLACKLIST MANAGEMENT
// ============================================================================

pub struct BlacklistManager {
    entries: Arc<Mutex<HashMap<String, BlacklistEntry>>>,
    config: ReputationConfig,
}

impl BlacklistManager {
    pub fn new(config: ReputationConfig) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Manually blacklist a peer
    pub fn add_manual(&self, peer_id: String, reason: String) -> Result<(), String> {
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        entries.insert(
            peer_id.clone(),
            BlacklistEntry {
                peer_id,
                reason,
                blacklisted_at: now,
                is_automatic: false,
                evidence: None,
            },
        );

        Ok(())
    }

    /// Automatically blacklist based on score or verdict count
    pub fn check_auto_blacklist(
        &self,
        peer_id: &str,
        score: f64,
        bad_verdict_count: u32,
    ) -> Result<bool, String> {
        if !self.config.blacklist_auto_enabled {
            return Ok(false);
        }

        if self.config.blacklist_mode == "manual" {
            return Ok(false);
        }

        let should_blacklist = score < self.config.blacklist_score_threshold
            || bad_verdict_count >= self.config.blacklist_bad_verdicts_threshold;

        if should_blacklist {
            let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
            
            if !entries.contains_key(peer_id) {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let reason = if score < self.config.blacklist_score_threshold {
                    format!("Score {} below threshold {}", score, self.config.blacklist_score_threshold)
                } else {
                    format!("{} bad verdicts exceeds threshold {}", bad_verdict_count, self.config.blacklist_bad_verdicts_threshold)
                };

                entries.insert(
                    peer_id.to_string(),
                    BlacklistEntry {
                        peer_id: peer_id.to_string(),
                        reason,
                        blacklisted_at: now,
                        is_automatic: true,
                        evidence: None,
                    },
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if peer is blacklisted
    pub fn is_blacklisted(&self, peer_id: &str) -> Result<bool, String> {
        let entries = self.entries.lock().map_err(|e| e.to_string())?;
        Ok(entries.contains_key(peer_id))
    }

    /// Remove peer from blacklist
    pub fn remove(&self, peer_id: &str) -> Result<(), String> {
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        entries.remove(peer_id);
        Ok(())
    }

    /// Get all blacklist entries
    pub fn list_all(&self) -> Result<Vec<BlacklistEntry>, String> {
        let entries = self.entries.lock().map_err(|e| e.to_string())?;
        Ok(entries.values().cloned().collect())
    }

    /// Clean up expired automatic blacklist entries
    pub fn cleanup_expired(&self) -> Result<usize, String> {
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let retention_seconds = self.config.blacklist_retention * 86400;
        let mut removed = 0;

        entries.retain(|_, entry| {
            if entry.is_automatic {
                let age = now.saturating_sub(entry.blacklisted_at);
                if age > retention_seconds {
                    removed += 1;
                    return false;
                }
            }
            true
        });

        Ok(removed)
    }
}

// ============================================================================
// REPUTATION CACHE
// ============================================================================

#[derive(Debug, Clone)]
pub struct CachedScore {
    pub score: f64,
    pub trust_level: TrustLevel,
    pub cached_at: u64,
}

pub struct ReputationCache {
    scores: Arc<Mutex<HashMap<String, CachedScore>>>,
    ttl: u64,
}

impl ReputationCache {
    pub fn new(ttl: u64) -> Self {
        Self {
            scores: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    /// Get cached score if not stale
    pub fn get(&self, peer_id: &str) -> Result<Option<CachedScore>, String> {
        let scores = self.scores.lock().map_err(|e| e.to_string())?;
        
        if let Some(cached) = scores.get(peer_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let age = now.saturating_sub(cached.cached_at);
            if age < self.ttl {
                return Ok(Some(cached.clone()));
            }
        }

        Ok(None)
    }

    /// Cache a score
    pub fn set(&self, peer_id: String, score: f64, trust_level: TrustLevel) -> Result<(), String> {
        let mut scores = self.scores.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        scores.insert(
            peer_id,
            CachedScore {
                score,
                trust_level,
                cached_at: now,
            },
        );

        Ok(())
    }

    /// Clear cache
    pub fn clear(&self) -> Result<(), String> {
        let mut scores = self.scores.lock().map_err(|e| e.to_string())?;
        scores.clear();
        Ok(())
    }

    /// Remove stale entries
    pub fn cleanup_stale(&self) -> Result<usize, String> {
        let mut scores = self.scores.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let initial_count = scores.len();
        scores.retain(|_, cached| {
            let age = now.saturating_sub(cached.cached_at);
            age < self.ttl
        });

        Ok(initial_count - scores.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verdict_outcome_serialization() {
        let good = VerdictOutcome::Good;
        let serialized = serde_json::to_string(&good).unwrap();
        assert_eq!(serialized, "\"good\"");
    }

    #[test]
    fn test_trust_level_from_score() {
        assert_eq!(TrustLevel::from_score(0.9), TrustLevel::Trusted);
        assert_eq!(TrustLevel::from_score(0.7), TrustLevel::High);
        assert_eq!(TrustLevel::from_score(0.5), TrustLevel::Medium);
        assert_eq!(TrustLevel::from_score(0.3), TrustLevel::Low);
        assert_eq!(TrustLevel::from_score(0.1), TrustLevel::Unknown);
    }

    #[test]
    fn test_dht_key_computation() {
        let key = TransactionVerdict::dht_key_for_target("peer123");
        assert!(!key.is_empty());
        assert_eq!(key.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_transaction_score_calculation() {
        let config = ReputationConfig::default();
        
        let verdicts = vec![
            TransactionVerdict {
                target_id: "peer1".to_string(),
                tx_hash: Some("tx1".to_string()),
                outcome: VerdictOutcome::Good,
                details: None,
                metric: None,
                issued_at: 1000,
                issuer_id: "issuer1".to_string(),
                issuer_seq_no: 1,
                issuer_sig: "sig1".to_string(),
                tx_receipt: None,
                evidence_blobs: None,
            },
            TransactionVerdict {
                target_id: "peer1".to_string(),
                tx_hash: Some("tx2".to_string()),
                outcome: VerdictOutcome::Good,
                details: None,
                metric: None,
                issued_at: 2000,
                issuer_id: "issuer2".to_string(),
                issuer_seq_no: 1,
                issuer_sig: "sig2".to_string(),
                tx_receipt: None,
                evidence_blobs: None,
            },
        ];

        let score = calculate_transaction_score(&verdicts, &config);
        assert_eq!(score, 1.0); // All good verdicts = 1.0
    }

    #[test]
    fn test_blacklist_manager() {
        let config = ReputationConfig::default();
        let manager = BlacklistManager::new(config);

        // Add manual entry
        manager.add_manual("peer1".to_string(), "Testing".to_string()).unwrap();
        assert!(manager.is_blacklisted("peer1").unwrap());

        // Remove entry
        manager.remove("peer1").unwrap();
        assert!(!manager.is_blacklisted("peer1").unwrap());
    }

    #[test]
    fn test_reputation_cache() {
        let cache = ReputationCache::new(60);

        cache.set("peer1".to_string(), 0.8, TrustLevel::Trusted).unwrap();
        
        let cached = cache.get("peer1").unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().score, 0.8);

        let not_cached = cache.get("peer2").unwrap();
        assert!(not_cached.is_none());
    }
}
