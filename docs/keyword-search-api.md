# Keyword Search API

## Overview

The keyword search API allows users to search for files by keywords extracted from filenames. When files are published, keywords are automatically indexed in the DHT. Users can then search for files using these keywords.

## Problem Solved

**Issue:** User reported they couldn't find "README.md" uploaded by a friend when searching in the downloads tab.

**Root Cause:** The initial implementation created keyword indices (`idx:keyword` records) when files were published, but there was no API to query these indices. The existing `SearchFile` command only searched by exact file hash, not by keywords.

**Solution:** Added a complete keyword search API that queries the DHT keyword indices and returns matching file hashes.

## API Usage

### Backend Method

```rust
pub async fn search_by_keyword(&self, keyword: String, timeout_ms: u64) -> Result<Vec<String>, String>
```

**Parameters:**
- `keyword`: The keyword to search for (e.g., "readme", "document", "2024")
- `timeout_ms`: Maximum time to wait for DHT response (recommended: 5000ms)

**Returns:**
- `Ok(Vec<String>)`: List of merkle roots (file hashes) that match the keyword
- `Err(String)`: Error message if search fails or times out

### Tauri Command

```typescript
search_files_by_keyword(keyword: string, timeout_ms?: number): Promise<string[]>
```

**Example:**
```javascript
import { invoke } from "@tauri-apps/api/core";

// Search for files with keyword "readme"
const fileHashes = await invoke('search_files_by_keyword', { 
  keyword: 'readme',
  timeout_ms: 5000  // Optional, defaults to 5000ms
});

// Result: ["merkle_root_1", "merkle_root_2", ...]
console.log(`Found ${fileHashes.length} files matching 'readme'`);
```

## How It Works

### 1. Keyword Indexing (on file publish)

When a file is published:
1. Keywords are extracted from the filename (e.g., "Research-Paper-2024.pdf" → ["research", "paper", "2024"])
2. For each keyword, a DHT index record is created/updated:
   - Key: `idx:{keyword}` (e.g., `idx:research`)
   - Value: JSON array of merkle roots: `["hash1", "hash2", "hash3"]`
3. The file's merkle root is added to the index (with deduplication)

### 2. Keyword Search (user query)

When a user searches for a keyword:
1. Frontend calls `search_files_by_keyword('readme', 5000)`
2. Backend queries the DHT for record `idx:readme`
3. If found, deserializes the JSON array of merkle roots
4. Returns the list of file hashes to the frontend
5. Frontend can then fetch metadata for each hash to display results

## Integration Guide

### Frontend Implementation

To integrate keyword search in your UI:

```javascript
async function searchByKeyword(keyword) {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    
    // Search for files
    const fileHashes = await invoke('search_files_by_keyword', {
      keyword: keyword.toLowerCase(),
      timeout_ms: 5000
    });
    
    if (fileHashes.length === 0) {
      console.log(`No files found for keyword: ${keyword}`);
      return [];
    }
    
    // Fetch metadata for each file hash
    const fileMetadataList = [];
    for (const hash of fileHashes) {
      try {
        // Use your existing method to fetch file metadata
        const metadata = await getFileMetadata(hash);
        fileMetadataList.push(metadata);
      } catch (err) {
        console.warn(`Failed to fetch metadata for ${hash}:`, err);
      }
    }
    
    return fileMetadataList;
    
  } catch (error) {
    console.error('Keyword search failed:', error);
    throw error;
  }
}
```

### Example: Downloads Tab Integration

Update the search handler in `DownloadSearchSection.svelte`:

```svelte
<script>
async function searchForFile() {
  if (searchMode === 'name') {
    // Use keyword search instead of local cache
    try {
      const keyword = searchHash.trim().toLowerCase();
      const fileHashes = await invoke('search_files_by_keyword', {
        keyword,
        timeout_ms: 5000
      });
      
      if (fileHashes.length === 0) {
        pushMessage(`No files found for keyword: ${keyword}`, 'info');
        return;
      }
      
      // Fetch metadata for each file
      const results = [];
      for (const hash of fileHashes) {
        // Fetch file metadata and add to results
        // ...
      }
      
      versionResults = results;
      latestStatus = 'found';
      pushMessage(`Found ${results.length} file(s) for keyword: ${keyword}`, 'success');
      
    } catch (error) {
      console.error('Keyword search failed:', error);
      pushMessage(`Search failed: ${error}`, 'error');
    }
  }
}
</script>
```

## Performance Considerations

- **Latency:** 1-5 seconds typical (depends on DHT network conditions)
- **Timeout:** Recommended 5 seconds for balance between UX and success rate
- **Multiple Keywords:** Search one keyword at a time, or search multiple and combine results
- **Caching:** Consider caching search results temporarily to avoid repeated DHT queries

## Limitations

1. **Exact Match:** Keywords must match exactly (no fuzzy search or wildcards yet)
2. **Single Word:** Search one keyword at a time (can't search "research paper" as phrase)
3. **Case Insensitive:** Keywords are converted to lowercase automatically
4. **Network Dependent:** Requires DHT network connectivity
5. **No Ranking:** Results are unordered (all files with keyword are returned)

## Future Enhancements

1. **Multi-keyword Search:** Support searching multiple keywords with AND/OR logic
2. **Fuzzy Matching:** Support partial keyword matches and typos
3. **Result Ranking:** Rank results by relevance, popularity, or recency
4. **Autocomplete:** Suggest keywords based on what's indexed
5. **Search History:** Remember and suggest previous searches
6. **Batch Metadata Fetch:** Fetch all file metadata in parallel for faster results display

## Troubleshooting

### No results found
- Ensure the file was published with keywords (check logs for "Extracted N keywords")
- Keywords must be >2 characters (short words are filtered out)
- Allow time for DHT propagation (30-60 seconds after publishing)
- Check DHT connectivity with `get_peer_count()`

### Search timeout
- Increase timeout_ms (try 10000ms)
- Check network connectivity
- Verify DHT service is running
- Check if enough DHT nodes are connected

### Wrong results
- Verify keyword extraction logic in `extract_keywords()`
- Check that indices are being created correctly (look for "Updated keyword index" in logs)
- Ensure file was published after keyword indexing was implemented

## Related Documentation

- [Keyword Indexing Implementation](./keyword-indexing.md)
- [DHT Service API](../src-tauri/src/dht.rs)
- Implementation commits:
  - Initial keyword indexing: b6a6efd
  - Keyword search API: a8fed6f
