//! Chiral Node - Headless P2P node binary
//!
//! A standalone headless binary for running Chiral Network node services
//! without the Tauri frontend. Suitable for server/service deployments.

use std::path::PathBuf;
use std::process;

use clap::Parser;

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

fn main() {
    let args = Args::parse();

    println!("chiral-node starting...");
    println!("Config: {:?}", args.config);
    println!("Verbose level: {}", args.verbose);

    process::exit(0);
}
