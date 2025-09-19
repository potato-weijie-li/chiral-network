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

export class FileService {
  private static serviceStarted = false;

  /**
   * Upload file data to the network via Tauri backend
   * @param file The File object to upload
   * @returns Promise with upload result containing hash or error
   */
  static async uploadFile(file: File): Promise<UploadResult> {
    try {
      // Read file as array buffer and convert to byte array
      const arrayBuffer = await file.arrayBuffer();
      const fileData = Array.from(new Uint8Array(arrayBuffer));

      // Call Tauri backend to upload file data to network
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
   * Get all stored files from the backend
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
   * Reset the service started flag (for testing or app restart)
   */
  static resetServiceState(): void {
    this.serviceStarted = false;
  }
}