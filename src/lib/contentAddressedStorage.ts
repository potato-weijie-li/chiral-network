// Content-addressed storage types and helpers for the frontend
import { invoke } from '@tauri-apps/api/core';

export interface ChunkInfo {
  index: number;
  hash: string;
  size: number;
  encrypted_size: number;
  offset: number;
}

export interface EncryptionInfo {
  algorithm: string;
  encrypted_key_bundle?: {
    ephemeral_public_key: string;
    encrypted_key: string;
    nonce: string;
  };
}

export interface TimestampInfo {
  created: number;
  modified: number;
  accessed: number;
}

export interface FileManifest {
  version: string;
  file_hash: string;
  file_name: string;
  file_size: number;
  mime_type?: string;
  chunk_size: number;
  total_chunks: number;
  chunks: ChunkInfo[];
  encryption?: EncryptionInfo;
  timestamps: TimestampInfo;
  manifest_hash: string;
}

export interface StorageStats {
  total_chunks: number;
  total_manifests: number;
  chunks_storage_bytes: number;
  manifests_storage_bytes: number;
  total_storage_bytes: number;
}

/**
 * Content-addressed storage API for Chiral Network
 */
export class ContentAddressedStorage {
  /**
   * Initialize the chunk manager with a storage path
   */
  static async initChunkManager(storagePath: string): Promise<void> {
    return invoke('init_chunk_manager', { storagePath });
  }

  /**
   * Store a file using content-addressed chunking
   */
  static async storeFile(filePath: string, encrypt?: boolean): Promise<FileManifest> {
    return invoke('store_file_with_chunks', { filePath, encrypt });
  }

  /**
   * Load a file manifest by its hash
   */
  static async loadManifest(fileHash: string): Promise<FileManifest> {
    return invoke('load_file_manifest', { fileHash });
  }

  /**
   * Reconstruct a file from its chunks
   */
  static async reconstructFile(fileHash: string, outputPath: string): Promise<void> {
    return invoke('reconstruct_file_from_chunks', { fileHash, outputPath });
  }

  /**
   * Check which chunks are missing for a file
   */
  static async verifyChunksAvailable(fileHash: string): Promise<string[]> {
    return invoke('verify_chunks_available', { fileHash });
  }

  /**
   * Get storage statistics
   */
  static async getStorageStats(): Promise<StorageStats> {
    return invoke('get_storage_stats');
  }

  /**
   * Set custom chunk size in MB (must be called before storing files)
   */
  static async setChunkSize(sizeMb: number): Promise<void> {
    return invoke('set_chunk_size', { sizeMb });
  }

  /**
   * Format storage size for display
   */
  static formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  /**
   * Calculate chunk progress percentage
   */
  static calculateProgress(availableChunks: number, totalChunks: number): number {
    if (totalChunks === 0) return 100;
    return Math.round((availableChunks / totalChunks) * 100);
  }

  /**
   * Check if a file is fully downloaded (all chunks available)
   */
  static async isFileComplete(fileHash: string): Promise<boolean> {
    try {
      const missingChunks = await this.verifyChunksAvailable(fileHash);
      return missingChunks.length === 0;
    } catch {
      return false;
    }
  }

  /**
   * Get human-readable file size from manifest
   */
  static getFileSizeFormatted(manifest: FileManifest): string {
    return this.formatBytes(manifest.file_size);
  }

  /**
   * Get chunk size formatted for display
   */
  static getChunkSizeFormatted(manifest: FileManifest): string {
    return this.formatBytes(manifest.chunk_size);
  }

  /**
   * Generate a content-addressed storage path for a chunk hash
   */
  static getChunkStoragePath(chunkHash: string, baseDir: string): string {
    const subdir = chunkHash.substring(0, 2);
    return `${baseDir}/chunks/${subdir}/${chunkHash}`;
  }

  /**
   * Validate manifest integrity by checking required fields
   */
  static validateManifest(manifest: any): manifest is FileManifest {
    return (
      manifest &&
      typeof manifest.version === 'string' &&
      typeof manifest.file_hash === 'string' &&
      typeof manifest.file_name === 'string' &&
      typeof manifest.file_size === 'number' &&
      typeof manifest.chunk_size === 'number' &&
      typeof manifest.total_chunks === 'number' &&
      Array.isArray(manifest.chunks) &&
      manifest.timestamps &&
      typeof manifest.manifest_hash === 'string'
    );
  }

  /**
   * Calculate estimated download time based on chunks and network speed
   */
  static estimateDownloadTime(
    missingChunks: number,
    chunkSize: number,
    networkSpeedBps: number
  ): number {
    if (networkSpeedBps <= 0) return 0;
    const totalBytes = missingChunks * chunkSize;
    return totalBytes / networkSpeedBps; // seconds
  }

  /**
   * Format time duration for display
   */
  static formatDuration(seconds: number): string {
    if (seconds < 60) return `${Math.round(seconds)}s`;
    if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
    return `${Math.round(seconds / 3600)}h`;
  }
}

export default ContentAddressedStorage;