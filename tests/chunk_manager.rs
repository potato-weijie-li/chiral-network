use std::fs;
use std::path::Path;
use tempfile::TempDir;
use chiral_storage::chunks::*;

#[test]
fn test_chunk_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    assert_eq!(manager.chunk_size, 256 * 1024);
}

#[test]
fn test_hash_chunk() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    let data = b"Hello, World!";
    let hash = manager.hash_chunk(data);
    
    // SHA-256 of "Hello, World!" should be consistent
    assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
    
    // Test consistency - same input should produce same hash
    let hash2 = manager.hash_chunk(data);
    assert_eq!(hash, hash2);
}

#[test]
fn test_encrypt_decrypt_chunk() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    let data = b"Test data for encryption";
    let key = [1u8; 32]; // Test key
    
    let (encrypted, nonce) = manager.encrypt_chunk(data, &key).unwrap();
    let decrypted = manager.decrypt_chunk(&encrypted, &key, &nonce).unwrap();
    
    assert_eq!(data.to_vec(), decrypted);
    
    // Verify encryption actually changed the data
    assert_ne!(data.to_vec(), encrypted);
}

#[test]
fn test_chunk_file_small() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Create a test file
    let test_data = b"Hello, chunking world!";
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, test_data).unwrap();
    
    let key = [2u8; 32];
    let chunks = manager.chunk_file(&test_file, &key).unwrap();
    
    assert_eq!(chunks.len(), 1); // Should be only one chunk for small file
    assert_eq!(chunks[0].index, 0);
    assert_eq!(chunks[0].size, test_data.len());
    assert_eq!(chunks[0].total_chunks, 1);
    
    // Verify the chunk was actually saved
    assert!(Path::new(&manager.storage_path).join(&chunks[0].hash).exists());
}

#[test]
fn test_chunk_file_large() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Create a test file larger than chunk size
    let chunk_size = 256 * 1024;
    let test_data = vec![42u8; chunk_size + 1000]; // Slightly larger than one chunk
    let test_file = temp_dir.path().join("large_test.bin");
    fs::write(&test_file, &test_data).unwrap();
    
    let key = [3u8; 32];
    let chunks = manager.chunk_file(&test_file, &key).unwrap();
    
    assert_eq!(chunks.len(), 2); // Should be two chunks
    assert_eq!(chunks[0].size, chunk_size);
    assert_eq!(chunks[1].size, 1000);
    assert_eq!(chunks[0].total_chunks, 2);
    assert_eq!(chunks[1].total_chunks, 2);
    
    // Verify both chunks were saved
    for chunk in &chunks {
        assert!(Path::new(&manager.storage_path).join(&chunk.hash).exists());
    }
}

#[test]
fn test_reassemble_file() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Create original test data
    let original_data = b"This is test data for chunking and reassembly! It should be long enough to span multiple chunks if needed.";
    let test_file = temp_dir.path().join("original.txt");
    fs::write(&test_file, original_data).unwrap();
    
    let key = [4u8; 32];
    
    // Chunk the file
    let chunks = manager.chunk_file(&test_file, &key).unwrap();
    
    // Reassemble to a new file
    let output_file = temp_dir.path().join("reassembled.txt");
    manager.reassemble_file(&chunks, &output_file, &key).unwrap();
    
    // Verify the reassembled file matches the original
    let reassembled_data = fs::read(&output_file).unwrap();
    assert_eq!(original_data.to_vec(), reassembled_data);
}

#[test]
fn test_chunk_validation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Create a test file
    let test_data = b"Test chunk validation";
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, test_data).unwrap();
    
    let key = [5u8; 32];
    let chunks = manager.chunk_file(&test_file, &key).unwrap();
    
    // Load the chunk and validate it
    let chunk_data = manager.load_chunk(&chunks[0].hash).unwrap();
    assert!(manager.validate_chunk(&chunk_data).unwrap());
    
    // Corrupt the chunk and verify validation fails
    let mut corrupted_chunk = chunk_data.clone();
    corrupted_chunk[100] = corrupted_chunk[100].wrapping_add(1); // Flip a bit
    assert!(!manager.validate_chunk(&corrupted_chunk).unwrap());
}

#[test]
fn test_extract_header_and_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Create a test file
    let test_data = b"Test header and metadata extraction";
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, test_data).unwrap();
    
    let key = [6u8; 32];
    let chunks = manager.chunk_file(&test_file, &key).unwrap();
    
    // Load the chunk
    let chunk_data = manager.load_chunk(&chunks[0].hash).unwrap();
    
    // Extract and verify header
    let header = manager.extract_header(&chunk_data).unwrap();
    assert_eq!(header.magic, [0x43, 0x48, 0x4E, 0x4B]); // "CHNK"
    assert_eq!(header.version, 1);
    assert_eq!(header.chunk_index, 0);
    assert_eq!(header.total_chunks, 1);
    
    // Extract and verify metadata
    let metadata = manager.extract_metadata(&chunk_data).unwrap();
    assert_eq!(metadata.compression_type, 0);
    assert_eq!(metadata.original_size as usize, test_data.len());
    assert!(metadata.timestamp > 0);
    
    // Extract encrypted data
    let encrypted_data = manager.extract_encrypted_data(&chunk_data).unwrap();
    assert!(!encrypted_data.is_empty());
    
    // Decrypt and verify
    let decrypted = manager.decrypt_chunk(&encrypted_data, &key, &metadata.iv).unwrap();
    assert_eq!(decrypted, test_data);
}

#[test]
fn test_file_hash_consistency() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Create a test file
    let test_data = b"Test file hash consistency";
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, test_data).unwrap();
    
    // Calculate hash multiple times
    let hash1 = manager.hash_file(&test_file).unwrap();
    let hash2 = manager.hash_file(&test_file).unwrap();
    
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA-256 hex string length
}

#[test]
fn test_streaming_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Create a large test file (1MB)
    let large_data = vec![0xAAu8; 1024 * 1024];
    let test_file = temp_dir.path().join("large.bin");
    fs::write(&test_file, &large_data).unwrap();
    
    let key = [7u8; 32];
    
    // This should not cause memory issues due to streaming
    let chunks = manager.chunk_file(&test_file, &key).unwrap();
    
    // Should be 4 chunks (1MB / 256KB = 4)
    assert_eq!(chunks.len(), 4);
    
    // Verify each chunk
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, i as u32);
        assert_eq!(chunk.total_chunks, 4);
        assert_eq!(chunk.size, 256 * 1024); // All chunks should be full size
        
        // Verify chunk exists
        assert!(Path::new(&manager.storage_path).join(&chunk.hash).exists());
    }
    
    // Reassemble and verify
    let output_file = temp_dir.path().join("reassembled_large.bin");
    manager.reassemble_file(&chunks, &output_file, &key).unwrap();
    
    let reassembled_data = fs::read(&output_file).unwrap();
    assert_eq!(reassembled_data.len(), large_data.len());
    assert_eq!(reassembled_data, large_data);
}

#[test]
fn test_chunk_file_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ChunkManager::new(temp_dir.path().to_path_buf());
    
    // Test empty file
    let empty_file = temp_dir.path().join("empty.txt");
    fs::write(&empty_file, b"").unwrap();
    
    let key = [8u8; 32];
    let chunks = manager.chunk_file(&empty_file, &key).unwrap();
    
    // Empty file should produce no chunks
    assert_eq!(chunks.len(), 0);
    
    // Test single byte file
    let single_byte_file = temp_dir.path().join("single.txt");
    fs::write(&single_byte_file, b"x").unwrap();
    
    let chunks = manager.chunk_file(&single_byte_file, &key).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].size, 1);
    
    // Reassemble and verify
    let output_file = temp_dir.path().join("reassembled_single.txt");
    manager.reassemble_file(&chunks, &output_file, &key).unwrap();
    
    let reassembled_data = fs::read(&output_file).unwrap();
    assert_eq!(reassembled_data, b"x");
}