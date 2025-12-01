//! Chiral Node - Headless P2P node binary
//!
//! A standalone headless binary for running Chiral Network node services
//! without the Tauri frontend. Suitable for server/service deployments.

use std::path::PathBuf;
use std::process;

use chiral_network::node::{self, NodeConfig};
use clap::Parser;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Chiral Node - Headless P2P File Sharing Node
#[derive(Parser, Debug)]
#[command(name = "chiral-node")]
#[command(author = "Chiral Network Team")]
#[command(version)]
#[command(about = "Headless Chiral Network node for P2P file sharing", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(long, short = 'c')]
    config: Option<PathBuf>,

    /// Increase logging verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Run in foreground (default). Use --no-daemon to explicitly disable daemon mode.
    #[arg(long)]
    no_daemon: bool,

    /// DHT port to listen on
    #[arg(long, default_value = "4001")]
    dht_port: u16,

    /// Bootstrap nodes to connect to (can be specified multiple times)
    #[arg(long)]
    bootstrap: Vec<String>,

    /// Generate multiaddr for this node (shows the address others can connect to)
    #[arg(long)]
    show_multiaddr: bool,

    /// Secret for generating consistent peer ID
    #[arg(long)]
    secret: Option<String>,

    /// Run in bootstrap mode
    #[arg(long)]
    is_bootstrap: bool,

    /// Disable AutoNAT reachability probes
    #[arg(long)]
    disable_autonat: bool,

    /// Enable relay server mode
    #[arg(long)]
    enable_relay: bool,
}

/// Initialize tracing/logging based on verbosity level
fn init_logging(verbose: u8) {
    let level = match verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let filter = EnvFilter::from_default_env()
        .add_directive(format!("chiral_network={}", level).parse().unwrap())
        .add_directive(format!("chiral_node={}", level).parse().unwrap())
        .add_directive("libp2p=warn".parse().unwrap())
        .add_directive("libp2p_kad=info".parse().unwrap());

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating shutdown...");
        }
        _ = terminate => {
            info!("Received SIGTERM, initiating shutdown...");
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose);

    info!("Starting chiral-node (headless)");
    info!("Config file: {:?}", args.config);

    // Build node configuration from CLI args
    let node_config = NodeConfig {
        dht_port: args.dht_port,
        bootstrap_nodes: args.bootstrap.clone(),
        secret: args.secret.clone(),
        is_bootstrap: args.is_bootstrap,
        enable_autonat: !args.disable_autonat,
        autonat_probe_interval_secs: 30,
        enable_relay: args.enable_relay,
        enable_autorelay: true,
        preferred_relays: Vec::new(),
    };

    // Display multiaddr if requested
    if args.show_multiaddr {
        if let Some(ip) = node::get_local_ip() {
            info!("🔗 Expected multiaddr: /ip4/{}/tcp/{}/p2p/<peer_id>", ip, args.dht_port);
        }
    }

    // Start node services with shutdown signal handling
    tokio::select! {
        result = node::run(node_config) => {
            match result {
                Ok(_) => info!("Node services completed"),
                Err(e) => {
                    error!("Node services failed: {:?}", e);
                    process::exit(1);
                }
            }
        }
        _ = shutdown_signal() => {
            info!("Shutting down node services...");
        }
    }

    info!("chiral-node shutdown complete");
    process::exit(0);
}
