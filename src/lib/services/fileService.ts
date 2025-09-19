import { invoke } from "@tauri-apps/api/core";

export interface FileMetadata {
  file_hash: string;
  file_name: string;
  file_size: number;
  chunk_count: number;
  chunk_size: number;
  created_at: number;
  encryption: {
    algorithm: string;
    encrypted: boolean;
  };
  availability: {
    online_nodes: number;
    total_replicas: number;
    health_score: number;
  };
}

export interface ChunkInfo {
  index: number;
  hash: string;
  size: number;
  storage_nodes: string[];
}

export interface UploadResult {
  file_hash: string;
  chunks: ChunkInfo[];
  total_size: number;
  upload_time: number;
}

export interface Supplier {
  id: string;
  ip: string;
  port: number;
  price: number;
  reputation: number;
}

export class FileService {
  /**
   * Upload a file to the network using the Tauri backend
   * This will chunk, encrypt, and distribute the file across storage nodes
   */
  async uploadFile(file: File): Promise<string> {
    // Read file as array buffer
    const buffer = await file.arrayBuffer();
    const bytes = new Uint8Array(buffer);

    // Call Rust backend to process file
    const hash = await invoke<string>("upload_file_data_to_network", {
      fileName: file.name,
      fileData: Array.from(bytes),
    });

    return hash;
  }

  /**
   * Download a file from the network by hash
   */
  async downloadFile(hash: string): Promise<Blob> {
    // Download chunks via Tauri backend
    const chunks = await invoke<Uint8Array[]>("download_file_from_network", {
      fileHash: hash,
      outputPath: `/tmp/download_${hash}`,
    });

    // Combine chunks into blob
    const blob = new Blob(chunks);
    return blob;
  }

  /**
   * Get file metadata and availability information
   */
  async getFileInfo(hash: string): Promise<FileMetadata> {
    try {
      const metadata = await invoke<FileMetadata>("get_file_metadata", {
        fileHash: hash,
      });
      return metadata;
    } catch (error) {
      console.error("Failed to get file metadata:", error);
      throw new Error(`Failed to get file metadata: ${error}`);
    }
  }

  /**
   * Verify that a file has been properly stored with redundancy
   */
  async verifyStorage(hash: string): Promise<boolean> {
    try {
      // Call Tauri backend to verify storage
      const verified = await invoke<boolean>("verify_file_storage", {
        fileHash: hash,
      });
      return verified;
    } catch (error) {
      console.error("Storage verification failed:", error);
      return false;
    }
  }

  /**
   * Get upload progress for a file
   */
  async getUploadProgress(hash: string): Promise<{
    progress: number;
    status: "uploading" | "verifying" | "completed" | "failed";
    chunks_uploaded: number;
    total_chunks: number;
    file_size?: number;
    storage_verified?: boolean;
  }> {
    try {
      const status = await invoke<any>("get_file_upload_status", {
        fileHash: hash,
      });
      return {
        progress: status.progress || 0,
        status: status.status || "failed",
        chunks_uploaded: status.chunks_uploaded || 0,
        total_chunks: status.total_chunks || 0,
        file_size: status.file_size,
        storage_verified: status.storage_verified,
      };
    } catch (error) {
      console.error("Failed to get upload progress:", error);
      return {
        progress: 0,
        status: "failed",
        chunks_uploaded: 0,
        total_chunks: 0,
      };
    }
  }

  /**
   * List all files stored locally
   */
  async listStoredFiles(): Promise<Array<{ hash: string; name: string }>> {
    try {
      // This would call a backend command to list stored files
      // For now, we'll need to implement this in the backend
      return [];
    } catch (error) {
      console.error("Failed to list stored files:", error);
      return [];
    }
  }
}