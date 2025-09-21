# Manual Testing Guide for Chiral Network File Chunking & Storage

This guide provides step-by-step instructions for manually testing the file chunking, upload flow, and storage-node server implementation.

## Prerequisites

1. **Install Dependencies**:
   ```bash
   # Install Node.js dependencies
   npm install
   
   # Install Tauri system dependencies (Ubuntu/Debian)
   sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
   
   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Build Frontend**:
   ```bash
   npm run build
   ```

## Test 1: Storage Node Server

### Start the Storage Node
```bash
cd storage
cargo run --bin storage-node -- --port 8080 --storage-path ./test-storage
```

You should see:
```
🚀 Chiral Storage Node starting on port 8080
📁 Storage path: ./test-storage
🌐 Server running at http://0.0.0.0:8080
```

### Test API Endpoints

1. **Health Check**:
   ```bash
   curl http://localhost:8080/health
   ```
   Expected response:
   ```json
   {
     "status": "healthy",
     "version": "0.1.0",
     "uptime_seconds": 5,
     "chunks_stored": 0
   }
   ```

2. **Store a Test Chunk**:
   ```bash
   # Create test data
   echo "Hello, Chiral Network!" > test.txt
   
   # Upload chunk
   curl -X POST \
     -H "Content-Type: application/octet-stream" \
     -H "x-chunk-hash: test123" \
     --data-binary @test.txt \
     http://localhost:8080/chunks
   ```
   Expected response:
   ```json
   {
     "chunk_hash": "actual_sha256_hash",
     "size": 24,
     "stored_at": "timestamp"
   }
   ```

3. **List Stored Chunks**:
   ```bash
   curl http://localhost:8080/chunks
   ```

4. **Retrieve Chunk**:
   ```bash
   curl http://localhost:8080/chunks/{chunk_hash} -o retrieved.txt
   diff test.txt retrieved.txt  # Should show no differences
   ```

## Test 2: Tauri Application Build

### Build and Run (Development Mode)
```bash
# This will fail in headless environments but should compile successfully
npm run tauri dev
```

**Expected Results**:
- ✅ Frontend builds without errors
- ✅ Rust backend compiles successfully 
- ✅ May panic with GTK error in headless environment (this is normal)
- ✅ No compilation errors in console output

### Build Production Binary
```bash
npm run tauri build
```

**Expected Results**:
- ✅ Creates optimized production build
- ✅ Generates platform-specific installer/executable
- ✅ Binary available in `src-tauri/target/release/`

## Test 3: File Chunking Integration

### Test Chunk Manager (Rust Unit Tests)
```bash
cd storage
cargo test chunks::tests::test_chunk_file_small
cargo test chunks::tests::test_encrypt_decrypt_chunk
cargo test chunks::tests::test_hash_chunk
```

**Expected Results**:
- ✅ All basic chunking tests pass
- ✅ Encryption/decryption roundtrip works
- ✅ File hashing generates consistent SHA-256 hashes

### Test Storage API Integration
```bash
cd storage
cargo test api::tests::test_store_and_retrieve_chunk
cargo test api::tests::test_list_chunks
```

**Expected Results**:
- ✅ Chunk storage and retrieval works
- ✅ API endpoints handle requests correctly
- ✅ Hash validation prevents data corruption

## Test 4: Frontend Integration

### Test Settings Storage Path
1. Open the application (if GUI available)
2. Navigate to Settings page
3. Change storage path via file picker or manual entry
4. Verify path is saved and appears in input field

### Test File Service
If running in browser mode:
```bash
npm run dev
# Open http://localhost:1420 in browser
```

**Manual Steps**:
1. Navigate to Upload page
2. Attempt to drag/drop or select files
3. Check browser console for any errors
4. Verify FileService methods are called (check Network tab)

## Test 5: End-to-End File Upload Flow

### Full Integration Test
```bash
# Terminal 1: Start storage node
cd storage
cargo run --bin storage-node -- --port 8080

# Terminal 2: Test file upload simulation
cd storage
cat > test_upload.py << 'EOF'
import requests
import hashlib

# Create test file
test_data = b"This is a test file for Chiral Network chunking!"
chunk_hash = hashlib.sha256(test_data).hexdigest()

# Upload chunk
response = requests.post(
    'http://localhost:8080/chunks',
    data=test_data,
    headers={
        'Content-Type': 'application/octet-stream',
        'x-chunk-hash': chunk_hash
    }
)

print(f"Upload response: {response.status_code}")
print(f"Response body: {response.text}")

# Retrieve chunk
retrieve_response = requests.get(f'http://localhost:8080/chunks/{chunk_hash}')
print(f"Retrieve response: {retrieve_response.status_code}")
print(f"Data matches: {retrieve_response.content == test_data}")
EOF

python3 test_upload.py
```

**Expected Results**:
- ✅ Upload response: 201
- ✅ Retrieve response: 200  
- ✅ Data matches: True

## Test 6: Storage Settings Integration

### Test Tauri Commands
If you can run the Tauri app with GUI:

1. **Test Get Storage Path**:
   - Open Developer Tools in the app
   - Run: `await window.__TAURI__.invoke('get_storage_path_setting')`
   - Should return current storage path

2. **Test Set Storage Path**:
   - Run: `await window.__TAURI__.invoke('set_storage_path_setting', { newPath: '/tmp/test-storage' })`
   - Should create directory and return success

## Troubleshooting

### Common Issues

1. **"tauri: command not found"**:
   ```bash
   npm install @tauri-apps/cli
   # or
   cargo install tauri-cli
   ```

2. **GTK initialization failed**:
   - Normal in headless environments
   - Indicates successful compilation
   - Install desktop environment for GUI testing

3. **Storage node won't start**:
   ```bash
   # Check if port is in use
   lsof -i :8080
   
   # Use different port
   cargo run --bin storage-node -- --port 8081
   ```

4. **Permission denied creating storage directory**:
   ```bash
   # Use writable directory
   cargo run --bin storage-node -- --storage-path ./storage
   ```

### Verification Checklist

- [ ] Storage node starts without errors
- [ ] All HTTP API endpoints respond correctly
- [ ] File chunking tests pass
- [ ] Tauri application compiles successfully
- [ ] Frontend builds without errors
- [ ] Settings integration works
- [ ] End-to-end upload/download cycle completes

## Performance Testing

### Load Testing Storage Node
```bash
# Install Apache Benchmark
sudo apt-get install apache2-utils

# Test concurrent uploads
ab -n 100 -c 10 -T 'application/octet-stream' -p test.txt http://localhost:8080/chunks
```

### Large File Testing
```bash
# Create large test file (10MB)
dd if=/dev/urandom of=large_test.bin bs=1M count=10

# Test chunking performance
cd storage
time cargo run --example chunk_large_file large_test.bin
```

---

## Summary

This testing guide covers all major components:
- ✅ Storage node HTTP API functionality
- ✅ File chunking and encryption pipeline  
- ✅ Tauri application compilation
- ✅ Frontend integration
- ✅ Settings storage path management
- ✅ End-to-end file upload workflow

Follow these tests to verify that the file chunking, upload flow, and storage-node server implementation is working correctly.