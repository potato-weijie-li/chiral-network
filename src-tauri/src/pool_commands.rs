// Pool module - Tauri command wrappers for mining pool functionality
//
// This module wraps the core pool functionality from chiral-node
// with Tauri command attributes for use in the GUI.

use tauri::command;

// Re-export types from node crate
pub use chiral_node::pool::{
    JoinedPoolInfo, MiningPool, PoolStats, PoolStatus,
};

// Tauri command wrappers
#[command]
pub async fn discover_mining_pools() -> Result<Vec<MiningPool>, String> {
    chiral_node::pool::discover_mining_pools().await
}

#[command]
pub async fn create_mining_pool(
    address: String,
    name: String,
    description: String,
    fee_percentage: f64,
    min_payout: f64,
    payment_method: String,
    region: String,
) -> Result<MiningPool, String> {
    chiral_node::pool::create_mining_pool(
        address,
        name,
        description,
        fee_percentage,
        min_payout,
        payment_method,
        region,
    )
    .await
}

#[command]
pub async fn join_mining_pool(
    pool_id: String,
    address: String,
) -> Result<JoinedPoolInfo, String> {
    chiral_node::pool::join_mining_pool(pool_id, address).await
}

#[command]
pub async fn leave_mining_pool() -> Result<(), String> {
    chiral_node::pool::leave_mining_pool().await
}

#[command]
pub async fn get_current_pool_info() -> Result<Option<JoinedPoolInfo>, String> {
    chiral_node::pool::get_current_pool_info().await
}

#[command]
pub async fn get_pool_stats() -> Result<Option<PoolStats>, String> {
    chiral_node::pool::get_pool_stats().await
}

#[command]
pub async fn update_pool_discovery() -> Result<(), String> {
    chiral_node::pool::update_pool_discovery().await
}
