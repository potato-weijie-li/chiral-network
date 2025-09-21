import { invoke } from "@tauri-apps/api/tauri";

export interface ChunkInfo {
  index: number;
  hash: string;
  size: number;
  encrypted_size: number;
  total_chunks: number;
  file_hash: string;
}

export interface FileUploadResponse {
  file_hash: string;
  chunks: ChunkInfo[];
  total_size: number;
  upload_time: number;
}

export interface ChunkUploadStatus {
  chunk_hash: string;
  uploaded: boolean;
  storage_node_url?: string;
  error?: string;
}

export class FileService {
  private storageNodeUrl = "http://localhost:8080"; // Default storage node URL
  private cachedStoragePath: string | null = null;

  /**
   * Sets the storage node URL for chunk uploads
   */
  setStorageNodeUrl(url: string) {
    this.storageNodeUrl = url;
  }

  /**
   * Gets the current storage path from settings
   */
  async getStoragePath(): Promise<string> {
    if (this.cachedStoragePath) {
      return this.cachedStoragePath;
    }

    try {
      const path = await invoke<string>("get_storage_path_setting");
      this.cachedStoragePath = path;
      return path;
    } catch (error) {
      console.warn("Failed to get storage path from settings:", error);
      return "~/ChiralNetwork/Storage"; // Fallback to default
    }
  }

  /**
   * Sets a new storage path in settings
   */
  async setStoragePath(newPath: string): Promise<boolean> {
    try {
      await invoke<string>("set_storage_path_setting", { newPath });
      this.cachedStoragePath = newPath; // Update cache
      return true;
    } catch (error) {
      console.error("Failed to set storage path:", error);
      return false;
    }
  }

  /**
   * Uploads a file by chunking it and uploading chunks to the storage node
   */
  async uploadFile(file: File, encryptionKey?: string): Promise<FileUploadResponse> {
    try {
      // First, save the file to a temporary location
      const fileData = await this.fileToUint8Array(file);
      const tempPath = await invoke<string>("save_temp_file", {
        fileName: file.name,
        fileData: Array.from(fileData),
      });

      // Chunk the file
      const uploadResponse = await invoke<FileUploadResponse>("chunk_file", {
        filePath: tempPath,
        encryptionKey,
      });

      // Upload chunks to storage node
      const uploadStatuses = await this.uploadChunksToStorageNode(uploadResponse.chunks);

      // Check for upload failures
      const failedUploads = uploadStatuses.filter(status => !status.uploaded);
      if (failedUploads.length > 0) {
        console.warn(`${failedUploads.length} chunks failed to upload:`, failedUploads);
      }

      return uploadResponse;
    } catch (error) {
      throw new Error(`Failed to upload file: ${error}`);
    }
  }

  /**
   * Uploads chunks to the storage node
   */
  private async uploadChunksToStorageNode(chunks: ChunkInfo[]): Promise<ChunkUploadStatus[]> {
    const uploadPromises = chunks.map(chunk =>
      this.uploadSingleChunk(chunk.hash)
    );

    return Promise.all(uploadPromises);
  }

  /**
   * Uploads a single chunk to the storage node
   */
  private async uploadSingleChunk(chunkHash: string): Promise<ChunkUploadStatus> {
    try {
      const result = await invoke<ChunkUploadStatus>("upload_chunk_to_storage_node", {
        chunkHash,
        storageNodeUrl: this.storageNodeUrl,
      });
      return result;
    } catch (error) {
      return {
        chunk_hash: chunkHash,
        uploaded: false,
        storage_node_url: this.storageNodeUrl,
        error: String(error),
      };
    }
  }

  /**
   * Downloads a file from the network by retrieving chunks and reassembling
   */
  async downloadFile(fileHash: string, chunks: ChunkInfo[], outputPath: string, encryptionKey?: string): Promise<string> {
    try {
      // Download chunks from storage nodes
      const downloadPromises = chunks.map(chunk =>
        this.downloadSingleChunk(chunk.hash)
      );

      await Promise.all(downloadPromises);

      // Reassemble the file
      const result = await invoke<string>("reassemble_file", {
        fileHash,
        outputPath,
        chunks,
        encryptionKey,
      });

      return result;
    } catch (error) {
      throw new Error(`Failed to download file: ${error}`);
    }
  }

  /**
   * Downloads a single chunk from the storage node
   */
  private async downloadSingleChunk(chunkHash: string): Promise<boolean> {
    try {
      const result = await invoke<boolean>("download_chunk_from_storage_node", {
        chunkHash,
        storageNodeUrl: this.storageNodeUrl,
      });
      return result;
    } catch (error) {
      console.error(`Failed to download chunk ${chunkHash}:`, error);
      return false;
    }
  }

  /**
   * Gets information about a locally stored chunk
   */
  async getChunkInfo(chunkHash: string): Promise<ChunkInfo | null> {
    try {
      const result = await invoke<ChunkInfo | null>("get_chunk_info", {
        chunkHash,
      });
      return result;
    } catch (error) {
      console.error(`Failed to get chunk info for ${chunkHash}:`, error);
      return null;
    }
  }

  /**
   * Lists all locally stored chunks
   */
  async getLocalChunks(): Promise<string[]> {
    try {
      const result = await invoke<string[]>("get_local_chunks");
      return result;
    } catch (error) {
      console.error("Failed to get local chunks:", error);
      return [];
    }
  }

  /**
   * Calculates the hash of a file
   */
  async calculateFileHash(file: File): Promise<string> {
    try {
      // Save file to temporary location
      const fileData = await this.fileToUint8Array(file);
      const tempPath = await invoke<string>("save_temp_file", {
        fileName: file.name,
        fileData: Array.from(fileData),
      });

      // Calculate hash
      const hash = await invoke<string>("calculate_file_hash", {
        filePath: tempPath,
      });

      return hash;
    } catch (error) {
      throw new Error(`Failed to calculate file hash: ${error}`);
    }
  }

  /**
   * Cleans up temporary files
   */
  async cleanupTempFiles(): Promise<number> {
    try {
      const result = await invoke<number>("cleanup_temp_files");
      return result;
    } catch (error) {
      console.error("Failed to cleanup temp files:", error);
      return 0;
    }
  }

  /**
   * Converts a File object to Uint8Array
   */
  private async fileToUint8Array(file: File): Promise<Uint8Array> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        if (reader.result instanceof ArrayBuffer) {
          resolve(new Uint8Array(reader.result));
        } else {
          reject(new Error("Failed to read file as ArrayBuffer"));
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsArrayBuffer(file);
    });
  }

  /**
   * Uploads multiple files and returns progress information
   */
  async uploadFiles(
    files: File[],
    encryptionKey?: string,
    onProgress?: (file: File, progress: number) => void
  ): Promise<FileUploadResponse[]> {
    const results: FileUploadResponse[] = [];

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      
      try {
        onProgress?.(file, 0);
        const result = await this.uploadFile(file, encryptionKey);
        results.push(result);
        onProgress?.(file, 100);
      } catch (error) {
        console.error(`Failed to upload file ${file.name}:`, error);
        onProgress?.(file, -1); // -1 indicates error
        throw error;
      }
    }

    return results;
  }

  /**
   * Tests connection to the storage node
   */
  async testStorageNodeConnection(): Promise<boolean> {
    try {
      const response = await fetch(`${this.storageNodeUrl}/health`);
      return response.ok;
    } catch (error) {
      console.error("Storage node connection test failed:", error);
      return false;
    }
  }

  /**
   * Gets storage node health information
   */
  async getStorageNodeHealth(): Promise<any> {
    try {
      const response = await fetch(`${this.storageNodeUrl}/health`);
      if (response.ok) {
        return await response.json();
      }
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    } catch (error) {
      throw new Error(`Failed to get storage node health: ${error}`);
    }
  }

  /**
   * Lists chunks available on the storage node
   */
  async listStorageNodeChunks(): Promise<string[]> {
    try {
      const response = await fetch(`${this.storageNodeUrl}/chunks`);
      if (response.ok) {
        const data = await response.json();
        return data.chunks || [];
      }
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    } catch (error) {
      throw new Error(`Failed to list storage node chunks: ${error}`);
    }
  }
}

// Export a singleton instance for easy use
export const fileService = new FileService();