# DHT Keyword Indexing

## Overview

The DHT keyword indexing feature enables efficient file discovery by indexing files based on keywords extracted from their filenames. This allows users to search for files by keyword rather than requiring exact hash matches.

## How It Works

### 1. Keyword Extraction

When a file is published, keywords are automatically extracted from the filename:

- Splits filename on non-alphanumeric characters
- Filters out short words (≤ 2 characters)
- Removes file extensions
- Deduplicates keywords
- Converts to lowercase

**Example:**
```
Filename: "Important-Document-2024.pdf"
Keywords: ["important", "document", "2024"]
```

### 2. Index Structure

Each keyword maintains its own index in the DHT:

- **Index Key**: `idx:{keyword}` (e.g., `idx:document`)
- **Index Value**: JSON array of merkle roots: `["hash1", "hash2", "hash3"]`
- **Storage**: Kademlia DHT records with `Quorum::One`

### 3. Read-Modify-Write Flow

For each keyword:

1. **Read**: Query DHT for existing index (`get_record`)
2. **Modify**: 
   - If found: Deserialize, add new merkle_root (with deduplication)
   - If not found: Create new list with single merkle_root
3. **Write**: Serialize updated list and store (`put_record`)

### 4. Size Limits

- Maximum record size: 2048 bytes
- Exceeding this limit will log an error and skip the update
- Each merkle root is ~64 characters, allowing ~25-30 files per keyword

## Implementation Details

### Data Structures

```rust
struct PendingIndexUpdate {
    keyword: String,
    index_key: kad::RecordKey,
    new_merkle_root: String,
    timestamp: std::time::Instant,
}
```

### Timeout Handling

- Queries timeout after 30 seconds
- Periodic cleanup removes stale queries
- Timeout generates a warning event

### Error Handling

- Serialization errors: Log and emit `DhtEvent::Error`
- Size limit exceeded: Log error and skip update
- Put failures: Log error and emit `DhtEvent::Error`

## Usage Example

```rust
// When publishing a file, keywords are automatically indexed
let metadata = FileMetadata {
    file_name: "research-paper-2024.pdf".to_string(),
    merkle_root: "abc123...".to_string(),
    // ... other fields
};

// This will automatically:
// 1. Extract keywords: ["research", "paper", "2024"]
// 2. Update indices: idx:research, idx:paper, idx:2024
// 3. Each index will contain the merkle_root "abc123..."
dht_service.publish_file(metadata).await?;
```

## Future Enhancements

1. **Search API**: Add a command to query keyword indices and retrieve matching files
2. **Index Splitting**: Split large indices into buckets (`idx:keyword:1`, `idx:keyword:2`)
3. **CRDT Merge**: Implement proper conflict resolution for concurrent updates
4. **Popularity Ranking**: Track download counts and rank results by popularity
5. **Fuzzy Matching**: Support partial keyword matches and typo tolerance

## Testing

The implementation includes comprehensive tests:

- `test_extract_keywords`: Keyword extraction and filtering
- `test_keyword_index_serialization`: JSON serialization format
- `test_keyword_index_deduplication`: Merkle root deduplication
- `test_keyword_index_key_format`: Index key format validation
- `test_pending_index_update_creation`: Data structure creation
- `test_size_limit_check`: Size limit enforcement

Run tests with:
```bash
cargo test --lib dht::tests
```

## Performance Considerations

- **Write Amplification**: Each file publish generates N DHT writes (one per keyword)
- **Network Traffic**: Each keyword requires a DHT read + write roundtrip
- **Concurrency**: Multiple publishers may update the same index simultaneously (last-writer-wins)
- **Query Overhead**: 30-second timeout per keyword limits throughput

## Limitations

1. No atomic read-modify-write across DHT nodes (eventual consistency)
2. Fixed size limit may require index splitting for popular keywords
3. No query deduplication (multiple concurrent publishes of same file)
4. Timeout handling is best-effort (cleanup every 30 minutes)
