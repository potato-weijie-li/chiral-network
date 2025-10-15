# Chiral Node

This is the standalone node crate for Chiral Network. It provides core functionality that can be used independently of the Tauri desktop application.

## Features

- **Keystore Management**: Secure storage and management of Ethereum accounts
- **Command Line Interface**: Easy-to-use CLI for keystore operations

## Usage

### Building

```bash
cargo build --release
```

The binary will be located at `target/release/chiral-node`.

### Keystore Commands

#### Add an account to the keystore

```bash
chiral-node keystore add \
  --address "0x1234..." \
  --private-key "0xabcd..." \
  --password "your-password"
```

#### List all accounts in the keystore

```bash
chiral-node keystore list
```

#### Load an account from the keystore

```bash
chiral-node keystore load \
  --address "0x1234..." \
  --password "your-password"
```

## Integration with Tauri

The `chiral-network` crate (in `src-tauri`) depends on this crate and uses its functionality for keystore operations. This allows the same keystore logic to be used both in the desktop application and in standalone nodes.

## Running Standalone Nodes

For running standalone bootstrap nodes or other headless nodes, use the `chiral-network-node` binary which is built as part of the `src-tauri` package:

```bash
cd ../src-tauri
cargo build --release --bin chiral-network-node
./target/release/chiral-network-node --is-bootstrap --dht-port 4001
```

Or use the convenience script:

```bash
cd ..
./run-bootstrap.sh --port 4001
```

This allows you to run bootstrap nodes without building the frontend.
