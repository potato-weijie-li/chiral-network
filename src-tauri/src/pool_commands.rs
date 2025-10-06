// Pool module - Tauri command wrappers for mining pool functionality
//
// This module wraps the core pool functionality from chiral-node
// with Tauri command attributes for use in the GUI.

use tauri::command;

// Re-export types from node crate
pub use chiral_node::pool::{
    MiningPool, PoolDiscoveryService, PoolInfo, PoolStats, PoolStatus,
};

// Tauri command wrappers
#[command]
pub async fn discover_mining_pools(
    pool_discovery: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<Option<PoolDiscoveryService>>>>,
) -> Result<Vec<MiningPool>, String> {
    chiral_node::pool::discover_mining_pools(pool_discovery.inner().clone()).await
}

#[command]
pub async fn create_mining_pool(
    name: String,
    url: String,
    description: String,
    fee_percentage: f64,
    min_payout: f64,
    payment_method: String,
    creator_address: String,
    pool_discovery: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<Option<PoolDiscoveryService>>>>,
) -> Result<MiningPool, String> {
    chiral_node::pool::create_mining_pool(
        name,
        url,
        description,
        fee_percentage,
        min_payout,
        payment_method,
        creator_address,
        pool_discovery.inner().clone(),
    )
    .await
}

#[command]
pub async fn join_mining_pool(
    pool_id: String,
    miner_address: String,
    pool_discovery: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<Option<PoolDiscoveryService>>>>,
) -> Result<(), String> {
    chiral_node::pool::join_mining_pool(pool_id, miner_address, pool_discovery.inner().clone()).await
}

#[command]
pub async fn leave_mining_pool(
    pool_discovery: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<Option<PoolDiscoveryService>>>>,
) -> Result<(), String> {
    chiral_node::pool::leave_mining_pool(pool_discovery.inner().clone()).await
}

#[command]
pub async fn get_current_pool_info(
    pool_discovery: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<Option<PoolDiscoveryService>>>>,
) -> Result<Option<PoolInfo>, String> {
    chiral_node::pool::get_current_pool_info(pool_discovery.inner().clone()).await
}

#[command]
pub async fn get_pool_stats(
    pool_discovery: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<Option<PoolDiscoveryService>>>>,
) -> Result<Option<PoolStats>, String> {
    chiral_node::pool::get_pool_stats(pool_discovery.inner().clone()).await
}

#[command]
pub async fn update_pool_discovery(
    pool_discovery: tauri::State<'_, std::sync::Arc<tokio::sync::Mutex<Option<PoolDiscoveryService>>>>,
) -> Result<Vec<MiningPool>, String> {
    chiral_node::pool::update_pool_discovery(pool_discovery.inner().clone()).await
}
