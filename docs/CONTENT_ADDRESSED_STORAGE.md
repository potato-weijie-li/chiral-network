# Content-Addressed Storage Implementation

This document describes the content-addressed chunk storage system implemented for Chiral Network, which provides deduplication, integrity verification, streaming I/O, and resume capabilities.

## Overview

The content-addressed storage system splits files into fixed-size chunks (1MB default), stores each chunk under its SHA-256 hash, and creates a manifest file containing metadata and the ordered list of chunk hashes. This approach enables:

- **Deduplication**: Identical chunks are stored only once
- **Integrity Verification**: Each chunk and the complete file are verified using SHA-256
- **Streaming Operations**: Files are processed chunk-by-chunk to avoid memory issues
- **Resume/Parallel Downloads**: Missing chunks can be downloaded independently
- **Atomic Operations**: File locking and atomic writes prevent corruption

## Storage Layout

```
storage_root/
├── chunks/
│   ├── ab/
│   │   ├── abc123def456...   # Chunk file (name = SHA-256 hash)
│   │   └── abf789abc123...
│   ├── cd/
│   │   └── cde456789abc...
│   └── ...
└── manifests/
    ├── file_hash_1.json      # File manifest
    ├── file_hash_2.json
    └── ...
```

### Chunk Storage
- Chunks are stored in subdirectories based on the first 2 characters of their hash
- This prevents having too many files in a single directory
- Chunk files contain either raw data or encrypted data (with prepended nonce)

### Manifest Storage
- Manifests are stored as JSON files named by the file's overall hash
- Each manifest contains complete metadata about the file and its chunks

## File Manifest Structure

```json
{
  "version": "1.0",
  "file_hash": "sha256_of_complete_file",
  "file_name": "example.pdf",
  "file_size": 10485760,
  "mime_type": "application/pdf",
  "chunk_size": 1048576,
  "total_chunks": 10,
  "chunks": [
    {
      "index": 0,
      "hash": "sha256_of_chunk_0",
      "size": 1048576,
      "encrypted_size": 1048592,
      "offset": 0
    }
  ],
  "encryption": {
    "algorithm": "AES-256-GCM",
    "encrypted_key_bundle": {
      "ephemeral_public_key": "hex_encoded_public_key",
      "encrypted_key": "hex_encoded_aes_key",
      "nonce": "hex_encoded_nonce"
    }
  },
  "timestamps": {
    "created": 1234567890,
    "modified": 1234567890,
    "accessed": 1234567890
  },
  "manifest_hash": "sha256_of_manifest_excluding_this_field"
}
```

## API Usage

### Backend (Rust)

```rust
use manager::{ChunkManager, FileManifest};

// Initialize chunk manager
let storage_path = PathBuf::from("/path/to/storage");
let manager = ChunkManager::new(storage_path);

// Store a file
let manifest = manager.store_file_with_manifest(
    Path::new("/path/to/input.txt"),
    None // No encryption
)?;

// Reconstruct file
manager.reconstruct_file(
    &manifest,
    Path::new("/path/to/output.txt"),
    None // No decryption key needed
)?;

// Check missing chunks
let missing = manager.verify_chunks_available(&manifest)?;
```

### Frontend (TypeScript)

```typescript
import ContentAddressedStorage from './lib/contentAddressedStorage';

// Initialize storage
await ContentAddressedStorage.initChunkManager('/path/to/storage');

// Store a file
const manifest = await ContentAddressedStorage.storeFile('/path/to/file.pdf');

// Check download progress
const missingChunks = await ContentAddressedStorage.verifyChunksAvailable(manifest.file_hash);
const progress = ContentAddressedStorage.calculateProgress(
  manifest.total_chunks - missingChunks.length,
  manifest.total_chunks
);

// Reconstruct file when complete
if (missingChunks.length === 0) {
  await ContentAddressedStorage.reconstructFile(manifest.file_hash, '/output/file.pdf');
}

// Get storage statistics
const stats = await ContentAddressedStorage.getStorageStats();
console.log(`Storage: ${ContentAddressedStorage.formatBytes(stats.total_storage_bytes)}`);
```

## Configuration

### Chunk Size
The default chunk size is 1MB (1,048,576 bytes), which provides a good balance between:
- Deduplication efficiency (smaller chunks = more deduplication opportunities)
- Storage overhead (larger chunks = fewer manifest entries)
- Network efficiency (reasonable chunk size for P2P transfers)

```typescript
// Set custom chunk size (must be done before storing files)
await ContentAddressedStorage.setChunkSize(2.0); // 2MB chunks
```

### Storage Path
The storage path should be on a filesystem that supports:
- File locking (for atomic operations)
- Sufficient space for chunks and manifests
- Fast random access (SSD recommended for many small files)

## Security Features

### Integrity Verification
1. **Chunk-level**: Each chunk is verified against its SHA-256 hash
2. **File-level**: Reconstructed file is verified against the overall file hash
3. **Manifest-level**: Manifest integrity is verified using its own hash

### Encryption Support
- Files can be encrypted using AES-256-GCM
- Each chunk gets a unique nonce for security
- AES key is encrypted using X25519 ECIES for sharing

### Atomic Operations
- Chunks are written to temporary files then atomically renamed
- File locking prevents concurrent access during writes
- Manifests are saved atomically to prevent corruption

## Performance Characteristics

### Memory Usage
- **Streaming I/O**: Only one chunk (1MB default) loaded in memory at a time
- **Constant Memory**: Memory usage doesn't grow with file size
- **Efficient**: Suitable for processing very large files

### Disk Usage
- **Deduplication**: Identical chunks stored only once across all files
- **Overhead**: Small overhead for manifest files (~1KB per file)
- **Scalability**: Subdirectory structure prevents filesystem bottlenecks

### Network Efficiency
- **Parallel Downloads**: Multiple chunks can be downloaded simultaneously
- **Resume Support**: Interrupted downloads can be resumed
- **Selective Sync**: Only missing chunks need to be transferred

## Error Handling

The system provides comprehensive error handling for:
- **Storage Errors**: Disk full, permission denied, etc.
- **Integrity Failures**: Corrupted chunks or manifests
- **Missing Data**: Chunks or manifests not found
- **Encryption Errors**: Invalid keys or corrupted encrypted data

## Testing

Run the test suite to verify implementation:

```bash
node tests/contentAddressedStorage.test.mjs
```

Tests cover:
- Manifest structure validation
- Content-addressed path generation
- Chunk deduplication logic
- Integrity hash calculation

## Future Enhancements

Potential improvements for the storage system:
- **Compression**: Optional chunk compression using zstd
- **Erasure Coding**: Redundancy for data recovery
- **Garbage Collection**: Automatic cleanup of unreferenced chunks
- **Metrics**: Detailed performance and usage statistics
- **Multi-tier Storage**: Hot/cold storage based on access patterns

## Troubleshooting

### Common Issues

1. **Permission Denied**
   - Ensure storage directory is writable
   - Check file system permissions

2. **Disk Full**
   - Monitor available space
   - Implement storage quotas if needed

3. **Integrity Failures**
   - Check for filesystem corruption
   - Verify chunk files haven't been modified externally

4. **Performance Issues**
   - Use SSD storage for better random access performance
   - Adjust chunk size based on use case
   - Monitor memory usage during large file operations

### Debug Information

Enable debug logging to get detailed information about storage operations:
```rust
// In debug builds, storage operations are logged
log::debug!("Storing chunk {} at {}", chunk_hash, chunk_path);
```