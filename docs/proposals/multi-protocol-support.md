# Feature Proposal: Multi-Protocol Download Support

**Status:** DRAFT  
**Author:** Copilot Agent  
**Date:** October 21, 2025  
**Related Issue:** Phase 3 Roadmap Item - "Support for multiple protocols"

## Executive Summary

This proposal addresses the Phase 3 roadmap item "Support for multiple protocols, such as http, ftp, bittorrent, ed2k, etc." by analyzing whether this feature aligns with Chiral Network's core mission and proposing an alternative approach if needed.

## Problem Statement

The current Phase 3 roadmap includes "Support for multiple protocols (http, ftp, bittorrent, ed2k, etc.)" as an in-progress item. However, this appears to conflict with the project's core design principles:

### Core Design Principles (from CLAUDE.md)
1. **Fully Decentralized P2P**: No centralized servers - all peer discovery via DHT
2. **BitTorrent-Style Sharing**: Files immediately start seeding when added
3. **Privacy-First**: Circuit Relay v2, AutoNAT v2, SOCKS5 proxy support
4. **Legitimate Use Only**: Designed for personal, educational, and organizational file sharing

### The Conflict

Adding general-purpose protocol support (HTTP, FTP, ed2k) would:
- Transform the application from a P2P sharing platform into a general download manager
- Introduce centralized dependencies (HTTP/FTP servers)
- Potentially enable piracy by making it easier to download from file-sharing sites
- Dilute the project's focus on decentralized, peer-to-peer file sharing
- Require maintaining compatibility with multiple external protocols

## Analysis: Why NOT to Implement General Protocol Support

### 1. Mission Creep
Chiral Network is designed to be a **decentralized P2P file sharing platform**, not a general-purpose download manager like wget, aria2, or jDownloader.

### 2. Legal & Piracy Concerns
Supporting protocols commonly used for piracy (ed2k, generic HTTP downloads) goes against the project's "Legitimate Use Only" principle and anti-piracy design decisions already made:
- ❌ No global file search/discovery (prevents finding copyrighted content)
- ❌ No marketplace (prevents commercial piracy)
- ❌ Hash-based discovery only (requires out-of-band sharing)

### 3. Architectural Inconsistency
The project is built around:
- Kademlia DHT for peer discovery
- Bitswap protocol for chunk exchange
- libp2p for P2P networking
- Merkle trees for file integrity

Adding HTTP/FTP/ed2k would require entirely different code paths and maintenance burden.

### 4. Existing Solutions
If users need multi-protocol downloads, excellent tools already exist:
- aria2 (CLI multi-protocol downloader)
- jDownloader (GUI download manager)
- qBittorrent (BitTorrent + HTTP downloads)

## Alternative Proposal: Enhanced P2P Protocol Support

Instead of general protocol support, I propose enhancing the **existing P2P protocols** that align with the project's mission:

### Option A: Enhanced BitTorrent Compatibility (RECOMMENDED)

**Purpose:** Allow Chiral Network to interoperate with the existing BitTorrent ecosystem while maintaining decentralized architecture.

**Benefits:**
- Aligns with "BitTorrent-like" design philosophy
- Enables cross-network file sharing
- Leverages existing BitTorrent infrastructure
- Maintains decentralization
- No piracy concerns (users are already responsible for content)

**Implementation Plan:**

#### 1. BitTorrent Protocol Bridge
```rust
// Add BitTorrent wire protocol support to libp2p
pub struct BitTorrentProtocol {
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
}

impl BitTorrentProtocol {
    // Translate Bitswap blocks to BitTorrent pieces
    fn translate_to_pieces(&self, blocks: Vec<Block>) -> Vec<Piece>;
    
    // Handle BitTorrent handshake
    fn handshake(&mut self, peer: PeerId) -> Result<(), Error>;
    
    // Support BitTorrent chunk protocol
    fn handle_request(&mut self, piece_index: u32) -> Result<Vec<u8>, Error>;
}
```

#### 2. Magnet Link Support
- Parse magnet links: `magnet:?xt=urn:btih:...`
- Resolve to DHT lookups
- Download via Chiral Network OR BitTorrent network
- Seed back to both networks

#### 3. .torrent File Import
- Parse .torrent files
- Extract info_hash and metadata
- Convert to Chiral's FileMetadata format
- Enable downloading from BitTorrent peers

#### 4. DHT Bootstrap Compatibility
- Support BitTorrent DHT (BEP 5)
- Enable finding BitTorrent seeders
- Allow BitTorrent clients to find Chiral nodes

### Option B: IPFS Protocol Bridge

**Purpose:** Interoperate with IPFS for content-addressed file sharing.

**Benefits:**
- Content-addressable storage (similar to Chiral's hash-based approach)
- Large existing ecosystem
- Similar decentralized architecture
- No piracy concerns

**Implementation:**
- Support IPFS CIDs
- Translate between Bitswap protocols
- Cross-network seeding

### Option C: Keep Status Quo

**Purpose:** Focus on perfecting the existing Bitswap/DHT implementation.

**Benefits:**
- No scope creep
- Maintain focus on core features
- Better user experience for existing features
- Faster path to stability

## Recommendation

I recommend **Option A: Enhanced BitTorrent Compatibility** for the following reasons:

1. **Aligns with Core Mission:** BitTorrent is already the inspiration for Chiral's design
2. **Massive Network Effect:** Tap into existing BitTorrent ecosystem
3. **User Benefit:** Users can download from both networks simultaneously
4. **Legitimate Use:** BitTorrent is used for legal content distribution (Linux ISOs, open source software, etc.)
5. **Technical Feasibility:** Both use similar DHT/chunk-based architectures

### What NOT to Implement

- ❌ HTTP/HTTPS downloading (centralized, enables piracy sites)
- ❌ FTP support (outdated, centralized)
- ❌ ed2k/eMule protocol (associated with piracy)
- ❌ Direct Connect++ (associated with piracy)
- ❌ Generic URL downloading

## Implementation Timeline

### Phase 3A: BitTorrent Protocol Foundation (4 weeks)
- [ ] Week 1: Research BitTorrent protocol specifications (BEP 3, BEP 5)
- [ ] Week 2: Implement BitTorrent wire protocol in Rust
- [ ] Week 3: Add .torrent file parser
- [ ] Week 4: Testing and integration

### Phase 3B: Magnet Link Support (2 weeks)
- [ ] Week 1: Magnet link parser
- [ ] Week 2: DHT integration for magnet resolution

### Phase 3C: Cross-Network Seeding (3 weeks)
- [ ] Week 1: Bidirectional protocol translation
- [ ] Week 2: GUI for managing BitTorrent compatibility
- [ ] Week 3: Testing with real BitTorrent clients

### Phase 3D: Documentation (1 week)
- [ ] Update user guide
- [ ] Add BitTorrent interop documentation
- [ ] Create tutorials

**Total:** ~10 weeks

## Technical Specifications

### BitTorrent Wire Protocol Integration

```typescript
interface BitTorrentCompat {
  // Parse .torrent file
  parseTorrent(file: File): Promise<TorrentMetadata>;
  
  // Parse magnet link
  parseMagnet(uri: string): MagnetMetadata;
  
  // Convert to Chiral format
  toChiralMetadata(torrent: TorrentMetadata): FileMetadata;
  
  // Enable cross-network download
  downloadFromBitTorrent(infoHash: string): Promise<void>;
  
  // Seed to BitTorrent network
  seedToBitTorrent(fileHash: string): Promise<void>;
}
```

### Rust Backend

```rust
// src-tauri/src/bittorrent.rs
pub struct BitTorrentBridge {
    dht: Arc<DhtService>,
    bitswap: Arc<BitswapService>,
}

impl BitTorrentBridge {
    pub async fn download_torrent(&self, info_hash: [u8; 20]) -> Result<Vec<u8>, Error> {
        // 1. Query BitTorrent DHT for peers
        let peers = self.query_bittorrent_dht(info_hash).await?;
        
        // 2. Connect to BitTorrent peers
        for peer in peers {
            self.connect_bittorrent_peer(peer).await?;
        }
        
        // 3. Download pieces using BitTorrent protocol
        let pieces = self.download_pieces(info_hash).await?;
        
        // 4. Store in Chiral's Bitswap
        self.store_in_bitswap(pieces).await?;
        
        Ok(pieces)
    }
}
```

## Security Considerations

1. **Magnet Link Validation:** Verify info_hash format before processing
2. **Peer Verification:** Apply reputation system to BitTorrent peers
3. **Content Validation:** Use merkle trees for integrity checking
4. **Rate Limiting:** Prevent DHT flooding attacks
5. **Privacy:** Route BitTorrent traffic through proxy when in anonymous mode

## Testing Strategy

### Unit Tests
- Magnet link parsing
- .torrent file parsing
- Protocol translation

### Integration Tests
- Download from real BitTorrent swarm
- Seed to BitTorrent clients (qBittorrent, Transmission)
- Cross-network availability

### Manual Testing
1. Download Ubuntu ISO via magnet link
2. Verify file integrity
3. Seed back to BitTorrent network
4. Monitor reputation system

## Success Metrics

- [ ] Successfully download files from BitTorrent network
- [ ] Successfully seed files to BitTorrent clients
- [ ] < 10% performance overhead vs. native BitTorrent
- [ ] No security vulnerabilities introduced
- [ ] Positive user feedback

## Alternative Approaches Considered

### 1. IPFS-only Bridge
- **Pros:** Clean protocol, good documentation
- **Cons:** Smaller network effect than BitTorrent

### 2. Both BitTorrent + IPFS
- **Pros:** Maximum interoperability
- **Cons:** Too much complexity for Phase 3

### 3. Generic Protocol Framework
- **Pros:** Extensible architecture
- **Cons:** Scope creep, mission conflict

## Open Questions

1. Should we support BitTorrent v1 or v2 (BEP 52)?
   - **Answer:** Start with v1 for maximum compatibility, add v2 later

2. Should we support DHT-only mode (no tracker)?
   - **Answer:** Yes, aligns with decentralized philosophy

3. How to handle BitTorrent clients that don't understand Chiral nodes?
   - **Answer:** Implement standard BitTorrent protocol, appear as normal peer

4. Should private torrents be supported?
   - **Answer:** No, focus on public DHT-based torrents only

## Documentation Requirements

### User-Facing
- [ ] How to import .torrent files
- [ ] How to use magnet links
- [ ] Cross-network seeding guide
- [ ] BitTorrent compatibility FAQ

### Developer-Facing
- [ ] BitTorrent protocol implementation details
- [ ] Protocol translation architecture
- [ ] Testing with BitTorrent clients
- [ ] Debugging guide

## Backward Compatibility

This feature is **fully backward compatible**:
- Existing Chiral files work unchanged
- BitTorrent support is optional
- No breaking changes to existing APIs
- Users can opt-in to BitTorrent features

## Conclusion

The roadmap's "Support for multiple protocols" item should be interpreted as **enhanced P2P protocol interoperability**, not generic download manager functionality. 

I recommend implementing **BitTorrent protocol compatibility** to:
1. Stay true to the project's decentralized P2P mission
2. Enable massive network effects
3. Provide legitimate value to users
4. Avoid piracy-enabling features
5. Maintain architectural consistency

This approach provides the benefits of multi-protocol support while staying aligned with Chiral Network's core principles and anti-piracy stance.

## Next Steps

1. **Get stakeholder approval** on this proposal
2. **Create detailed technical specification** for BitTorrent bridge
3. **Begin implementation** if approved
4. **Update roadmap** to reflect new understanding of "multi-protocol support"

## References

- [BEP 3: The BitTorrent Protocol Specification](http://www.bittorrent.org/beps/bep_0003.html)
- [BEP 5: DHT Protocol](http://www.bittorrent.org/beps/bep_0005.html)
- [BEP 9: Extension for Peers to Send Metadata Files](http://www.bittorrent.org/beps/bep_0009.html)
- [BEP 52: The BitTorrent Protocol Specification v2](http://www.bittorrent.org/beps/bep_0052.html)
- [CLAUDE.md - Project Guidelines](../CLAUDE.md)
- [Roadmap - Phase 3](../docs/roadmap.md#phase-3-core-file-sharing-features--in-progress)
