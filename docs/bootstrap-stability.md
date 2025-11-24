# DHT & Bootstrap Stability - Technical Proposal

## 🔴 PRIORITY 1 - CRITICAL ISSUE

**Status:** Planning Phase | **Date:** 2024-11-24

---

## Executive Summary

This document addresses critical DHT and bootstrap stability issues affecting peer discoverability in Chiral Network. The main challenges are:

1. **Bootstrap node failures** causing infinite hangs and silent degradation
2. **Private IP problem** where NAT'd nodes (60-70% of users) are undiscoverable via DHT
3. **Discovery verification** ensuring peers can actually see each other

---

## Part 1: Bootstrap Node Stability

### Critical Issues

| Issue | Problem | Impact |
|-------|---------|--------|
| Infinite Hang | `connectPeer()` has no timeout | Users force-quit app |
| Silent Degradation | Bootstrap crashes after connection | Peers invisible for hours |
| Discovery Not Verified | DHT connected ≠ discovery working | Users can't find each other |

### Business Impact
- 30% connection failure rate
- 15 support tickets/week
- Hours of downtime when bootstrap crashes

### Solution #1: Connection Timeout

Wrap `connectPeer()` in `Promise.race()` with 10s timeout:

```javascript
// Network.svelte - connectWithTimeout()
async function connectWithTimeout(peer, timeout = 10000) {
  return Promise.race([
    connectPeer(peer),
    new Promise((_, reject) => 
      setTimeout(() => reject(new Error('Connection timeout')), timeout)
    )
  ]);
}
```

**Benefit:** Fail fast instead of infinite hang

### Solution #2: Health Monitoring

New `check_bootstrap_health()` command returns:
- Latency, reachability, peer count, routing table size

Frontend runs 30s interval health checks with auto-reconnect.

```rust
// bootstrap.rs
pub struct BootstrapHealth {
    pub latency: u64,
    pub reachable: bool,
    pub peer_count: usize,
    pub routing_table_size: usize,
}
```

**Benefit:** Detect issues in 30s, auto-recover

### Solution #3: Discovery Verification

6-step verification flow with 5 new Tauri commands:

1. Check routing table populated (`get_dht_routing_table_size`)
2. Publish test key via DHT
3. Query test key from peer perspective (`test_peer_discovery`)
4. List discovered peer IDs (`list_discovered_peer_ids`)
5. Verify bidirectional reachability (`verify_peer_reachable`)
6. Confirm provider records synced (`get_provider_records`)

**Benefit:** Proves peers can actually see each other

### Solution #4: Server Auto-Recovery

- Systemd timer checks bootstrap process every 1 minute
- Auto-restart on failure with monitoring alerts
- Downtime reduced from hours to <2min

### Success Metrics

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Connection Success Rate | 70% | 95%+ | 95% |
| Bootstrap Downtime | Hours | <2min | <5min |
| Discovery Failures | 20% | <1% | <2% |
| Issue Detection Time | Never | 30s | <60s |

---

## Part 2: The Private IP vs Public IP Problem

### Problem Statement

**Core Issue**: Nodes behind NAT have private IP addresses that cannot be reached from the public internet, making them undiscoverable even when they announce themselves as providers in the DHT.

### Why This Happens

When a NAT'd node publishes a file:
1. **Stores metadata** in DHT under file's Merkle root
2. **Announces as provider** via `kademlia.start_providing(file_key)`

The problem: Provider announcement includes unreachable private addresses.

```
Node A (Behind NAT):
  Peer ID: 12D3KooWxyz...
  Private IP: 192.168.1.100:4001  ← UNREACHABLE from internet
  
Node B (Searching):
  Finds provider 12D3KooWxyz...
  Tries to connect to 192.168.1.100:4001
  Result: CONNECTION FAILS ❌
```

### Private Address Ranges (RFC 1918)

These addresses are **not routable** on the public internet:

| Range | Addresses |
|-------|-----------|
| 10.0.0.0/8 | 10.0.0.0 - 10.255.255.255 |
| 172.16.0.0/12 | 172.16.0.0 - 172.31.255.255 |
| 192.168.0.0/16 | 192.168.0.0 - 192.168.255.255 |
| 127.0.0.0/8 | Loopback (localhost) |

### Real-World Impact

| User Type | Behind NAT | Impact |
|-----------|------------|--------|
| Home users | 60-70% | Cannot share files directly |
| Mobile users | 90%+ | Almost always behind CGNAT |
| Corporate | 40-50% | Restrictive NAT policies |

**Without NAT traversal**: Majority of peers become unreachable, severely limiting file availability.

---

## Part 3: Circuit Relay v2 Solution

### How It Works

Circuit Relay v2 enables NAT'd peers to be reachable through public relay nodes:

```
NAT'd Node A → Public Relay → Node B
              (encrypted tunnel)
```

### Circuit Relay Address Format

```
/ip4/relay.chiral.network/tcp/4001/p2p/QmRelayPeer.../p2p-circuit/p2p/QmNATdPeer...
│                                    │                │           │
│                                    │                │           └─ Target peer
│                                    │                └─ Circuit marker
│                                    └─ Relay peer ID
└─ Relay public address (REACHABLE!)
```

### Connection Flow

**1. NAT'd Node Setup:**
```
Node A (Behind NAT):
1. Connects to bootstrap node
2. Discovers relay-capable peers
3. Requests relay reservation
4. Gets relay circuit address
```

**2. File Announcement:**
```
Node A announces file:
  File: QmFileHash...
  Provider: 12D3KooWxyz...
  Reachable via: /ip4/relay.example.com/.../p2p-circuit/...
```

**3. Remote Peer Connects:**
```
Node B:
1. Queries DHT for file → Gets provider ID
2. Discovers relay circuit address via identify protocol
3. Connects through relay → SUCCESS ✅
```

### Key Properties

- ✅ **Globally reachable**: Uses relay's public IP
- ✅ **NAT-proof**: No direct connection needed
- ✅ **End-to-end encrypted**: Relay cannot read content
- ✅ **DHT-compatible**: Can be stored and distributed

---

## Part 4: Current Implementation

### Private IP Filtering

The network filters out unreachable addresses from relay candidates:

```rust
// src-tauri/src/dht.rs
fn ma_plausibly_reachable(ma: &Multiaddr) -> bool {
    // Relay paths are always allowed
    if ma.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
        return true;
    }
    
    if let Some(Protocol::Ip4(v4)) = ma.iter().find(|p| matches!(p, Protocol::Ip4(_))) {
        // Reject loopback and private addresses
        return !v4.is_loopback() && !v4.is_private();
    }
    
    false
}
```

**Applied to:**
- Relay candidate selection
- Bootstrap node validation
- AutoNAT server configuration

### AutoNAT v2 - Detecting Reachability

AutoNAT detects whether a node is publicly reachable or behind NAT:

```
Node A:
1. Observes own addresses via identify protocol
2. Asks remote peers to dial back
3. If dialback succeeds → Public
4. If dialback fails → Private (behind NAT)
```

**Note**: AutoNAT only detects NAT status - it doesn't provide traversal.

### Implemented Features

**NAT Traversal:**
- ✅ Circuit Relay v2 (client + server mode)
- ✅ AutoRelay for automatic relay discovery
- ✅ AutoNAT v2 for reachability detection
- ✅ DCUtR for hole punching
- ✅ Private address filtering

**DHT Operations:**
- ✅ Kademlia DHT with file metadata
- ✅ Provider record announcements
- ✅ Periodic heartbeat refresh
- ✅ Bootstrap node connectivity

---

## Part 5: Configuration

### For Users Behind NAT

**Recommended Settings:**
```toml
[network]
enable_autonat = true
enable_autorelay = true
preferred_relays = [
    "/ip4/relay1.chiral.network/tcp/4001/p2p/12D3KooW...",
    "/ip4/relay2.chiral.network/tcp/4001/p2p/12D3KooW...",
]
```

**CLI:**
```bash
chiral-network \
  --enable-autorelay \
  --relay /ip4/relay.chiral.network/tcp/4001/p2p/12D3KooW...
```

### For Public Nodes (Relay Server)

Help the network by running as a relay:
```toml
[network]
enable_relay_server = true

[relay_server]
max_reservations = 128
max_circuits_per_peer = 16
```

---

## Part 6: Troubleshooting

### "No peers found for file"

**Diagnosis:**
```bash
chiral-cli network dht-status
chiral-cli network routing-table-size
chiral-cli network bootstrap-health
```

**Solutions:**
1. Verify bootstrap node connectivity
2. Wait 30-60s for DHT to populate
3. Check provider record TTL
4. Ensure file is still being seeded

### "Connection failed to provider"

**Diagnosis:**
```bash
chiral-cli network peer-info <peer-id>
chiral-cli network nat-status
chiral-cli network relay-status
```

**Solutions:**
1. Verify provider has active relay reservation
2. Check your relay connectivity
3. Try adding more relay nodes

### "Relay reservation failed"

**Solutions:**
1. Verify relay nodes are public and reachable
2. Check relay circuit limits
3. Try different relay nodes

---

## Part 7: Implementation Checklist

### Phase 1: Bootstrap Timeout
- [ ] `connectWithTimeout()` wrapper
- [ ] Retry UI with error messages

### Phase 2: Health Monitoring
- [ ] `check_bootstrap_health()` command
- [ ] 30s interval health loop
- [ ] UI health indicator

### Phase 3: Discovery Verification
- [ ] 5 verification Tauri commands
- [ ] 6-step verification flow
- [ ] Diagnostics panel

### Phase 4: Server Auto-Recovery
- [ ] Health check bash script
- [ ] Systemd timer/service
- [ ] Monitoring alerts

### Phase 5: NAT Traversal
- [ ] Circuit Relay v2 integration
- [ ] AutoNAT v2 integration
- [ ] Private IP filtering validation

### Phase 6: Testing & Deploy
- [ ] All stability tests passing
- [ ] Load test 50+ concurrent users
- [ ] Documentation updates
- [ ] Production deployment

---

## Key Files

| File | Changes | Purpose |
|------|---------|---------|
| `Network.svelte` | ~50 lines | Timeout, health loop, verification |
| `bootstrap.rs` | ~150 lines | Health struct, Tauri commands |
| `dht.rs` | ~30 lines | Routing table, address filtering |
| `peerEventService.ts` | ~20 lines | Discovery events |
| Server scripts | ~100 lines | Auto-recovery, systemd |

---

## Conclusion

**Problems:**
1. Bootstrap failures → 30% connection failure rate
2. Private IPs in DHT → 60-70% of users undiscoverable
3. No verification → Peers can't find each other

**Solutions:**
1. Timeout + health monitoring + auto-recovery
2. Circuit Relay v2 with private IP filtering
3. 6-step discovery verification

**Outcome:**
- 95%+ connection success rate
- <2min bootstrap downtime
- NAT'd users fully discoverable via relay
- Guaranteed bidirectional peer visibility

---

## References

- [libp2p Circuit Relay](https://docs.libp2p.io/concepts/nat/circuit-relay/)
- [AutoNAT v2 Specification](https://github.com/libp2p/specs/blob/master/autonat/README.md)
- [Kademlia DHT](https://docs.libp2p.io/concepts/fundamentals/protocols/#kademlia)
- [RFC 1918 - Private IP Addresses](https://tools.ietf.org/html/rfc1918)
