//! Chiral Node - Headless P2P node binary
//!
//! A standalone headless binary for running Chiral Network node services
//! without the Tauri frontend. Suitable for server/service deployments.

use std::path::PathBuf;
use std::process;

use clap::Parser;
use tracing::{info, Level};
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

fn main() {
    let args = Args::parse();

    // Initialize logging
    init_logging(args.verbose);

    info!("Starting chiral-node (headless)");
    info!("Config file: {:?}", args.config);

    process::exit(0);
}
