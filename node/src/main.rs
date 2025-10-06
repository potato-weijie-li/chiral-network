// Chiral Node - Standalone CLI entry point
//
// This binary allows running a Chiral Network node without the Tauri GUI.
// It's designed for bootstrap nodes, servers, and headless deployments.

use chiral_node::headless::{run_headless, CliArgs};
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("chiral_node=info".parse().unwrap())
                .add_directive("libp2p=info".parse().unwrap())
                .add_directive("libp2p_kad=debug".parse().unwrap())
                .add_directive("libp2p_swarm=info".parse().unwrap()),
        )
        .init();

    // Parse command line arguments
    let args = CliArgs::parse();

    // Run in headless mode
    if let Err(e) = run_headless(args).await {
        eprintln!("Error running node: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
