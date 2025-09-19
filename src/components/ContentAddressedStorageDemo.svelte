<script lang="ts">
  import { onMount } from 'svelte';
  import ContentAddressedStorage, { type FileManifest, type StorageStats } from '../lib/contentAddressedStorage';
  import { Button } from '$lib/components/ui/button';
  import { Progress } from '$lib/components/ui/progress';
  import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import { FileText, Download, Upload, Settings, HardDrive } from 'lucide-svelte';
  
  let storageStats: StorageStats | null = null;
  let manifests: FileManifest[] = [];
  let selectedFile: FileManifest | null = null;
  let chunkSize = 1.0; // MB
  let isLoading = false;
  let error = '';
  
  // File upload handling
  let fileInput: HTMLInputElement;
  
  onMount(async () => {
    try {
      // Initialize storage with default path
      await ContentAddressedStorage.initChunkManager('~/ChiralNetwork/Storage');
      await loadStorageStats();
    } catch (err) {
      error = `Failed to initialize storage: ${err}`;
    }
  });
  
  async function loadStorageStats() {
    try {
      storageStats = await ContentAddressedStorage.getStorageStats();
    } catch (err) {
      error = `Failed to load storage stats: ${err}`;
    }
  }
  
  async function handleFileUpload(event: Event) {
    const target = event.target as HTMLInputElement;
    const files = target.files;
    
    if (!files || files.length === 0) return;
    
    isLoading = true;
    error = '';
    
    try {
      // Set chunk size before storing
      await ContentAddressedStorage.setChunkSize(chunkSize);
      
      for (const file of files) {
        // For demo purposes, we're using the file path
        // In a real implementation, this would be handled by Tauri's file system API
        const manifest = await ContentAddressedStorage.storeFile(file.name);
        manifests = [...manifests, manifest];
      }
      
      await loadStorageStats();
    } catch (err) {
      error = `Failed to store file: ${err}`;
    } finally {
      isLoading = false;
      target.value = ''; // Reset file input
    }
  }
  
  async function checkFileProgress(manifest: FileManifest) {
    try {
      const missingChunks = await ContentAddressedStorage.verifyChunksAvailable(manifest.file_hash);
      return ContentAddressedStorage.calculateProgress(
        manifest.total_chunks - missingChunks.length,
        manifest.total_chunks
      );
    } catch {
      return 0;
    }
  }
  
  async function downloadFile(manifest: FileManifest) {
    try {
      isLoading = true;
      const outputPath = `~/Downloads/${manifest.file_name}`;
      await ContentAddressedStorage.reconstructFile(manifest.file_hash, outputPath);
      
      // Show success message or update UI
      console.log(`File downloaded to ${outputPath}`);
    } catch (err) {
      error = `Failed to download file: ${err}`;
    } finally {
      isLoading = false;
    }
  }
  
  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }
</script>

<div class="space-y-6 p-6">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-bold">Content-Addressed Storage</h2>
    <div class="flex items-center gap-2">
      <Settings class="h-4 w-4" />
      <label class="text-sm">Chunk Size (MB):</label>
      <input 
        type="number" 
        bind:value={chunkSize} 
        min="0.1" 
        max="100" 
        step="0.1"
        class="w-16 px-2 py-1 text-sm border rounded"
        disabled={isLoading}
      />
    </div>
  </div>

  {#if error}
    <div class="p-4 text-red-600 bg-red-50 border border-red-200 rounded">
      {error}
    </div>
  {/if}

  <!-- Storage Statistics -->
  {#if storageStats}
    <div class="bg-white rounded-lg border shadow-sm">
      <div class="p-6 border-b">
        <h3 class="font-semibold flex items-center gap-2">
          <HardDrive class="h-5 w-5" />
          Storage Statistics
        </h3>
      </div>
      <div class="p-6">
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div class="text-center">
            <div class="text-2xl font-bold">{storageStats.total_chunks}</div>
            <div class="text-sm text-gray-600">Total Chunks</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold">{storageStats.total_manifests}</div>
            <div class="text-sm text-gray-600">Files</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold">
              {ContentAddressedStorage.formatBytes(storageStats.chunks_storage_bytes)}
            </div>
            <div class="text-sm text-gray-600">Chunk Data</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold">
              {ContentAddressedStorage.formatBytes(storageStats.total_storage_bytes)}
            </div>
            <div class="text-sm text-gray-600">Total Used</div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- File Upload -->
  <div class="bg-white rounded-lg border shadow-sm">
    <div class="p-6 border-b">
      <h3 class="font-semibold flex items-center gap-2">
        <Upload class="h-5 w-5" />
        Add Files to Storage
      </h3>
    </div>
    <div class="p-6">
      <div class="space-y-4">
        <input 
          type="file" 
          bind:this={fileInput}
          on:change={handleFileUpload}
          multiple
          class="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded file:border-0 file:text-sm file:font-semibold file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
          disabled={isLoading}
        />
        
        <div class="text-sm text-gray-600">
          Files will be split into {ContentAddressedStorage.formatBytes(chunkSize * 1024 * 1024)} chunks
          for deduplication and integrity verification.
        </div>
      </div>
    </div>
  </div>

  <!-- File List -->
  {#if manifests.length > 0}
    <div class="bg-white rounded-lg border shadow-sm">
      <div class="p-6 border-b">
        <h3 class="font-semibold flex items-center gap-2">
          <FileText class="h-5 w-5" />
          Stored Files
        </h3>
      </div>
      <div class="p-6">
        <div class="space-y-3">
          {#each manifests as manifest}
            <div class="flex items-center justify-between p-3 border rounded-lg hover:bg-gray-50">
              <div class="flex-1">
                <div class="font-medium">{manifest.file_name}</div>
                <div class="text-sm text-gray-600">
                  {ContentAddressedStorage.getFileSizeFormatted(manifest)} • 
                  {manifest.total_chunks} chunks • 
                  Created {formatTimestamp(manifest.timestamps.created)}
                </div>
                <div class="text-xs text-gray-500 mt-1">
                  Hash: {manifest.file_hash.substring(0, 16)}...
                </div>
                
                {#if manifest.mime_type}
                  <span class="inline-block px-2 py-1 text-xs bg-gray-100 text-gray-700 rounded mt-1">
                    {manifest.mime_type}
                  </span>
                {/if}
              </div>
              
              <div class="flex items-center gap-2">
                {#await checkFileProgress(manifest)}
                  <div class="w-24">
                    <div class="w-full bg-gray-200 rounded-full h-2">
                      <div class="bg-blue-600 h-2 rounded-full" style="width: 0%"></div>
                    </div>
                  </div>
                {:then progress}
                  <div class="w-24">
                    <div class="w-full bg-gray-200 rounded-full h-2">
                      <div class="bg-blue-600 h-2 rounded-full" style="width: {progress}%"></div>
                    </div>
                    <div class="text-xs text-center mt-1">{progress}%</div>
                  </div>
                {:catch}
                  <div class="w-24">
                    <div class="w-full bg-gray-200 rounded-full h-2">
                      <div class="bg-red-600 h-2 rounded-full" style="width: 0%"></div>
                    </div>
                    <div class="text-xs text-center mt-1">Error</div>
                  </div>
                {/await}
                
                <button
                  class="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-50 disabled:opacity-50 flex items-center gap-1"
                  on:click={() => downloadFile(manifest)}
                  disabled={isLoading}
                >
                  <Download class="h-3 w-3" />
                  Download
                </button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  {#if isLoading}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="bg-white p-6 rounded-lg shadow-lg">
        <div class="flex items-center gap-3">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Processing...</span>
        </div>
      </div>
    </div>
  {/if}
</div>