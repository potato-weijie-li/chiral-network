//! Node module for headless Chiral Network node operation.
//!
//! This module provides the core functionality for running a Chiral Network
//! node without the Tauri frontend, suitable for server deployments.

use crate::dht::DhtService;
use std::{sync::Arc, time::Duration};
use tracing::{info, warn};

/// Configuration for the headless node
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// DHT port to listen on
    pub dht_port: u16,
    /// Bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<String>,
    /// Secret for generating consistent peer ID
    pub secret: Option<String>,
    /// Run in bootstrap mode
    pub is_bootstrap: bool,
    /// Enable AutoNAT reachability probes
    pub enable_autonat: bool,
    /// AutoNAT probe interval in seconds
    pub autonat_probe_interval_secs: u64,
    /// Enable relay server mode
    pub enable_relay: bool,
    /// Enable AutoRelay
    pub enable_autorelay: bool,
    /// Preferred relay nodes
    pub preferred_relays: Vec<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            dht_port: 4001,
            bootstrap_nodes: Vec::new(),
            secret: None,
            is_bootstrap: false,
            enable_autonat: true,
            autonat_probe_interval_secs: 30,
            enable_relay: false,
            enable_autorelay: true,
            preferred_relays: Vec::new(),
        }
    }
}

/// Result of running the node
pub type NodeResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Run the headless node with the given configuration
pub async fn run(config: NodeConfig) -> NodeResult<()> {
    info!("Starting Chiral Network node");
    info!("DHT port: {}", config.dht_port);
    info!("Bootstrap nodes: {:?}", config.bootstrap_nodes);
    info!("Is bootstrap: {}", config.is_bootstrap);

    let enable_autonat = config.enable_autonat;
    let probe_interval = if enable_autonat {
        Some(Duration::from_secs(config.autonat_probe_interval_secs))
    } else {
        None
    };

    // Start DHT service
    let dht_service = DhtService::new(
        config.dht_port,
        config.bootstrap_nodes.clone(),
        config.secret,
        config.is_bootstrap,
        enable_autonat,
        probe_interval,
        config.bootstrap_nodes.clone(), // autonat_servers
        None, // proxy_address
        None, // file_transfer_service
        None, // chunk_manager
        None, // chunk_size_kb
        None, // cache_size_mb
        config.enable_autorelay,
        config.preferred_relays,
        config.enable_relay,
        true, // enable_upnp
        None, // blockstore_path
        None, // previous_autorelay_enabled
        None, // previous_autorelay_disabled
    )
    .await
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.to_string().into() })?;

    let peer_id = dht_service.get_peer_id().await;
    info!("Node started with peer ID: {}", peer_id);

    // Connect to bootstrap nodes
    if !config.bootstrap_nodes.is_empty() {
        for bootstrap_addr in &config.bootstrap_nodes {
            match dht_service.connect_peer(bootstrap_addr.clone()).await {
                Ok(_) => info!("Connected to bootstrap: {}", bootstrap_addr),
                Err(e) => warn!("Failed to connect to {}: {}", bootstrap_addr, e),
            }
        }
    }

    let dht_arc = Arc::new(dht_service);
    
    info!("Node is running. Waiting for shutdown signal...");

    // Keep running until shutdown is triggered externally
    // The caller should use tokio::select! with a shutdown signal
    std::future::pending::<()>().await;

    Ok(())
}

/// Get the local IP address for display purposes
pub fn get_local_ip() -> Option<String> {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return Some(addr.ip().to_string());
            }
        }
    }
    None
}
