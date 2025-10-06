# Standalone Node Implementation

## Summary

This PR implements a standalone node crate that allows running Chiral Network nodes (e.g., bootstrap nodes, storage nodes) without building the frontend application. This addresses the issue where standalone nodes had to build the entire Tauri application including the frontend just to run headless.

## Changes Made

### 1. Created `node/` Crate

Created a new Rust crate in the `node/` directory that contains:

- **Core P2P functionality**: DHT, libp2p networking, file transfer
- **Blockchain functionality**: Ethereum/Geth integration, mining
- **Encryption & Security**: File encryption, keystore management
- **Peer selection**: Intelligent peer management
- **WebRTC**: P2P file transfer over WebRTC
- **CLI Entry Point**: `main.rs` for running standalone nodes

### 2. Refactored `src-tauri/` 

Updated the Tauri application to use the node crate:

- Added `chiral-node` as a dependency in `src-tauri/Cargo.toml`
- Updated `src-tauri/src/main.rs` to import from `chiral-node`
- Removed redundant source files (analytics.rs, dht.rs, etc.)
- Created `headless.rs` wrapper to add `--headless` flag support
- Created `pool_commands.rs` to wrap pool functions with Tauri commands
- Updated `src-tauri/src/lib.rs` to re-export from node crate

### 3. Updated Bootstrap Script

Modified `run-bootstrap.sh` to:

- Build and use `node/target/release/chiral-node` instead of `src-tauri/target/release/chiral-network`
- Remove dependency on creating empty `dist/` folder
- Use the standalone node binary directly

### 4. Documentation

Updated `README.md` with:

- Section on "Running Standalone Nodes"
- Build instructions for the standalone node
- Command-line options documentation
- Example usage for bootstrap nodes and storage nodes

## Benefits

1. **No Frontend Build Required**: Bootstrap nodes and storage servers can be built without compiling the Tauri frontend
2. **Faster Build Times**: Standalone nodes don't need GTK, webkit, or other GUI dependencies
3. **Better Separation of Concerns**: Core P2P functionality is now in a separate crate
4. **Code Reuse**: Both the GUI and CLI use the same core functionality
5. **Easier Testing**: The node crate can be tested independently

## Testing

Verified that:

- ✅ Standalone node builds successfully: `cd node && cargo build --release`
- ✅ Node binary runs and starts correctly
- ✅ Bootstrap script (`run-bootstrap.sh`) works as expected
- ✅ Node connects to bootstrap peers successfully
- ✅ DHT functionality works (peer discovery, file metadata publishing)

## File Structure

```
chiral-network/
├── node/                          # New standalone node crate
│   ├── Cargo.toml                # Node dependencies (no Tauri)
│   ├── src/
│   │   ├── main.rs               # CLI entry point
│   │   ├── lib.rs                # Library exports
│   │   ├── analytics.rs          # Analytics functionality
│   │   ├── dht.rs                # DHT and P2P networking
│   │   ├── encryption.rs         # File encryption
│   │   ├── ethereum.rs           # Blockchain integration
│   │   ├── file_transfer.rs      # File transfer logic
│   │   ├── headless.rs           # Headless mode runner
│   │   └── ...                   # Other core modules
│   └── target/
│       └── release/
│           └── chiral-node       # Standalone binary
├── src-tauri/                     # Tauri GUI application
│   ├── Cargo.toml                # Now depends on node crate
│   ├── src/
│   │   ├── main.rs               # GUI entry point
│   │   ├── lib.rs                # Re-exports from node
│   │   ├── headless.rs           # Wrapper with --headless flag
│   │   ├── pool_commands.rs      # Tauri command wrappers
│   │   └── two_fa.rs             # GUI-specific 2FA
│   └── ...
└── run-bootstrap.sh              # Updated to use node binary
```

## Migration Notes

- The `src-tauri` crate now imports core functionality from the `node` crate
- GUI-specific features (2FA, proxy commands) remain in `src-tauri`
- The `headless` flag is kept in `src-tauri` for backward compatibility
- Pool functions now have Tauri command wrappers in `pool_commands.rs`

## Usage Examples

### Building Standalone Node

```bash
cd node
cargo build --release
```

### Running Bootstrap Node

```bash
# Using the script
./run-bootstrap.sh --port 4001

# Or directly
./node/target/release/chiral-node --is-bootstrap --dht-port 4001
```

### Running Storage Node

```bash
./node/target/release/chiral-node \
  --dht-port 4002 \
  --bootstrap /ip4/BOOTSTRAP_IP/tcp/4001/p2p/PEER_ID
```

## Related Files

- `node/Cargo.toml` - Standalone node dependencies
- `node/src/main.rs` - CLI entry point
- `node/src/lib.rs` - Library exports
- `src-tauri/Cargo.toml` - Added node crate dependency
- `src-tauri/src/main.rs` - Updated to use node crate
- `src-tauri/src/lib.rs` - Re-exports from node crate
- `run-bootstrap.sh` - Updated to use node binary
- `README.md` - Added standalone node documentation
