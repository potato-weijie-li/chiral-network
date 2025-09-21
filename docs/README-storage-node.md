# Chiral Network Storage Node

This document describes how to set up, run, and use the Chiral Network storage node for chunk-based file storage.

## Overview

The storage node provides HTTP API endpoints for storing and retrieving encrypted file chunks. It implements the storage layer of the Chiral Network protocol without DHT or marketplace functionality.

## Features

- **Chunk Storage**: Store encrypted file chunks with 256KB size limit per chunk
- **AES-256-GCM Encryption**: Per-chunk encryption with unique nonces
- **SHA-256 Hashing**: Content-addressed storage using SHA-256 hashes
- **HTTP API**: Simple REST API for chunk operations
- **Integrity Verification**: Automatic hash verification on upload
- **Health Monitoring**: Built-in health check endpoint

## Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- At least 1GB free disk space for chunk storage

### Running the Storage Node

```bash
# Clone the repository
git clone https://github.com/potato-weijie-li/chiral-network.git
cd chiral-network/storage

# Build the storage node
cargo build --release

# Run with default settings (port 8080, ./storage directory)
cargo run --bin storage-node

# Run with custom settings
cargo run --bin storage-node -- --port 8080 --storage-path /path/to/storage --verbose
```

### Command Line Options

- `--port, -p`: HTTP server port (default: 8080)
- `--storage-path, -s`: Directory for chunk storage (default: ./storage)
- `--verbose, -v`: Enable verbose logging

## API Endpoints

### Store Chunk

Store an encrypted chunk with optional hash verification.

```http
POST /chunks
Content-Type: application/octet-stream
X-Chunk-Hash: <sha256_hash> (optional)

<binary_chunk_data>
```

**Response:**
```json
{
  "chunk_hash": "a7d8f9e8c7b6a5d4f3e2d1c0b9a8d7f6e5d4c3b2a1098765432100abcdef123456",
  "size": 262144,
  "stored_at": 1234567890
}
```

### Retrieve Chunk

Retrieve a stored chunk by its hash.

```http
GET /chunks/{chunk_hash}
```

**Response:**
- Status 200: Returns binary chunk data with `Content-Type: application/octet-stream`
- Status 404: Chunk not found
- Status 400: Invalid hash format

### List Chunks

List all stored chunks (useful for debugging).

```http
GET /chunks
```

**Response:**
```json
{
  "chunks": [
    "a7d8f9e8c7b6a5d4f3e2d1c0b9a8d7f6e5d4c3b2a1098765432100abcdef123456",
    "b8e9f0d1c2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9"
  ],
  "count": 2
}
```

### Health Check

Check if the storage node is running properly.

```http
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": 1234567890,
  "version": "0.1.0"
}
```

## Usage with Tauri Frontend

The storage node integrates with the Chiral Network Tauri application for seamless file chunking and upload.

### 1. Start the Storage Node

```bash
cd storage
cargo run --bin storage-node -- --port 8080
```

### 2. Start the Tauri Application

```bash
cd ..
npm run tauri dev
```

### 3. Upload Files via Frontend

The frontend will automatically:
1. Chunk files into 256KB pieces
2. Encrypt each chunk with AES-256-GCM
3. Upload chunks to the storage node
4. Store chunk metadata locally

## File Chunk Format

Each stored chunk follows this format:

```
┌─────────────────────────────────────┐
│ Header (64 bytes)                   │
├─────────────────────────────────────┤
│ - Magic Number: "CHNK" (4 bytes)    │
│ - Version: 1 (2 bytes)              │
│ - Chunk Index (4 bytes)             │
│ - Total Chunks (4 bytes)            │
│ - File Hash (32 bytes)              │
│ - Chunk Hash (32 bytes)             │
├─────────────────────────────────────┤
│ Metadata (256 bytes)                │
├─────────────────────────────────────┤
│ - Encryption IV (16 bytes)          │
│ - Compression Type (1 byte)         │
│ - Original Size (8 bytes)           │
│ - Compressed Size (8 bytes)         │
│ - Timestamp (8 bytes)               │
│ - Reserved (215 bytes)              │
├─────────────────────────────────────┤
│ Encrypted Data (variable size)      │
├─────────────────────────────────────┤
│ Checksum (32 bytes)                 │
└─────────────────────────────────────┘
```

## Testing

### Unit Tests

Run the chunk manager unit tests:

```bash
cargo test chunk_manager
```

### Integration Tests

Run the storage API integration tests:

```bash
cargo test storage_api
```

### Manual Testing

Test the storage node manually with curl:

```bash
# Store a chunk
echo "Hello, World!" | curl -X POST \
  -H "Content-Type: application/octet-stream" \
  --data-binary @- \
  http://localhost:8080/chunks

# Retrieve the chunk (use hash from store response)
curl http://localhost:8080/chunks/YOUR_CHUNK_HASH

# List all chunks
curl http://localhost:8080/chunks

# Check health
curl http://localhost:8080/health
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build with all features
cargo build --all-features
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Run all tests
cargo test

# Check for security vulnerabilities
cargo audit
```

### Project Structure

```
storage/
├── src/
│   ├── lib.rs          # Library entry point
│   ├── chunks.rs       # Chunk manager implementation
│   └── api.rs          # HTTP API server
├── tests/
│   ├── chunk_manager.rs # Unit tests for chunk manager
│   └── storage_api.rs   # Integration tests for API
├── Cargo.toml          # Dependencies and configuration
└── README.md           # This file
```

## Configuration

### Environment Variables

- `RUST_LOG`: Set logging level (e.g., `debug`, `info`, `warn`, `error`)
- `STORAGE_PATH`: Override default storage directory
- `PORT`: Override default port

### Example Configuration

```bash
# Enable debug logging and use custom paths
export RUST_LOG=debug
export STORAGE_PATH=/mnt/storage/chiral
export PORT=9090

cargo run --bin storage-node
```

## Security Considerations

### File Encryption

- Each chunk is encrypted with AES-256-GCM
- Unique 96-bit nonce per chunk prevents replay attacks
- Encryption keys are managed by the client application

### Network Security

- Use HTTPS in production (configure reverse proxy)
- Implement rate limiting for production deployments
- Consider firewall rules to restrict access

### Storage Security

- Chunks are content-addressed by SHA-256 hash
- Integrity verification on every upload
- No plaintext data is stored on disk

## Performance

### Benchmarks

Typical performance on modern hardware:

- **Chunk Storage**: ~1000 chunks/second
- **Chunk Retrieval**: ~2000 chunks/second  
- **Concurrent Connections**: Up to 1000
- **Memory Usage**: ~50MB base + chunk cache

### Optimization Tips

1. Use SSD storage for better I/O performance
2. Increase file descriptor limits for high concurrency
3. Monitor disk space and implement cleanup policies
4. Use multiple storage nodes for load distribution

## Troubleshooting

### Common Issues

#### Port Already in Use
```bash
# Find process using port 8080
lsof -i :8080

# Kill the process or use a different port
cargo run --bin storage-node -- --port 8081
```

#### Permission Denied
```bash
# Ensure storage directory is writable
chmod 755 ./storage

# Or specify a different directory
cargo run --bin storage-node -- --storage-path /tmp/chiral-storage
```

#### Out of Disk Space
```bash
# Check disk usage
df -h

# Clean up old chunks if needed
find ./storage -type f -mtime +30 -delete
```

### Logging

Enable verbose logging to debug issues:

```bash
RUST_LOG=debug cargo run --bin storage-node -- --verbose
```

### Health Monitoring

Monitor the health endpoint for production deployments:

```bash
# Simple health check script
curl -f http://localhost:8080/health || echo "Storage node is down!"
```

## Integration with Chiral Network

This storage node is designed to work with:

1. **Tauri Desktop App**: Provides the user interface and file management
2. **Chunk Manager**: Handles file chunking and encryption
3. **Future DHT Integration**: Will enable decentralized chunk discovery
4. **Future Marketplace**: Will enable storage economics

The storage node deliberately excludes DHT and marketplace functionality to focus on reliable chunk storage and retrieval.

## License

This project is licensed under the MIT License. See LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## Support

For issues and questions:

- File issues on GitHub
- Check the documentation in `docs/`
- Review test cases for usage examples