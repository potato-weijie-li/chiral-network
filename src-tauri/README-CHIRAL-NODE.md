# chiral-node (Headless)

A standalone headless binary for running Chiral Network P2P/DHT/mining/network services without depending on Tauri or the frontend. Suitable for server/service deployments (systemd, containers).

## Features

- 🖥️ **No GUI Required** - Runs without Tauri or frontend assets
- 🌐 **P2P Network Services** - DHT, libp2p, peer discovery
- ⚙️ **Configurable** - Command-line arguments for all settings
- 🔄 **Graceful Shutdown** - Handles SIGINT/SIGTERM properly
- 📝 **Logging** - Configurable verbosity levels

## Building

```bash
# Build release binary
cargo build --manifest-path src-tauri/Cargo.toml --bin chiral-node --release

# The binary will be at:
# target/release/chiral-node
```

## Usage

```bash
# Show help
./target/release/chiral-node --help

# Run with default settings
./target/release/chiral-node

# Run with custom DHT port
./target/release/chiral-node --dht-port 4002

# Run as bootstrap node
./target/release/chiral-node --is-bootstrap --show-multiaddr

# Connect to specific bootstrap nodes
./target/release/chiral-node --bootstrap /ip4/1.2.3.4/tcp/4001/p2p/PEER_ID

# Increase logging verbosity
./target/release/chiral-node -v      # Debug level
./target/release/chiral-node -vv     # Trace level

# Enable relay server mode
./target/release/chiral-node --enable-relay
```

## Command-Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-c, --config <PATH>` | Path to configuration file | None |
| `-v, --verbose` | Increase logging verbosity | Info level |
| `--no-daemon` | Run in foreground | true |
| `--dht-port <PORT>` | DHT port to listen on | 4001 |
| `--bootstrap <ADDR>` | Bootstrap nodes (repeatable) | None |
| `--enable-geth` | Enable geth node | false |
| `--geth-data-dir <PATH>` | Geth data directory | ./bin/geth-data |
| `--miner-address <ADDR>` | Miner address for geth | None |
| `--show-multiaddr` | Show node's multiaddr | false |
| `--secret <SECRET>` | Secret for consistent peer ID | None |
| `--is-bootstrap` | Run in bootstrap mode | false |
| `--disable-autonat` | Disable AutoNAT probes | false |
| `--enable-relay` | Enable relay server mode | false |

## Running as a Service (systemd)

Create a systemd service file at `/etc/systemd/system/chiral-node.service`:

```ini
[Unit]
Description=Chiral Network Node
After=network.target

[Service]
Type=simple
User=chiral
Group=chiral
ExecStart=/usr/local/bin/chiral-node --dht-port 4001
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
# Copy binary to /usr/local/bin
sudo cp target/release/chiral-node /usr/local/bin/

# Create service user
sudo useradd -r -s /bin/false chiral

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable chiral-node
sudo systemctl start chiral-node

# Check status
sudo systemctl status chiral-node

# View logs
sudo journalctl -u chiral-node -f
```

## Docker

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --manifest-path src-tauri/Cargo.toml --bin chiral-node --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/chiral-node /usr/local/bin/
EXPOSE 4001
ENTRYPOINT ["chiral-node"]
CMD ["--dht-port", "4001"]
```

Build and run:

```bash
docker build -t chiral-node .
docker run -p 4001:4001 chiral-node
```

## Environment Variables

- `RUST_LOG` - Override log filtering (e.g., `RUST_LOG=debug`)
- `CHIRAL_CHAIN_ID` - Override chain ID
- `CHIRAL_NETWORK_ID` - Override network ID
- `CHIRAL_DISABLE_AUTORELAY` - Set to "1" to disable AutoRelay

## License

MIT License - See [LICENSE](../LICENSE) for details.
