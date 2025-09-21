use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;
use chiral_storage::api::*;

#[tokio::test]
async fn test_storage_api_store_and_retrieve() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    // Start storage server on a random port
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 0)).await;
    });
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Test data
    let test_data = b"Hello, storage API!";
    let expected_hash = calculate_chunk_hash(test_data);
    
    // Store chunk
    let client = reqwest::Client::new();
    let store_response = client
        .post("http://127.0.0.1:8080/chunks")
        .header("content-type", "application/octet-stream")
        .header("x-chunk-hash", &expected_hash)
        .body(test_data.to_vec())
        .send()
        .await
        .unwrap();
    
    assert_eq!(store_response.status(), 201);
    
    let store_result: ChunkUploadResponse = store_response.json().await.unwrap();
    assert_eq!(store_result.chunk_hash, expected_hash);
    assert_eq!(store_result.size, test_data.len());
    
    // Retrieve chunk
    let retrieve_response = client
        .get(&format!("http://127.0.0.1:8080/chunks/{}", expected_hash))
        .send()
        .await
        .unwrap();
    
    assert_eq!(retrieve_response.status(), 200);
    assert_eq!(retrieve_response.headers().get("content-type").unwrap(), "application/octet-stream");
    
    let retrieved_data = retrieve_response.bytes().await.unwrap();
    assert_eq!(retrieved_data.as_ref(), test_data);
    
    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_storage_api_hash_validation() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8081)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    let test_data = b"Test hash validation";
    let wrong_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    
    // Try to store with wrong hash
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8081/chunks")
        .header("content-type", "application/octet-stream")
        .header("x-chunk-hash", wrong_hash)
        .body(test_data.to_vec())
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 400);
    
    let error_response: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error_response.error, "Chunk hash mismatch");
    
    server_handle.abort();
}

#[tokio::test]
async fn test_storage_api_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8082)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    // Try to retrieve non-existent chunk
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8082/chunks/nonexistent1234567890abcdef1234567890abcdef1234567890abcdef")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 404);
    
    let error_response: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error_response.error, "Chunk not found");
    
    server_handle.abort();
}

#[tokio::test]
async fn test_storage_api_invalid_hash_format() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8083)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    // Try to retrieve chunk with invalid hash format
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8083/chunks/invalid_hash")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 400);
    
    let error_response: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error_response.error, "Invalid chunk hash format");
    
    server_handle.abort();
}

#[tokio::test]
async fn test_storage_api_list_chunks() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8084)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    let client = reqwest::Client::new();
    
    // Initially should be empty
    let response = client
        .get("http://127.0.0.1:8084/chunks")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    #[derive(serde::Deserialize)]
    struct ChunkListResponse {
        chunks: Vec<String>,
        count: usize,
    }
    
    let list_result: ChunkListResponse = response.json().await.unwrap();
    assert_eq!(list_result.count, 0);
    assert_eq!(list_result.chunks.len(), 0);
    
    // Store a chunk
    let test_data = b"Test chunk for listing";
    let expected_hash = calculate_chunk_hash(test_data);
    
    let store_response = client
        .post("http://127.0.0.1:8084/chunks")
        .header("content-type", "application/octet-stream")
        .body(test_data.to_vec())
        .send()
        .await
        .unwrap();
    
    assert_eq!(store_response.status(), 201);
    
    // List chunks again
    let response = client
        .get("http://127.0.0.1:8084/chunks")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    let list_result: ChunkListResponse = response.json().await.unwrap();
    assert_eq!(list_result.count, 1);
    assert_eq!(list_result.chunks.len(), 1);
    assert_eq!(list_result.chunks[0], expected_hash);
    
    server_handle.abort();
}

#[tokio::test]
async fn test_storage_api_health_check() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8085)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8085/health")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
    
    #[derive(serde::Deserialize)]
    struct HealthResponse {
        status: String,
        timestamp: u64,
        version: String,
    }
    
    let health_result: HealthResponse = response.json().await.unwrap();
    assert_eq!(health_result.status, "healthy");
    assert!(health_result.timestamp > 0);
    assert!(!health_result.version.is_empty());
    
    server_handle.abort();
}

#[tokio::test]
async fn test_storage_api_empty_chunk() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8086)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    // Try to store empty chunk
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8086/chunks")
        .header("content-type", "application/octet-stream")
        .body(Vec::<u8>::new())
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 400);
    
    let error_response: ErrorResponse = response.json().await.unwrap();
    assert_eq!(error_response.error, "Empty chunk data");
    
    server_handle.abort();
}

#[tokio::test]
async fn test_storage_api_large_chunk() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8087)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    // Create a large chunk (1MB)
    let large_data = vec![0xABu8; 1024 * 1024];
    let expected_hash = calculate_chunk_hash(&large_data);
    
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8087/chunks")
        .header("content-type", "application/octet-stream")
        .header("x-chunk-hash", &expected_hash)
        .body(large_data.clone())
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 201);
    
    let store_result: ChunkUploadResponse = response.json().await.unwrap();
    assert_eq!(store_result.chunk_hash, expected_hash);
    assert_eq!(store_result.size, large_data.len());
    
    // Retrieve and verify
    let retrieve_response = client
        .get(&format!("http://127.0.0.1:8087/chunks/{}", expected_hash))
        .send()
        .await
        .unwrap();
    
    assert_eq!(retrieve_response.status(), 200);
    
    let retrieved_data = retrieve_response.bytes().await.unwrap();
    assert_eq!(retrieved_data.len(), large_data.len());
    assert_eq!(retrieved_data.as_ref(), large_data.as_slice());
    
    server_handle.abort();
}

#[tokio::test]
async fn test_concurrent_chunk_operations() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let server = StorageNodeServer::new(storage_path, 0);
    let api = server.create_api();
    
    let server_handle = tokio::spawn(async move {
        warp::serve(api).run(([127, 0, 0, 1], 8088)).await;
    });
    
    sleep(Duration::from_millis(100)).await;
    
    let client = reqwest::Client::new();
    
    // Store multiple chunks concurrently
    let mut store_tasks = Vec::new();
    
    for i in 0..10 {
        let client = client.clone();
        let data = format!("Test chunk {}", i).into_bytes();
        let hash = calculate_chunk_hash(&data);
        
        let task = tokio::spawn(async move {
            let response = client
                .post("http://127.0.0.1:8088/chunks")
                .header("content-type", "application/octet-stream")
                .header("x-chunk-hash", &hash)
                .body(data)
                .send()
                .await
                .unwrap();
            
            (hash, response.status().as_u16())
        });
        
        store_tasks.push(task);
    }
    
    // Wait for all stores to complete
    let mut stored_hashes = Vec::new();
    for task in store_tasks {
        let (hash, status) = task.await.unwrap();
        assert_eq!(status, 201);
        stored_hashes.push(hash);
    }
    
    // Retrieve all chunks concurrently
    let mut retrieve_tasks = Vec::new();
    
    for hash in &stored_hashes {
        let client = client.clone();
        let hash = hash.clone();
        
        let task = tokio::spawn(async move {
            let response = client
                .get(&format!("http://127.0.0.1:8088/chunks/{}", hash))
                .send()
                .await
                .unwrap();
            
            (hash, response.status().as_u16())
        });
        
        retrieve_tasks.push(task);
    }
    
    // Wait for all retrievals to complete
    for task in retrieve_tasks {
        let (_, status) = task.await.unwrap();
        assert_eq!(status, 200);
    }
    
    // Verify all chunks are listed
    let list_response = client
        .get("http://127.0.0.1:8088/chunks")
        .send()
        .await
        .unwrap();
    
    assert_eq!(list_response.status(), 200);
    
    #[derive(serde::Deserialize)]
    struct ChunkListResponse {
        chunks: Vec<String>,
        count: usize,
    }
    
    let list_result: ChunkListResponse = list_response.json().await.unwrap();
    assert_eq!(list_result.count, 10);
    assert_eq!(list_result.chunks.len(), 10);
    
    // Verify all our hashes are in the list
    for hash in stored_hashes {
        assert!(list_result.chunks.contains(&hash));
    }
    
    server_handle.abort();
}