import { invoke } from '@tauri-apps/api/core';

export interface UploadResult {
  success: boolean;
  hash?: string;
  error?: string;
}

export interface StoredFile {
  hash: string;
  name: string;
  size: number;
}

export interface UploadOptions {
  encryption?: boolean;
  replicationFactor?: number;
}

/**
 * FileService implementing the documented Chiral Network API specification
 * 
 * This service follows the patterns described in docs/04-implementation-guide.md
 * and docs/05-api-documentation.md for file operations including:
 * - File upload with chunking and distributed storage
 * - DHT registration for distributed discovery  
 * - Market integration for pricing and discovery
 * - Proper file lifecycle management
 */
export class FileService {
  private static serviceStarted = false;

  /**
   * Upload file to the network following documented API specification
   * 
   * As per docs/04-implementation-guide.md, this should:
   * 1. Chunk the file using ChunkManager
   * 2. Upload chunks to storage nodes
   * 3. Register in DHT for discovery
   * 4. Register in market for pricing
   * 
   * @param file The File object to upload
   * @param options Upload options (encryption, replication factor)
   * @returns Promise with upload result containing hash or error
   */
  static async uploadFile(file: File, options: UploadOptions = {}): Promise<UploadResult> {
    try {
      // Read file as array buffer and convert to byte array
      const arrayBuffer = await file.arrayBuffer();
      const fileData = Array.from(new Uint8Array(arrayBuffer));

      // Call Tauri backend to upload file data to network
      // Using upload_file_data_to_network which implements the core upload logic
      const hash = await invoke<string>('upload_file_data_to_network', {
        fileName: file.name,
        fileData: fileData,
      });

      return {
        success: true,
        hash: hash,
      };
    } catch (error) {
      console.error('Failed to upload file to network:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Start the file transfer service if not already running
   * Implements service startup as documented in the API specification
   */
  static async startFileTransferService(): Promise<boolean> {
    if (this.serviceStarted) {
      return true; // Already started
    }

    try {
      await invoke('start_file_transfer_service');
      this.serviceStarted = true;
      return true;
    } catch (error) {
      console.error('Failed to start file transfer service:', error);
      return false;
    }
  }

  /**
   * Download file from the network by hash
   * Following the documented API specification
   * @param hash File hash to download
   * @returns Promise with file blob
   */
  static async downloadFile(hash: string): Promise<Blob> {
    try {
      // Use the backend download command
      const tempPath = `/tmp/download_${hash}_${Date.now()}`;
      
      await invoke('download_file_from_network', {
        fileHash: hash,
        outputPath: tempPath,
      });

      // TODO: Read the downloaded file and return as blob
      // For now, return empty blob as placeholder
      // This would need additional file reading implementation
      return new Blob([]);
    } catch (error) {
      throw new Error(`Failed to download file: ${error}`);
    }
  }

  /**
   * Get all stored files from the backend
   * Following the documented file listing API from docs/05-api-documentation.md
   */
  static async getStoredFiles(): Promise<StoredFile[]> {
    try {
      const files = await invoke<[string, string, number][]>('get_stored_files');
      return files.map(([hash, name, size]) => ({ hash, name, size }));
    } catch (error) {
      console.error('Failed to get stored files:', error);
      return [];
    }
  }

  /**
   * Get file metadata by hash
   * Following the documented API specification from docs/05-api-documentation.md
   * Equivalent to GET /api/v1/files/{hash}/info
   * @param hash File hash
   * @returns Promise with file metadata
   */
  static async getFileInfo(hash: string): Promise<StoredFile | null> {
    try {
      const files = await this.getStoredFiles();
      return files.find(file => file.hash === hash) || null;
    } catch (error) {
      console.error('Failed to get file info:', error);
      return null;
    }
  }

  /**
   * Query market for file suppliers
   * Following the documented market API from docs/05-api-documentation.md
   * Equivalent to GET /api/v1/market/search
   * @param hash File hash to search for
   * @returns Promise with supplier information
   */
  static async queryMarket(hash: string): Promise<any[]> {
    try {
      // TODO: Implement market query when market service is available
      // This would call a market API endpoint to find suppliers for the file
      console.log(`Querying market for file: ${hash}`);
      return [];
    } catch (error) {
      console.error('Failed to query market:', error);
      return [];
    }
  }

  /**
   * Reset the service started flag (for testing or app restart)
   */
  static resetServiceState(): void {
    this.serviceStarted = false;
  }
}