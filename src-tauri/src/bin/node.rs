// Standalone node binary without Tauri dependencies
// This binary can run bootstrap nodes and other standalone nodes
// without requiring the frontend to be built

use chiral_network::headless::{run_headless, CliArgs};
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    // Initialize logging
    let args = CliArgs::parse();
    
    let log_level = args.log_level.clone();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();

    // Run the headless mode
    if let Err(e) = run_headless(args).await {
        eprintln!("Error in headless mode: {}", e);
        std::process::exit(1);
    }
}
