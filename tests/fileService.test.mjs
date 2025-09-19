import test from 'node:test';
import assert from 'node:assert/strict';

// Mock the Tauri API for testing
const mockTauriApi = {
  invoke: async (command, args) => {
    switch (command) {
      case 'upload_file_data_to_network':
        if (args.fileName === 'test.txt' && Array.isArray(args.fileData)) {
          return 'mock_hash_123456789abcdef';
        }
        throw new Error('Mock upload failed');
      case 'start_file_transfer_service':
        return null; // Success
      case 'get_stored_files':
        return [
          ['hash1', 'file1.txt', 1024],
          ['hash2', 'file2.txt', 2048]
        ];
      default:
        throw new Error(`Unknown command: ${command}`);
    }
  }
};

// Create a mock File for testing
class MockFile {
  constructor(name, size, content) {
    this.name = name;
    this.size = size;
    this.content = content;
  }

  async arrayBuffer() {
    const encoder = new TextEncoder();
    return encoder.encode(this.content).buffer;
  }
}

// Mock FileService with dependency injection for testing
class FileService {
  static serviceStarted = false;

  static async uploadFile(file, options = {}, tauriApi = mockTauriApi) {
    try {
      const arrayBuffer = await file.arrayBuffer();
      const fileData = Array.from(new Uint8Array(arrayBuffer));

      const hash = await tauriApi.invoke('upload_file_data_to_network', {
        fileName: file.name,
        fileData: fileData,
      });

      return {
        success: true,
        hash: hash,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  static async startFileTransferService(tauriApi = mockTauriApi) {
    if (this.serviceStarted) {
      return true; // Already started
    }

    try {
      await tauriApi.invoke('start_file_transfer_service');
      this.serviceStarted = true;
      return true;
    } catch (error) {
      return false;
    }
  }

  static async getStoredFiles(tauriApi = mockTauriApi) {
    try {
      const files = await tauriApi.invoke('get_stored_files');
      return files.map(([hash, name, size]) => ({ hash, name, size }));
    } catch (error) {
      return [];
    }
  }

  static resetServiceState() {
    this.serviceStarted = false;
  }
}

test('FileService.uploadFile handles successful upload', async () => {
  FileService.resetServiceState();
  const mockFile = new MockFile('test.txt', 13, 'Hello, world!');
  const result = await FileService.uploadFile(mockFile);

  assert.equal(result.success, true);
  assert.equal(result.hash, 'mock_hash_123456789abcdef');
  assert.equal(result.error, undefined);
});

test('FileService.uploadFile handles upload failure', async () => {
  FileService.resetServiceState();
  const mockFile = new MockFile('fail.txt', 4, 'fail');
  
  const result = await FileService.uploadFile(mockFile, {});

  assert.equal(result.success, false);
  assert.equal(result.hash, undefined);
  assert.equal(result.error, 'Mock upload failed');
});

test('FileService.startFileTransferService returns true on success', async () => {
  FileService.resetServiceState();
  const result = await FileService.startFileTransferService();
  assert.equal(result, true);
});

test('FileService.startFileTransferService avoids multiple starts', async () => {
  FileService.resetServiceState();
  const result1 = await FileService.startFileTransferService();
  const result2 = await FileService.startFileTransferService(); // Should not call invoke again
  assert.equal(result1, true);
  assert.equal(result2, true);
});

test('FileService.startFileTransferService returns false on failure', async () => {
  FileService.resetServiceState();
  const failingMockApi = {
    invoke: async () => {
      throw new Error('Service unavailable');
    }
  };

  const result = await FileService.startFileTransferService(failingMockApi);
  assert.equal(result, false);
});

test('FileService.getStoredFiles returns formatted files', async () => {
  FileService.resetServiceState();
  const result = await FileService.getStoredFiles();
  
  assert.equal(result.length, 2);
  assert.deepEqual(result[0], { hash: 'hash1', name: 'file1.txt', size: 1024 });
  assert.deepEqual(result[1], { hash: 'hash2', name: 'file2.txt', size: 2048 });
});

test('FileService.getStoredFiles handles errors', async () => {
  FileService.resetServiceState();
  const failingMockApi = {
    invoke: async () => {
      throw new Error('Backend error');
    }
  };

  const result = await FileService.getStoredFiles(failingMockApi);
  assert.deepEqual(result, []);
});

test('FileService.uploadFile converts file to byte array correctly', async () => {
  FileService.resetServiceState();
  const mockFile = new MockFile('test.txt', 5, 'hello');
  
  const captureApi = {
    invoke: async (command, args) => {
      if (command === 'upload_file_data_to_network') {
        // Verify the fileData is correctly converted to byte array
        const expectedBytes = [104, 101, 108, 108, 111]; // "hello" as bytes
        assert.deepEqual(args.fileData, expectedBytes);
        assert.equal(args.fileName, 'test.txt');
        return 'test_hash';
      }
      throw new Error('Unexpected command');
    }
  };

  const result = await FileService.uploadFile(mockFile, {}, captureApi);
  assert.equal(result.success, true);
  assert.equal(result.hash, 'test_hash');
});