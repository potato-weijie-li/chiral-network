# Testing Guide: Multi-Protocol Support Feature

**Status:** AWAITING APPROVAL  
**Related:** [Feature Proposal](multi-protocol-support.md)

This guide explains how to manually test the BitTorrent compatibility feature once it's implemented.

## Prerequisites

### Software Required
- Chiral Network (with BitTorrent support implemented)
- qBittorrent or Transmission (for cross-compatibility testing)
- Wireshark (optional, for protocol debugging)
- Ubuntu or another Linux distro (for test files)

### Test Files Needed
- Ubuntu 22.04 ISO torrent/magnet link (legal, well-seeded)
- Custom test file (1-10 MB, for quick iteration)

## Test Suite

### Test 1: Magnet Link Import ⚡

**Objective:** Verify magnet link parsing and DHT resolution

**Steps:**
1. Open Chiral Network
2. Navigate to Download page
3. Paste Ubuntu magnet link:
   ```
   magnet:?xt=urn:btih:5a1e8e4e2b3c9d8f7a6e5d4c3b2a1e0f9d8c7b6a&dn=ubuntu-22.04.3-desktop-amd64.iso
   ```
4. Click "Search & Download"
5. Wait for metadata resolution

**Expected Results:**
- ✅ Magnet link parsed successfully
- ✅ Info hash extracted: `5a1e8e4e...`
- ✅ DHT query initiated
- ✅ Metadata received (file name, size, piece info)
- ✅ Seeder list populated
- ✅ Peer selection modal displayed

**Pass Criteria:**
- No errors in console
- File metadata matches Ubuntu ISO
- At least 1 seeder found

**Debugging:**
```bash
# Check DHT queries
tail -f ~/.chiral/logs/dht.log | grep "magnet"

# Verify info_hash
echo "5a1e8e4e2b3c9d8f7a6e5d4c3b2a1e0f9d8c7b6a" | wc -c
# Should output: 40 (20 bytes in hex)
```

---

### Test 2: .torrent File Import 📁

**Objective:** Verify .torrent file parsing

**Steps:**
1. Download Ubuntu .torrent file
2. In Chiral Network, go to Download page
3. Click "Import .torrent" button
4. Select downloaded .torrent file
5. Review parsed metadata

**Expected Results:**
- ✅ File parsed without errors
- ✅ Info hash calculated
- ✅ Piece hashes extracted
- ✅ Tracker list ignored (DHT-only mode)
- ✅ File name and size correct

**Pass Criteria:**
- Info hash matches magnet link version
- All pieces accounted for
- Chiral format metadata created

**File Structure Check:**
```javascript
// Expected parsed structure
{
  infoHash: "5a1e8e4e2b3c9d8f7a6e5d4c3b2a1e0f9d8c7b6a",
  name: "ubuntu-22.04.3-desktop-amd64.iso",
  length: 4697620480,
  pieceLength: 262144,
  pieces: ["hash1", "hash2", ...] // 17920 pieces
}
```

---

### Test 3: Download from BitTorrent Network 📥

**Objective:** Download file from BitTorrent peers

**Steps:**
1. Use magnet link or .torrent for well-seeded file
2. Start download in Chiral Network
3. Monitor download progress
4. Wait for completion
5. Verify file integrity

**Expected Results:**
- ✅ Connected to multiple BitTorrent peers
- ✅ Chunks downloaded in parallel
- ✅ Progress updates every second
- ✅ Download speed > 1 MB/s (with good seeders)
- ✅ File completes successfully
- ✅ SHA-256 hash verified

**Monitoring:**
```bash
# Watch network connections
netstat -an | grep ESTABLISHED | grep 6881

# Check download progress
curl http://localhost:8080/api/downloads/status

# Verify peers
curl http://localhost:8080/api/peers | jq '.[] | select(.protocol == "bittorrent")'
```

**Performance Metrics:**
- Download speed: > 1 MB/s
- Peer connections: > 5 active
- Chunk failure rate: < 5%

---

### Test 4: Seed to BitTorrent Clients 📤

**Objective:** Verify Chiral node can seed to BitTorrent clients

**Setup:**
1. Add test file to Chiral Network (Upload page)
2. Generate .torrent file for same content
3. Open qBittorrent
4. Add .torrent file to qBittorrent
5. Monitor qBittorrent's peer list

**Steps:**
1. Verify Chiral node appears in qBittorrent peer list
2. Start download in qBittorrent
3. Monitor upload in Chiral
4. Verify qBittorrent completes download

**Expected Results:**
- ✅ Chiral node discovered by qBittorrent
- ✅ BitTorrent handshake successful
- ✅ Pieces uploaded from Chiral to qBittorrent
- ✅ Upload speed tracked in Chiral analytics
- ✅ qBittorrent verifies all pieces
- ✅ Download completes successfully

**Verification in qBittorrent:**
```
Right-click torrent → Properties → Peers
- Should see Chiral node (IP:PORT)
- Client: "Chiral/1.0.0"
- Progress: Increasing
```

---

### Test 5: Cross-Network Seeding 🌐

**Objective:** File available on both networks simultaneously

**Steps:**
1. Add large file (500 MB+) to Chiral Network
2. Export as .torrent file
3. Verify file discoverable via:
   - Chiral hash search
   - BitTorrent magnet link
   - BitTorrent DHT query
4. Download same file using:
   - Another Chiral node
   - qBittorrent client
5. Monitor seeder count in both

**Expected Results:**
- ✅ File discoverable in both DHTs
- ✅ Chiral node counts as seeder in BitTorrent swarm
- ✅ Both Chiral and BitTorrent clients can download
- ✅ Upload bandwidth shared between protocols
- ✅ Reputation system tracks both types of peers

**DHT Verification:**
```bash
# Query BitTorrent DHT
dht-tool query <info_hash>

# Query Chiral DHT
curl http://localhost:8080/api/dht/query/<file_hash>

# Compare results
# Both should return Chiral node as seeder
```

---

### Test 6: Reputation Integration 🎯

**Objective:** Verify reputation system works with BitTorrent peers

**Steps:**
1. Download file from mixed peer set (Chiral + BitTorrent)
2. Note reputation scores before download
3. Complete download
4. Check updated reputation scores

**Expected Results:**
- ✅ BitTorrent peers get reputation entries
- ✅ Successful chunk transfers increase reputation
- ✅ Failed chunks decrease reputation
- ✅ Peer selection prioritizes high-reputation peers
- ✅ Reputation persists across sessions

**Reputation Checks:**
```javascript
// Check peer reputation
GET /api/reputation/peers

// Expected structure
{
  "peers": [
    {
      "peerId": "QmBitTorrentPeer...",
      "protocol": "bittorrent",
      "reputation": 85,
      "successfulTransfers": 42,
      "failedTransfers": 3,
      "avgBandwidth": 2048000
    }
  ]
}
```

---

### Test 7: Protocol Translation 🔄

**Objective:** Verify seamless protocol conversion

**Steps:**
1. Upload file via Chiral (Bitswap format)
2. Download same file via BitTorrent client
3. Verify piece-to-chunk translation
4. Check merkle proof compatibility

**Expected Results:**
- ✅ Chiral chunks mapped to BitTorrent pieces
- ✅ Piece boundaries aligned correctly
- ✅ Hash verification works for both formats
- ✅ No data corruption
- ✅ Performance overhead < 10%

**Translation Verification:**
```rust
// Verify piece mapping
Chiral chunk size: 256 KB
BitTorrent piece size: 256 KB (or multiple)

// Example:
// Chiral: chunks[0..1000]
// BitTorrent: pieces[0..1000]
// Mapping: 1:1
```

---

### Test 8: Privacy Mode 🔒

**Objective:** Verify proxy/relay works with BitTorrent

**Steps:**
1. Enable anonymous mode in Settings
2. Enable SOCKS5 proxy or Circuit Relay
3. Download file via BitTorrent
4. Verify traffic routed through proxy

**Expected Results:**
- ✅ BitTorrent traffic routed via proxy
- ✅ Real IP not exposed to BitTorrent peers
- ✅ DHT queries use proxy
- ✅ Download still works
- ✅ Performance acceptable (<20% overhead)

**Privacy Verification:**
```bash
# Check if direct BitTorrent connections exist
netstat -an | grep 6881 | grep ESTABLISHED
# Should be 0 in anonymous mode

# Verify proxy usage
curl --socks5 localhost:9050 https://api.ipify.org
# Should return proxy IP, not real IP
```

---

### Test 9: Error Handling 🚨

**Objective:** Verify graceful degradation

**Test Cases:**

#### 9a. Invalid Magnet Link
```
Input: magnet:?xt=urn:btih:INVALID
Expected: Error message, no crash
```

#### 9b. No Seeders
```
Input: Magnet link for rare/dead torrent
Expected: "No seeders found" message
```

#### 9c. Corrupted .torrent File
```
Input: Invalid bencode data
Expected: Parse error, user-friendly message
```

#### 9d. Network Disconnection
```
Action: Disable network mid-download
Expected: Pause, resume when reconnected
```

#### 9e. Malicious Peer
```
Action: Connect to peer sending invalid data
Expected: Peer blacklisted, reputation penalty
```

---

### Test 10: Performance Benchmark ⚡

**Objective:** Measure performance impact

**Baseline (Native BitTorrent):**
```bash
# Download Ubuntu ISO with qBittorrent
Time: X seconds
Speed: Y MB/s
CPU: Z%
Memory: A MB
```

**Chiral with BitTorrent:**
```bash
# Download same file with Chiral
Time: X seconds (+/- 10%)
Speed: Y MB/s (+/- 10%)
CPU: Z% + overhead
Memory: A MB + overhead
```

**Acceptance Criteria:**
- ⏱️ Time: Within 10% of native
- 📊 Speed: Within 10% of native
- 💻 CPU overhead: < 15%
- 🧠 Memory overhead: < 50 MB

---

## Regression Testing

Verify existing Chiral features still work:

### R1. Chiral-to-Chiral Transfer
- ✅ Upload file
- ✅ Download via hash
- ✅ Bitswap chunks work
- ✅ Encryption works
- ✅ Reputation system works

### R2. DHT Functionality
- ✅ Peer discovery
- ✅ File metadata lookup
- ✅ Bootstrap nodes reachable
- ✅ NAT traversal works

### R3. GUI Features
- ✅ Download queue
- ✅ Upload management
- ✅ Settings persist
- ✅ Analytics update

---

## Continuous Testing

### Automated Tests
```bash
# Run test suite
npm test -- --grep "bittorrent"

# Expected tests:
# - Magnet link parsing
# - .torrent file parsing
# - Info hash calculation
# - Piece mapping
# - Protocol translation
```

### Integration Tests
```bash
# Docker compose for multi-node testing
docker-compose -f tests/docker-compose.bittorrent.yml up

# Runs:
# - Chiral node 1 (seeder)
# - Chiral node 2 (leecher)
# - qBittorrent (leecher)
# - Transmission (leecher)

# Verifies cross-compatibility
```

---

## Success Criteria Summary

Feature is ready for release when:

- [ ] All 10 test cases pass
- [ ] Regression tests pass
- [ ] Performance within 10% of native BitTorrent
- [ ] No security vulnerabilities found
- [ ] Documentation complete
- [ ] User feedback positive (beta testing)

---

## Troubleshooting

### Common Issues

**Issue: Can't connect to BitTorrent peers**
```bash
# Check DHT bootstrap
curl http://localhost:8080/api/dht/status

# Verify port forwarding
sudo ufw status | grep 6881

# Test connectivity
nc -zv router.bittorrent.com 6881
```

**Issue: Slow downloads from BitTorrent**
```bash
# Check peer count
curl http://localhost:8080/api/peers | grep bittorrent | wc -l

# Verify bandwidth limits
curl http://localhost:8080/api/settings | jq .bandwidth

# Test direct connection
transmission-remote -l
```

**Issue: .torrent parsing fails**
```bash
# Validate file
bencode-tool decode torrent.torrent

# Check for v2 torrent (not supported in initial version)
grep "meta version" torrent.torrent
```

---

## Test Environment Setup

### Quick Setup Script

```bash
#!/bin/bash
# setup-test-env.sh

# Install qBittorrent
sudo apt install qbittorrent-nox

# Download test torrent
wget https://releases.ubuntu.com/22.04/ubuntu-22.04.3-desktop-amd64.iso.torrent

# Start Chiral in test mode
cd chiral-network
npm run dev -- --test-mode

# Start qBittorrent
qbittorrent-nox &

echo "Test environment ready!"
echo "Chiral: http://localhost:1420"
echo "qBittorrent: http://localhost:8080"
```

---

## Reporting Issues

When reporting bugs, include:

1. **Test case** that failed
2. **Expected** vs **actual** results
3. **Logs** from `~/.chiral/logs/`
4. **Network capture** (if relevant)
5. **System info** (OS, versions)

Example:
```
Test: Test 4 - Seed to BitTorrent Clients
Expected: qBittorrent sees Chiral peer
Actual: Peer list empty
Logs: [attach dht.log]
System: Ubuntu 22.04, Chiral v1.0.0
```

---

**Note:** This testing guide assumes BitTorrent compatibility feature has been approved and implemented. If implementing a different interpretation of "multiple protocols," adjust tests accordingly.
