use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageNode {
    pub node_id: String,
    pub ip: String,
    pub port: u16,
    pub capacity: u64,
    pub available: u64,
    pub price_per_mb: f64,
    pub bandwidth_limit: u64,
    pub reputation: f32,
    pub uptime: f32,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSupplier {
    pub supplier_id: String,
    pub file_hash: String,
    pub ip: String,
    pub port: u16,
    pub price: f64,
    pub bandwidth: u64,
    pub reputation: f32,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketListing {
    pub file_hash: String,
    pub file_size: u64,
    pub suppliers: Vec<FileSupplier>,
    pub min_price: f64,
    pub avg_price: f64,
    pub created_at: u64,
}

pub struct MarketService {
    storage_nodes: Arc<Mutex<HashMap<String, StorageNode>>>,
    file_suppliers: Arc<Mutex<HashMap<String, Vec<FileSupplier>>>>, // file_hash -> suppliers
}

impl MarketService {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        
        // Add some mock storage nodes for testing
        nodes.insert("node_1".to_string(), StorageNode {
            node_id: "node_1".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 8080,
            capacity: 1024 * 1024 * 1024, // 1GB
            available: 512 * 1024 * 1024, // 512MB
            price_per_mb: 0.001,
            bandwidth_limit: 100,
            reputation: 4.5,
            uptime: 0.99,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        nodes.insert("node_2".to_string(), StorageNode {
            node_id: "node_2".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 8081,
            capacity: 2048 * 1024 * 1024, // 2GB
            available: 1024 * 1024 * 1024, // 1GB
            price_per_mb: 0.0008,
            bandwidth_limit: 150,
            reputation: 4.8,
            uptime: 0.95,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });

        MarketService {
            storage_nodes: Arc::new(Mutex::new(nodes)),
            file_suppliers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Query available storage nodes that can store a file
    pub async fn query_storage_nodes(&self, file_size: u64, replication_factor: u32) -> Result<Vec<StorageNode>, String> {
        let nodes = self.storage_nodes.lock().await;
        
        let mut suitable_nodes: Vec<StorageNode> = nodes
            .values()
            .filter(|node| {
                node.available >= file_size && 
                node.uptime > 0.9 && 
                node.reputation > 3.0
            })
            .cloned()
            .collect();
        
        // Sort by price (cheapest first), then by reputation (best first)
        suitable_nodes.sort_by(|a, b| {
            a.price_per_mb.partial_cmp(&b.price_per_mb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.reputation.partial_cmp(&a.reputation).unwrap_or(std::cmp::Ordering::Equal))
        });
        
        // Return only the number of nodes needed for replication
        let needed_nodes = std::cmp::min(replication_factor as usize, suitable_nodes.len());
        suitable_nodes.truncate(needed_nodes);
        
        info!("Found {} suitable storage nodes for file size {}", suitable_nodes.len(), file_size);
        
        if suitable_nodes.len() < replication_factor as usize {
            warn!("Only found {} nodes, but {} replication factor requested", 
                  suitable_nodes.len(), replication_factor);
        }
        
        Ok(suitable_nodes)
    }

    /// Register that a storage node is now serving a specific file
    pub async fn register_file_supplier(&self, file_hash: String, supplier: FileSupplier) -> Result<(), String> {
        let mut suppliers = self.file_suppliers.lock().await;
        
        suppliers
            .entry(file_hash.clone())
            .or_insert_with(Vec::new)
            .push(supplier.clone());
        
        info!("Registered supplier {} for file {}", supplier.supplier_id, file_hash);
        Ok(())
    }

    /// Look up suppliers for a specific file
    pub async fn lookup_file_suppliers(&self, file_hash: String) -> Result<Vec<FileSupplier>, String> {
        let suppliers = self.file_suppliers.lock().await;
        
        match suppliers.get(&file_hash) {
            Some(file_suppliers) => {
                let mut active_suppliers: Vec<FileSupplier> = file_suppliers
                    .iter()
                    .filter(|supplier| {
                        let current_time = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        // Only include suppliers seen in the last 5 minutes
                        current_time - supplier.last_seen < 300
                    })
                    .cloned()
                    .collect();
                
                // Sort by price and reputation
                active_suppliers.sort_by(|a, b| {
                    a.price.partial_cmp(&b.price)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| b.reputation.partial_cmp(&a.reputation).unwrap_or(std::cmp::Ordering::Equal))
                });
                
                info!("Found {} active suppliers for file {}", active_suppliers.len(), file_hash);
                Ok(active_suppliers)
            }
            None => {
                warn!("No suppliers found for file {}", file_hash);
                Ok(Vec::new())
            }
        }
    }

    /// Get market statistics
    pub async fn get_market_stats(&self) -> Result<serde_json::Value, String> {
        let nodes = self.storage_nodes.lock().await;
        let suppliers = self.file_suppliers.lock().await;
        
        let total_nodes = nodes.len();
        let online_nodes = nodes.values().filter(|node| {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            current_time - node.last_seen < 300 // 5 minutes
        }).count();
        
        let total_files = suppliers.len();
        let avg_suppliers_per_file = if total_files > 0 {
            suppliers.values().map(|s| s.len()).sum::<usize>() as f64 / total_files as f64
        } else {
            0.0
        };
        
        Ok(serde_json::json!({
            "total_storage_nodes": total_nodes,
            "online_storage_nodes": online_nodes,
            "total_files": total_files,
            "avg_suppliers_per_file": avg_suppliers_per_file,
            "market_health": if online_nodes as f64 / total_nodes as f64 > 0.8 { "healthy" } else { "degraded" }
        }))
    }

    /// Register a new storage node
    pub async fn register_storage_node(&self, node: StorageNode) -> Result<(), String> {
        let mut nodes = self.storage_nodes.lock().await;
        nodes.insert(node.node_id.clone(), node.clone());
        info!("Registered storage node {}", node.node_id);
        Ok(())
    }
}