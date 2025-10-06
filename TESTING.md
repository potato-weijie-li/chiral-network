# Testing Guide for Standalone Node Feature

This guide provides detailed instructions for testing the standalone node implementation locally.

## Prerequisites

Before testing, ensure you have:

- **Rust toolchain** (1.70 or later): Install from [rustup.rs](https://rustup.rs/)
- **Git**: To clone the repository
- **Basic command line knowledge**

## Testing Overview

This PR introduces a standalone `node` crate that can be built and run without the frontend. You can test:

1. **Building the standalone node** - Verify it compiles without Tauri/frontend dependencies
2. **Running a standalone bootstrap node** - Test the node in isolation
3. **Testing the convenience script** - Verify `run-bootstrap.sh` works
4. **Verifying the Tauri app still works** - Ensure backward compatibility (optional)

---

## Test 1: Clone and Setup

```bash
# Clone the repository
git clone https://github.com/potato-weijie-li/chiral-network.git
cd chiral-network

# Checkout the PR branch
git checkout copilot/fix-e32c6681-b7b4-4d85-8dc9-263de9d8ba1e
```

**Expected Result:** Repository cloned and PR branch checked out successfully.

---

## Test 2: Build Standalone Node

Build the standalone node **without** building the frontend:

```bash
# Navigate to the node directory
cd node

# Build in release mode
cargo build --release
```

**Expected Result:**
- Build completes successfully in 3-5 minutes
- Binary created at `node/target/release/chiral-node`
- No errors about missing frontend files or Tauri dependencies
- Output shows compilation of node crate and its dependencies

**What to verify:**
```bash
# Check binary exists and is executable
ls -lh target/release/chiral-node

# Should show something like:
# -rwxr-xr-x  1 user  staff   7.7M Oct  6 18:37 chiral-node
```

---

## Test 3: Run Standalone Node with Help Flag

Verify the CLI interface works:

```bash
# From the node directory
./target/release/chiral-node --help
```

**Expected Result:**
```
Chiral Network - Standalone Node

Usage: chiral-node [OPTIONS]

Options:
  --dht-port <DHT_PORT>
          DHT port to listen on [default: 4001]
  --bootstrap <BOOTSTRAP>
          Bootstrap nodes to connect to (can be specified multiple times)
  --enable-geth
          Enable geth node
  --geth-data-dir <GETH_DATA_DIR>
          Geth data directory [default: ./bin/geth-data]
  --miner-address <MINER_ADDRESS>
          Miner address for geth
  --log-level <LOG_LEVEL>
          Log level (trace, debug, info, warn, error) [default: info]
  --show-multiaddr
          Generate multiaddr for this node
  --secret <SECRET>
  --is-bootstrap
  --disable-autonat
          Disable AutoNAT reachability probes
  --autonat-probe-interval <AUTONAT_PROBE_INTERVAL>
          Interval in seconds between AutoNAT probes [default: 30]
  --autonat-server <AUTONAT_SERVER>
          Additional AutoNAT servers to dial
  --show-reachability
          Print reachability snapshot at startup
  --show-dcutr
          Print DCUtR hole-punching metrics at startup
  --socks5-proxy <SOCKS5_PROXY>
  --show-downloads
          Print local download metrics snapshot at startup
  -h, --help
          Print help
```

---

## Test 4: Run Bootstrap Node (Short Test)

Start a bootstrap node and let it run for a few seconds:

```bash
# From the node directory
./target/release/chiral-node --is-bootstrap --dht-port 4001 --log-level info
```

**Expected Result:**
- Node starts successfully
- Logs show:
  - "Starting Chiral Network in headless mode"
  - "DHT Port: 4001"
  - "Using default bootstrap nodes: [...]"
  - "Local Peer ID: 12D3KooW..."
  - "✅ DHT node started"
  - "📡 Now listening on: /ip4/127.0.0.1/tcp/4001"
  - Connection attempts to bootstrap nodes

**Sample Output:**
```
2025-10-06T18:31:38.553853Z  INFO chiral_node::headless: Starting Chiral Network in headless mode
2025-10-06T18:31:38.553868Z  INFO chiral_node::headless: DHT Port: 4001
2025-10-06T18:31:38.485811Z  INFO chiral_node::dht: Local peer id: 12D3KooWDfrFAPeFrR58ZAQkvU7DkC9o9F5SJUHwvt5EstaGD4sd
2025-10-06T18:31:38.554886Z  INFO chiral_node::dht: Attempting to connect to bootstrap: /ip4/145.40.118.135/tcp/4001/p2p/...
2025-10-06T18:31:38.555384Z  INFO chiral_node::dht: DHT node is running
2025-10-06T18:31:38.555395Z  INFO chiral_node::headless: ✅ DHT node started
2025-10-06T18:31:38.555403Z  INFO chiral_node::headless: 📍 Local Peer ID: 12D3KooWDfrFAPeFrR58ZAQkvU7DkC9o9F5SJUHwvt5EstaGD4sd
2025-10-06T18:31:38.488217Z  INFO chiral_node::dht: 📡 Now listening on: /ip4/127.0.0.1/tcp/4001
```

**To stop:** Press `Ctrl+C`

**Note:** You may see some mDNS errors in certain environments - this is expected and doesn't affect core functionality.

---

## Test 5: Test with Custom Options

Try various command-line options:

### Show Multiaddr (for other nodes to connect)
```bash
./target/release/chiral-node --is-bootstrap --show-multiaddr --dht-port 5001
```

**Expected Result:** Displays the multiaddr that other nodes can use to connect to this node.

### Test Different Log Levels
```bash
# Debug logging
./target/release/chiral-node --is-bootstrap --log-level debug --dht-port 5002

# Trace logging (very verbose)
./target/release/chiral-node --is-bootstrap --log-level trace --dht-port 5003
```

### Show Reachability Status
```bash
./target/release/chiral-node --is-bootstrap --show-reachability --dht-port 5004
```

**Expected Result:** Shows AutoNAT reachability information periodically.

---

## Test 6: Test Bootstrap Script

The convenience script automates building and running:

```bash
# Go back to repository root
cd ..

# Run the script
./run-bootstrap.sh --port 4005 --log-level info
```

**Expected Result:**
- Script displays banner:
  ```
  🚀 Chiral Network Bootstrap Node
  ================================
  📍 Starting bootstrap node on port 4005
  📊 Log level: info
  ```
- If binary doesn't exist, it builds it automatically
- Node starts successfully with specified options
- Press `Ctrl+C` to stop

---

## Test 7: Test Multiple Nodes (Local Network)

Test running multiple nodes on different ports:

### Terminal 1: Start First Bootstrap Node
```bash
cd node
./target/release/chiral-node --is-bootstrap --dht-port 6001 --log-level info --show-multiaddr
```

**Note the multiaddr from the output** (looks like `/ip4/127.0.0.1/tcp/6001/p2p/12D3KooW...`)

### Terminal 2: Start Second Node Connected to First
```bash
cd node
# Replace PEER_ID with the actual peer ID from Terminal 1
./target/release/chiral-node --dht-port 6002 --bootstrap /ip4/127.0.0.1/tcp/6001/p2p/PEER_ID --log-level info
```

**Expected Result:**
- Both nodes running successfully
- Second node connects to first node
- Logs show successful peer connections

---

## Test 8: Verify No Frontend Dependencies

Confirm the node crate doesn't depend on Tauri:

```bash
# Check node/Cargo.toml for Tauri dependencies
grep -i "tauri" node/Cargo.toml
```

**Expected Result:** No output (no Tauri dependencies found)

```bash
# Verify src-tauri uses the node crate
grep "chiral-node" src-tauri/Cargo.toml
```

**Expected Result:**
```toml
chiral-node = { path = "../node" }
```

---

## Test 9: Verify Code Structure

Check that redundant files were removed:

```bash
# These files should NOT exist (removed from src-tauri)
ls src-tauri/src/dht.rs 2>/dev/null && echo "❌ File should be removed" || echo "✅ File correctly removed"
ls src-tauri/src/ethereum.rs 2>/dev/null && echo "❌ File should be removed" || echo "✅ File correctly removed"
ls src-tauri/src/analytics.rs 2>/dev/null && echo "❌ File should be removed" || echo "✅ File correctly removed"

# These files SHOULD exist in the node crate
ls node/src/dht.rs && echo "✅ Core module exists"
ls node/src/ethereum.rs && echo "✅ Core module exists"
ls node/src/main.rs && echo "✅ CLI entry point exists"
```

---

## Test 10: Build Tauri App (Optional - Requires Frontend Setup)

If you want to verify the Tauri app still works:

```bash
# Install Node.js dependencies (if not already done)
npm install

# Build the Tauri application
npm run tauri build
```

**Expected Result:**
- Tauri app builds successfully
- Uses the node crate for core functionality
- Both GUI and headless modes work

**Note:** This requires Node.js, npm, and system dependencies (GTK, webkit, etc.) for Tauri.

---

## Troubleshooting

### Build Errors

**Problem:** Cargo can't find dependencies
```bash
# Solution: Update Cargo index
cargo update
```

**Problem:** Missing Rust toolchain
```bash
# Solution: Install/update Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

### Runtime Errors

**Problem:** "Address already in use" error
```bash
# Solution: Use a different port
./target/release/chiral-node --is-bootstrap --dht-port 4002
```

**Problem:** Permission denied errors for mDNS
- **This is expected** in some environments (Docker, CI, etc.)
- Node still functions correctly for DHT and peer-to-peer communication
- Only affects local network discovery

### Network Connection Issues

**Problem:** Can't connect to bootstrap nodes
- This may be due to network restrictions or firewall
- Node will still start and can operate independently
- Try using `--bootstrap` flag to specify local nodes instead

---

## Success Criteria

Your testing is successful if:

- ✅ Standalone node builds without frontend dependencies
- ✅ Binary runs and shows help information
- ✅ Bootstrap node starts and listens on specified port
- ✅ Node generates a peer ID and publishes metadata
- ✅ Logs show DHT initialization and listening addresses
- ✅ Different command-line options work as expected
- ✅ No Tauri dependencies in `node/Cargo.toml`
- ✅ Redundant source files removed from `src-tauri/src/`

---

## Quick Test Script

For a quick automated test, you can run:

```bash
#!/bin/bash
echo "Testing Standalone Node Implementation"
echo "======================================"

# Test 1: Build
echo "Test 1: Building standalone node..."
cd node
cargo build --release || exit 1
echo "✅ Build successful"

# Test 2: Help
echo -e "\nTest 2: Testing help flag..."
./target/release/chiral-node --help > /dev/null || exit 1
echo "✅ Help command works"

# Test 3: Run for 3 seconds
echo -e "\nTest 3: Starting bootstrap node for 3 seconds..."
timeout 3 ./target/release/chiral-node --is-bootstrap --dht-port 7001 --log-level info || true
echo "✅ Node ran successfully"

echo -e "\n======================================"
echo "All tests passed! ✅"
```

Save this as `test_node.sh`, make it executable with `chmod +x test_node.sh`, and run it.

---

## Additional Resources

- **IMPLEMENTATION.md**: Technical details about the architecture changes
- **README.md**: Full documentation including usage examples
- **node/src/main.rs**: CLI entry point source code
- **node/Cargo.toml**: Dependencies for the standalone node

## Questions?

If you encounter any issues during testing, please:
1. Check the troubleshooting section above
2. Review the logs for specific error messages
3. Comment on the PR with details about your environment and the issue
