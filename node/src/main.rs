// CLI entry point for standalone Chiral Network node
use chiral_node::keystore;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "chiral-node")]
#[command(about = "Chiral Network Node - Standalone CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manage keystore accounts
    Keystore {
        #[command(subcommand)]
        action: KeystoreCommands,
    },
    /// Run a bootstrap node (uses headless mode from chiral-network crate)
    Bootstrap {
        /// DHT port to listen on
        #[arg(long, default_value = "4001")]
        dht_port: u16,
        
        /// Bootstrap nodes to connect to
        #[arg(long)]
        bootstrap: Vec<String>,
        
        /// Enable geth node
        #[arg(long)]
        enable_geth: bool,
        
        /// Log level
        #[arg(long, default_value = "info")]
        log_level: String,
    },
}

#[derive(Subcommand, Debug)]
enum KeystoreCommands {
    /// Add a new account to the keystore
    Add {
        /// Account address
        #[arg(long)]
        address: String,
        
        /// Private key
        #[arg(long)]
        private_key: String,
        
        /// Password to encrypt the private key
        #[arg(long)]
        password: String,
    },
    /// List all accounts in the keystore
    List,
    /// Load an account from the keystore
    Load {
        /// Account address
        #[arg(long)]
        address: String,
        
        /// Password to decrypt the private key
        #[arg(long)]
        password: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keystore { action } => {
            handle_keystore_command(action).await;
        }
        Commands::Bootstrap { dht_port, bootstrap, enable_geth, log_level } => {
            println!("Bootstrap node functionality will be implemented by using chiral-network crate's headless mode");
            println!("DHT Port: {}", dht_port);
            println!("Bootstrap nodes: {:?}", bootstrap);
            println!("Enable Geth: {}", enable_geth);
            println!("Log level: {}", log_level);
            println!("\nNote: For now, please use the chiral-network binary with --headless flag");
        }
    }
}

async fn handle_keystore_command(action: KeystoreCommands) {
    match action {
        KeystoreCommands::Add { address, private_key, password } => {
            match keystore::save_account_to_keystore(address.clone(), private_key, password).await {
                Ok(_) => println!("Account {} added to keystore successfully", address),
                Err(e) => eprintln!("Failed to add account: {}", e),
            }
        }
        KeystoreCommands::List => {
            match keystore::list_keystore_accounts().await {
                Ok(accounts) => {
                    if accounts.is_empty() {
                        println!("No accounts in keystore");
                    } else {
                        println!("Accounts in keystore:");
                        for account in accounts {
                            println!("  - {}", account);
                        }
                    }
                }
                Err(e) => eprintln!("Failed to list accounts: {}", e),
            }
        }
        KeystoreCommands::Load { address, password } => {
            match keystore::load_account_from_keystore(address.clone(), password).await {
                Ok(private_key) => {
                    println!("Account {} loaded successfully", address);
                    println!("Private key: {}", private_key);
                }
                Err(e) => eprintln!("Failed to load account: {}", e),
            }
        }
    }
}
