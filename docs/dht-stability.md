# DHT Stability - Technical Proposal

## Document Status

**Status:** Proposal  
**Version:** 1.0  
**Date:** 2024-11-24  
**Author:** Chiral Network Team

---

## Executive Summary

This document outlines the challenges and solutions related to DHT (Distributed Hash Table) stability in the Chiral Network, with particular focus on the **private IP vs public IP address discoverability issue** that affects peer-to-peer file sharing. It details how the network's implementation of Circuit Relay v2, AutoNAT v2, and related protocols addresses these challenges.

### Key Issues Addressed

1. **Private IP Address Problem**: Nodes behind NAT cannot be discovered via DHT using their private addresses
2. **Provider Record Reachability**: Provider announcements fail when nodes lack public addresses
3. **Bootstrap Node Stability**: Single points of failure in network bootstrapping
4. **DHT Record Persistence**: Challenges maintaining provider records over time
5. **Network Partition Risks**: Isolated network segments due to NAT configurations

---

## 1. Overview of DHT Stability Challenges

### 1.1 What is DHT Stability?

DHT stability refers to the reliability and consistency of the distributed hash table used for:

- **Peer Discovery**: Finding other nodes in the network
- **Content Discovery**: Locating files and their providers
- **Metadata Storage**: Storing file information and routing data
- **Provider Records**: Maintaining lists of nodes serving specific content

A stable DHT ensures that:
- Peers can reliably discover each other
- File providers can be located consistently
- Network topology adapts to node churn (joins/leaves)
- Provider records remain fresh and accurate

### 1.2 Common DHT Stability Issues

#### Network-Level Issues
- **NAT Traversal Failures**: Peers cannot establish direct connections
- **Bootstrap Node Downtime**: Network becomes unreachable
- **Routing Table Churn**: Frequent node departures destabilize routing
- **Sybil Attacks**: Malicious nodes pollute routing tables

#### Content-Level Issues
- **Stale Provider Records**: Outdated information about file availability
- **Record Expiration**: Provider entries timing out prematurely
- **Search Failures**: Unable to locate known content
- **Split Brain**: Network partitions causing inconsistent views

---

## 2. The Private IP vs Public IP Problem

### 2.1 Problem Statement

**The Core Issue**: Nodes behind NAT (Network Address Translation) have private IP addresses that cannot be directly reached from the public internet, making them undiscoverable even when they announce themselves as content providers in the DHT.

#### Technical Details

When a node publishes a file to the Chiral Network, it performs two operations:

1. **Store Metadata Record**: Stores `FileMetadata` in the DHT under the file's Merkle root
2. **Announce as Provider**: Calls `kademlia.start_providing(file_key)` to register as a provider

The provider announcement includes:
- **Peer ID**: Unique cryptographic identifier (works globally)
- **Multiaddresses**: Network addresses where the peer can be reached (problem!)

#### Example Scenario

```
Node A (Behind NAT):
  Peer ID: 12D3KooWxyz...
  Private IP: 192.168.1.100:4001
  Public IP: None (NAT'd)
  
DHT Provider Record:
  File: QmFileHash...
  Provider: 12D3KooWxyz...
  Addresses: [/ip4/192.168.1.100/tcp/4001]
  
Node B (Searching):
  Queries DHT for QmFileHash...
  Receives: Provider 12D3KooWxyz... at /ip4/192.168.1.100/tcp/4001
  Attempts connection: FAILS ❌
  Reason: 192.168.1.100 is a private address, unreachable from Node B's network
```

### 2.2 Why Private IPs Cannot Be Stored in DHT

#### RFC 1918 Private Address Ranges

The following IP ranges are reserved for private networks and are **not routable** on the public internet:

- **10.0.0.0/8**: 10.0.0.0 - 10.255.255.255
- **172.16.0.0/12**: 172.16.0.0 - 172.31.255.255
- **192.168.0.0/16**: 192.168.0.0 - 192.168.255.255
- **127.0.0.0/8**: Loopback addresses (localhost)

#### Consequences for DHT

When private addresses are stored in provider records:

1. **Discovery Succeeds**: Peers can find the provider record via DHT lookup
2. **Connection Fails**: Peers cannot connect to private addresses
3. **False Positives**: Files appear available but are unreachable
4. **Wasted Bandwidth**: Repeated failed connection attempts
5. **Poor User Experience**: Downloads timeout or fail mysteriously

### 2.3 Real-World Impact

#### Statistics (Estimated)

Based on typical P2P network demographics:

- **60-70%** of home users are behind NAT
- **90%+** of mobile users are behind carrier-grade NAT (CGNAT)
- **40-50%** of corporate users are behind restrictive NATs

**Result**: Without NAT traversal solutions, the majority of potential peers become unreachable, severely limiting network capacity and file availability.

### 2.4 Chiral Network's Implementation

#### Current Address Filtering

The Chiral Network implements address filtering to prevent private IPs from being used as relay candidates:

```rust
/// Check if a multiaddress is plausibly reachable from remote peers
/// - Relay paths (p2p-circuit) are allowed
/// - IPv4 loopback (127.0.0.1) is REJECTED
/// - Private IPv4 addresses are REJECTED
fn ma_plausibly_reachable(ma: &Multiaddr) -> bool {
    // Relay paths are allowed (these work across NATs)
    if ma.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
        return true;
    }
    
    // Check for IPv4 addresses
    if let Some(Protocol::Ip4(v4)) = ma.iter().find(|p| matches!(p, Protocol::Ip4(_))) {
        // Reject loopback
        if v4.is_loopback() {
            return false;
        }
        // Allow public addresses, reject private
        return !v4.is_private();
    }
    
    false
}
```

This filtering is applied to:
- Relay candidate selection
- Bootstrap node validation
- AutoNAT server configuration

#### Provider Record Behavior

When a NAT'd node calls `start_providing()`:

1. libp2p Kademlia stores the **Peer ID** in the provider record
2. Remote peers querying the DHT receive the **Peer ID** only
3. To connect, remote peers need to discover **reachable addresses** for that Peer ID
4. Without Circuit Relay, this discovery fails for private addresses

---

## 3. How Circuit Relay v2 Solves the Problem

### 3.1 Circuit Relay Overview

Circuit Relay v2 is a libp2p protocol that enables NAT'd peers to be reachable through public relay nodes, providing an end-to-end encrypted communication path.

#### Key Concepts

- **Relay Node**: A publicly reachable node that forwards traffic
- **Circuit Relay Address**: Special multiaddress containing `/p2p-circuit`
- **Reservation**: NAT'd peer reserves a slot on the relay node
- **Circuit Connection**: Encrypted tunnel through the relay

### 3.2 Circuit Relay Address Format

Circuit relay addresses use a special format that works across NATs:

```
/ip4/relay.chiral.network/tcp/4001/p2p/QmRelayPeer.../p2p-circuit/p2p/QmNATdPeer...
│                                    │                │           │
│                                    │                │           └─ Target peer ID
│                                    │                └─ Circuit marker
│                                    └─ Relay peer ID
└─ Relay public address
```

**Key Properties**:
- ✅ **Globally reachable**: Uses relay's public IP
- ✅ **NAT-proof**: No direct connection needed
- ✅ **End-to-end encrypted**: Relay cannot read content
- ✅ **DHT-compatible**: Can be stored and distributed

### 3.3 How It Solves Private IP Problem

#### Step-by-Step Flow

**1. NAT'd Node Initialization**

```
Node A (Behind NAT):
1. Connects to bootstrap node
2. Discovers relay-capable peers via identify protocol
3. Requests relay reservation
4. Receives relay circuit address
```

**2. Provider Announcement with Circuit Address**

```
Node A announces file:
  File: QmFileHash...
  Provider: 12D3KooWxyz... (Node A's Peer ID)
  
Node A's reachable addresses now include:
  - /ip4/relay.example.com/tcp/4001/p2p/QmRelay.../p2p-circuit/p2p/12D3KooWxyz...
  
DHT provider record effectively contains:
  File: QmFileHash...
  Provider: 12D3KooWxyz...
  (Peer addresses discovered via libp2p identify protocol)
```

**3. Remote Peer Connection**

```
Node B (Searching for file):
1. Queries DHT for QmFileHash...
2. Receives provider: 12D3KooWxyz...
3. Queries identify protocol for 12D3KooWxyz...'s addresses
4. Discovers relay circuit address
5. Connects via relay: SUCCESS ✅
```

### 3.4 Technical Implementation Details

#### Address Discovery via Identify Protocol

libp2p's **identify protocol** allows peers to exchange their listen addresses. When Node A has an active relay reservation, its identify information includes:

```rust
identify::Info {
    peer_id: "12D3KooWxyz...",
    listen_addrs: vec![
        // Private address (filtered out by remote peers)
        "/ip4/192.168.1.100/tcp/4001",
        
        // Relay circuit address (globally reachable!)
        "/ip4/relay.example.com/tcp/4001/p2p/QmRelay.../p2p-circuit",
    ],
    protocols: vec![
        "/chiral/transfer/1.0.0",
        "/ipfs/kad/1.0.0",
        // ... other protocols
    ],
    observed_addr: Some("/ip4/203.0.113.42/tcp/54321"), // External address observed by relay
}
```

Remote peers automatically filter out unreachable private addresses and use relay circuit addresses for connections.

#### Provider Record Workflow

```
1. Node A: kademlia.start_providing(file_key)
   → Stores provider record with Peer ID only
   
2. Node B: kademlia.get_providers(file_key)
   → Receives list of provider Peer IDs
   
3. Node B: For each provider Peer ID
   → Sends identify request to discover addresses
   → Receives identify response with relay circuit addresses
   → Connects via relay if no direct path available
   
4. Node B: Successful connection via relay
   → Begins file transfer through circuit
   → (Optional: Attempts hole punching for direct upgrade via DCUtR)
```

---

## 4. AutoNAT v2 - Detecting Reachability

### 4.1 Purpose

AutoNAT v2 is a **reachability detection protocol** that determines whether a node is publicly reachable or behind NAT. This information is crucial for:

- Deciding whether to request relay reservations
- Displaying accurate network status to users
- Optimizing connection strategies

**Important**: AutoNAT does **NOT** provide NAT traversal - it only detects NAT status.

### 4.2 How It Works

```
Node A (Unknown reachability):
1. Observes its own addresses via libp2p identify
2. Asks remote peers to dial back on those addresses
3. Remote peers attempt connection
4. If dialback succeeds → Public
5. If dialback fails → Private (behind NAT)
```

#### Confidence Scoring

AutoNAT v2 uses confidence scoring to handle uncertain reachability:

```rust
pub enum NatStatus {
    Public,         // High confidence: publicly reachable
    Private,        // High confidence: behind NAT
    Unknown,        // Low confidence: inconclusive tests
}

pub struct ReachabilityInfo {
    pub status: NatStatus,
    pub confidence: f64,        // 0.0 to 1.0
    pub last_check: Instant,
}
```

### 4.3 Why Relay Addresses Are Allowed in AutoNAT Context

The code comment states:
> Cannot use relay connections for dial-back (libp2p security requirement)

This means:
- AutoNAT **cannot test reachability through relay circuits**
- Relay circuits are **always assumed reachable** (because relay is public)
- Private IPs must be tested with direct connections only

The `ma_plausibly_reachable()` function allows relay addresses because:
1. Relay addresses are **always globally reachable** by definition
2. They don't need reachability testing (the relay itself is public)
3. They are valid for provider announcements

---

## 5. Current Implementation Status

### 5.1 Implemented Features ✅

#### NAT Traversal Infrastructure
- ✅ Circuit Relay v2 client support
- ✅ Circuit Relay v2 server mode (opt-in)
- ✅ AutoRelay for automatic relay discovery
- ✅ AutoNAT v2 for reachability detection
- ✅ Private address filtering in relay candidate selection
- ✅ Relay health monitoring
- ✅ DCUtR (Direct Connection Upgrade through Relay) for hole punching

#### DHT Operations
- ✅ Kademlia DHT with file metadata storage
- ✅ Provider record announcements
- ✅ Periodic provider record refresh (heartbeats)
- ✅ DHT record expiration handling
- ✅ Bootstrap node connectivity
- ✅ Routing table management

#### Address Management
- ✅ Observed address tracking via identify protocol
- ✅ External address configuration
- ✅ Relay circuit address construction
- ✅ Private IP filtering for relay candidates
- ✅ Listen address management

### 5.2 Verification of Private IP Filtering

#### Code Location: `src-tauri/src/dht.rs`

**Relay Candidate Filtering** (Lines ~1370-1407):
```rust
for cand in relay_candidates {
    if let Ok(ma) = cand.parse::<Multiaddr>() {
        // Skip unreachable addresses (localhost/private IPs)
        if !ma_plausibly_reachable(&ma) {
            tracing::debug!("Skipping unreachable relay candidate: {}", ma);
            continue;
        }
        // ... process only reachable addresses
    }
}
```

**Reachability Check** (Lines ~1440-1465):
```rust
fn ma_plausibly_reachable(ma: &Multiaddr) -> bool {
    // Relay paths are allowed
    if ma.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
        return true;
    }
    // Only consider IPv4
    if let Some(Protocol::Ip4(v4)) = ma.iter().find(|p| matches!(p, Protocol::Ip4(_))) {
        // Reject loopback addresses
        if v4.is_loopback() {
            return false;
        }
        // Allow public addresses, reject private
        return !v4.is_private();
    }
    false
}
```

This filtering is applied at relay candidate selection time, preventing private IPs from being used as relay endpoints.

### 5.3 How Provider Discovery Works Today

#### Current Flow (Verified)

1. **File Publishing**:
   ```rust
   // Peer publishes file metadata to DHT
   swarm.behaviour_mut().kademlia.put_record(record, Quorum::One);
   
   // Peer announces as provider
   swarm.behaviour_mut().kademlia.start_providing(provider_key);
   ```

2. **Provider Record Content**:
   - Contains only **Peer ID** (not full addresses)
   - Addresses discovered separately via identify protocol
   - Circuit relay addresses included in identify response

3. **File Search**:
   ```rust
   // Search for file
   swarm.behaviour_mut().kademlia.get_record(file_key);
   
   // Get providers
   swarm.behaviour_mut().kademlia.get_providers(provider_key);
   ```

4. **Connection Establishment**:
   - Remote peer receives provider Peer ID
   - Sends identify request to provider
   - Receives addresses (including relay circuits)
   - Attempts connection prioritizing direct, falling back to relay

---

## 6. Best Practices and Recommendations

### 6.1 For Node Operators

#### Detecting Your NAT Status

Check your reachability status in the Network page:

```
Network Status:
  NAT Status: Private (Behind NAT)
  Confidence: High (95%)
  Active Relays: 2
  Circuit Reservations: 2/2
```

#### When Behind NAT

**Recommended Configuration**:
- ✅ Enable AutoRelay (Settings → Network → Enable AutoRelay)
- ✅ Configure multiple relay nodes for redundancy
- ✅ Monitor relay health regularly
- ✅ Keep relay reservations active

**Optional - Run as Relay Server**:
If you have a public IP, consider:
- ✅ Enable Relay Server Mode (Settings → Network → Enable Relay Server)
- ✅ Configure reservation limits
- ✅ Monitor bandwidth usage

#### When Publicly Reachable

**Recommended Configuration**:
- ✅ Consider enabling Relay Server Mode to help the network
- ✅ Configure AutoNAT servers for reachability verification
- ✅ Set external address if behind static NAT

### 6.2 For Network Administrators

#### Bootstrap Node Setup

**Critical Requirements**:
- ✅ Must have public IP address
- ✅ Must have stable uptime (99.9%+)
- ✅ Must be accessible on standard ports
- ✅ Should enable relay server mode

**Monitoring**:
- Health check scripts (`bootstrap-health.sh`)
- Systemd auto-recovery
- Connection rate monitoring
- Routing table size tracking

#### Relay Infrastructure

**Deployment Strategy**:
- Multiple relay nodes in different geographic regions
- Load balancing across relay nodes
- Circuit limit configuration based on capacity
- Metrics collection (Prometheus integration available)

**Relay Node Requirements**:
- Public IP address (not behind NAT)
- Sufficient bandwidth for relaying traffic
- Stable network connection
- Low latency for better user experience

### 6.3 For Developers

#### Working with Provider Records

**Best Practices**:

```rust
// ✅ Good: Let libp2p handle address discovery
kademlia.start_providing(file_key).await?;
// Provider record stores Peer ID
// Addresses discovered via identify protocol automatically

// ❌ Bad: Don't manually construct provider records with private IPs
// This is handled automatically by libp2p

// ✅ Good: Filter relay candidates
let reachable_candidates: Vec<_> = candidates
    .into_iter()
    .filter(|addr| ma_plausibly_reachable(&addr))
    .collect();
```

#### Handling Connection Failures

```rust
// Implement connection fallback strategy
async fn connect_to_provider(peer_id: PeerId, swarm: &mut Swarm) -> Result<()> {
    // 1. Try direct connection
    if let Ok(_) = swarm.dial(peer_id) {
        return Ok(());
    }
    
    // 2. Discover relay addresses via identify
    let addresses = discover_peer_addresses(peer_id, swarm).await?;
    
    // 3. Try relay circuit addresses
    for addr in addresses {
        if addr.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
            if let Ok(_) = swarm.dial(addr) {
                return Ok(());
            }
        }
    }
    
    Err(Error::UnreachablePeer)
}
```

#### Testing NAT Scenarios

**Test Cases**:

1. **Both Peers Public**: Direct connection should succeed
2. **One Peer NAT'd**: Relay connection should succeed
3. **Both Peers NAT'd**: Relay connection through common relay
4. **Symmetric NAT**: DCUtR hole punching attempt, fallback to relay
5. **Relay Failure**: Graceful degradation with error messages

---

## 7. Future Improvements

### 7.1 Short-Term (Phase 3)

#### Enhanced Discovery
- [ ] IPv6 support in reachability checks
- [ ] Multiple relay fallback strategies
- [ ] Smart relay selection based on latency
- [ ] Relay reputation system

#### Performance Optimizations
- [ ] Direct connection upgrade via DCUtR after relay handshake
- [ ] WebRTC data channels for browser compatibility
- [ ] Relay circuit caching for frequently contacted peers
- [ ] Provider record TTL optimization

### 7.2 Medium-Term (Phase 4)

#### Network Resilience
- [ ] Dynamic relay discovery without bootstrap
- [ ] Mesh relay networks for redundancy
- [ ] Automatic relay migration on failure
- [ ] Provider record replication across multiple DHT nodes

#### Advanced NAT Traversal
- [ ] UPnP/NAT-PMP for automatic port forwarding
- [ ] STUN server integration for reflexive address discovery
- [ ] ICE-like candidate gathering for optimal connection paths
- [ ] Adaptive relay usage based on network conditions

### 7.3 Long-Term (Phase 5+)

#### Distributed Relay Infrastructure
- [ ] Incentivized relay operation (rewards for relay nodes)
- [ ] Decentralized relay discovery via DHT
- [ ] Relay load balancing algorithms
- [ ] Geographic relay distribution optimization

#### Protocol Enhancements
- [ ] QUIC transport for better NAT traversal
- [ ] Gossipsub for relay health announcements
- [ ] Custom DHT record types for provider metadata
- [ ] Proof-of-relay for relay verification

---

## 8. Related Documentation

### Primary References

- **[nat-traversal.md](./nat-traversal.md)**: Comprehensive NAT traversal implementation guide
  - Circuit Relay v2 configuration
  - AutoNAT v2 setup
  - Relay server deployment
  - Headless mode configuration

- **[network-protocol.md](./network-protocol.md)**: Network protocol specifications
  - DHT protocol details
  - Peer discovery protocol
  - Provider record format
  - Message serialization

- **[bootstrap-stability.md](./bootstrap-stability.md)**: Bootstrap node stability
  - Connection timeout handling
  - Health monitoring
  - Discovery verification
  - Auto-recovery mechanisms

### Related Topics

- **[architecture.md](./architecture.md)**: System architecture overview
- **[deployment-guide.md](./deployment-guide.md)**: Production deployment
- **[security-privacy.md](./security-privacy.md)**: Security and privacy features
- **[relay/DEPLOYMENT.md](../relay/DEPLOYMENT.md)**: Advanced relay deployment

---

## 9. Troubleshooting

### 9.1 Common Issues

#### Issue: "No peers found for file"

**Symptoms**: DHT query succeeds but no providers returned

**Diagnosis**:
```bash
# Check DHT connectivity
chiral-cli network dht-status

# Verify routing table
chiral-cli network routing-table-size

# Check bootstrap connection
chiral-cli network bootstrap-health
```

**Solutions**:
1. Verify bootstrap node connectivity
2. Wait for DHT to populate (30-60 seconds after start)
3. Check provider record TTL (default: 24 hours)
4. Ensure file is still being seeded

#### Issue: "Connection failed to provider"

**Symptoms**: Provider found but connection times out

**Diagnosis**:
```bash
# Check peer addresses
chiral-cli network peer-info <peer-id>

# Check NAT status
chiral-cli network nat-status

# Verify relay connectivity
chiral-cli network relay-status
```

**Solutions**:
1. Verify provider has active relay reservation
2. Check your relay connectivity
3. Ensure provider isn't behind symmetric NAT without relay
4. Try adding more relay nodes

#### Issue: "Relay reservation failed"

**Symptoms**: Cannot establish relay circuit

**Diagnosis**:
```bash
# Check relay candidates
chiral-cli network relay-candidates

# Check relay health
chiral-cli network relay-health <relay-peer-id>
```

**Solutions**:
1. Verify relay nodes are public and reachable
2. Check relay circuit limits aren't exceeded
3. Try different relay nodes
4. Wait and retry (transient network issues)

### 9.2 Debug Logging

Enable debug logging for DHT issues:

```bash
# Enable DHT debug logs
RUST_LOG=chiral_network::dht=debug chiral-network

# Enable relay debug logs
RUST_LOG=libp2p_relay=debug chiral-network

# Enable identify protocol logs
RUST_LOG=libp2p_identify=debug chiral-network

# Enable all libp2p logs
RUST_LOG=libp2p=debug chiral-network
```

### 9.3 Health Check Commands

```bash
# Comprehensive health check
chiral-cli network health-check

# DHT-specific checks
chiral-cli network dht-verify-discovery

# Relay-specific checks
chiral-cli network relay-verify

# Bootstrap checks
chiral-cli network bootstrap-verify
```

---

## 10. Conclusion

### Key Takeaways

1. **Private IP Problem is Real**: ~60-70% of users are affected by NAT, making direct connections impossible without solutions

2. **Circuit Relay Solves It**: Circuit Relay v2 provides globally reachable addresses for NAT'd peers through public relay nodes

3. **AutoNAT Complements**: AutoNAT v2 detects reachability status, enabling automatic relay activation for NAT'd nodes

4. **Current Implementation Works**: Chiral Network's implementation includes:
   - Private IP filtering for relay candidates
   - Circuit relay address construction
   - Provider discovery via identify protocol
   - Automatic relay fallback

5. **Continuous Improvement**: Future enhancements will focus on:
   - Better relay discovery
   - Direct connection upgrades
   - Performance optimizations
   - Decentralized relay infrastructure

### Success Criteria

The DHT stability implementation is considered successful when:

- ✅ NAT'd peers can publish and discover files reliably
- ✅ Provider records contain only reachable addresses
- ✅ Connection success rate >95% for all network configurations
- ✅ Relay fallback happens transparently
- ✅ Network remains stable during node churn
- ✅ Bootstrap failures recover automatically (<2 minutes)

### Call to Action

**For Users**:
- Enable AutoRelay if behind NAT
- Configure multiple relay nodes
- Consider running relay server if publicly reachable

**For Developers**:
- Follow best practices in provider record handling
- Test with various NAT configurations
- Contribute relay infrastructure improvements

**For Operators**:
- Deploy reliable bootstrap nodes
- Run public relay nodes
- Monitor network health metrics

---

## Appendix A: Technical Glossary

**AutoNAT v2**: libp2p protocol for detecting if a node is publicly reachable or behind NAT

**Bootstrap Node**: Initial connection point for joining the DHT network

**Circuit Relay v2**: libp2p protocol for relaying traffic through public nodes to reach NAT'd peers

**DHT (Distributed Hash Table)**: Distributed key-value store used for peer and content discovery

**Identify Protocol**: libp2p protocol for exchanging peer information including listen addresses

**Kademlia**: DHT implementation used by libp2p and Chiral Network

**Multiaddress**: libp2p's flexible address format that can encode network protocols, addresses, and peer IDs

**NAT (Network Address Translation)**: Technology that maps private IP addresses to public IPs, preventing direct inbound connections

**Peer ID**: Unique cryptographic identifier for a libp2p peer, derived from public key

**Provider Record**: DHT entry indicating which peers can serve specific content

**Relay Reservation**: Agreement between a NAT'd peer and a relay node to forward traffic

**Routing Table**: Kademlia data structure storing known peers organized by XOR distance

---

## Appendix B: Configuration Examples

### Example: Node Behind NAT

`~/.config/chiral-network/config.toml`:

```toml
[network]
enable_autonat = true
autonat_probe_interval = 60  # seconds

enable_autorelay = true
preferred_relays = [
    "/ip4/relay1.chiral.network/tcp/4001/p2p/12D3KooWRelay1...",
    "/ip4/relay2.chiral.network/tcp/4001/p2p/12D3KooWRelay2...",
]

[bootstrap]
bootstrap_nodes = [
    "/ip4/bootstrap.chiral.network/tcp/4001/p2p/12D3KooWBootstrap...",
]
```

### Example: Public Relay Node

```toml
[network]
enable_relay_server = true

[relay_server]
max_reservations = 128
max_circuits_per_peer = 16
reservation_duration = 3600  # seconds

[bootstrap]
bootstrap_nodes = [
    "/ip4/bootstrap.chiral.network/tcp/4001/p2p/12D3KooWBootstrap...",
]
```

### Example: Headless CLI

```bash
# NAT'd node with relay
chiral-network \
  --enable-autorelay \
  --relay /ip4/relay.chiral.network/tcp/4001/p2p/12D3KooWRelay... \
  --autonat-probe-interval 60

# Public node acting as relay server
chiral-network \
  --enable-relay-server \
  --external-address /ip4/203.0.113.42/tcp/4001
```

---

## Appendix C: Performance Metrics

### Expected Latency

| Connection Type | Typical Latency | Notes |
|----------------|----------------|--------|
| Direct (Public-Public) | 10-50ms | Best case |
| Direct (Public-NAT) | 50-100ms | Via hole punching |
| Relay Circuit | 100-300ms | One relay hop |
| Double Relay | 200-600ms | Both peers NAT'd |

### Bandwidth Impact

| Scenario | Overhead | Notes |
|----------|----------|--------|
| Direct Transfer | 0% | No overhead |
| Relay Transfer | 2x bandwidth | Relay receives and sends |
| Encrypted Relay | 2x + 5-10% | Encryption overhead |

### DHT Query Performance

| Operation | Target Time | Maximum Time |
|-----------|-------------|--------------|
| Provider Lookup | 500ms | 2s |
| Record Retrieval | 200ms | 1s |
| Provider Announcement | 1s | 5s |
| Routing Table Refresh | 5s | 30s |

---

## Document History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2024-11-24 | Initial proposal | Chiral Network Team |

---

**End of Document**
