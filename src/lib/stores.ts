import { writable } from "svelte/store";

export interface FileItem {
  id: string;
  name: string;
  hash: string;
  size: number;
  status:
    | "downloading"
    | "paused"
    | "completed"
    | "failed"
    | "uploaded"
    | "queued"
    | "seeding"
    | "canceled";
  progress?: number;
  uploadDate?: Date;
  owner?: string;
  description?: string;
  seeders?: number;
  leechers?: number;
  encrypted?: boolean;
  priority?: "low" | "normal" | "high";
  downloadSpeed?: number;
  uploadSpeed?: number;
  timeRemaining?: number;
  visualOrder?: number; // For maintaining user's intended visual order
  downloadPath?: string; // Path where the file was downloaded
}

export interface ProxyNode {
  id: string;
  address: string;
  status: "online" | "offline" | "connecting";
  bandwidth: number;
  latency: number;
  region: string;
  reputation?: number;
  uptime?: number;
  price?: number;
  totalProxied?: number;
}

export interface WalletInfo {
  address: string;
  balance: number;
  pendingTransactions: number;
  totalEarned: number;
  totalSpent: number;
  stakedAmount?: number;
  miningRewards?: number;
  reputation?: number;
}

export interface ETCAccount {
  address: string;
  private_key: string;
}

export interface PeerInfo {
  id: string;
  address: string;
  nickname?: string;
  status: "online" | "offline" | "away";
  reputation: number;
  sharedFiles: number;
  totalSize: number;
  joinDate: Date;
  lastSeen: Date;
  location?: string;
}

export const suspiciousActivity = writable<{ type: string; description: string; date: string; severity: 'low' | 'medium' | 'high' }[]>([]);

export interface ChatMessage {
  id: string;
  peerId: string;
  peerNickname: string;
  content: string;
  timestamp: Date;
  type: "sent" | "received";
  read: boolean;
}

export interface NetworkStats {
  totalPeers: number;
  onlinePeers: number;
  totalFiles: number;
  networkSize: number;
  avgDownloadSpeed: number;
  avgUploadSpeed: number;
  totalTransactions: number;
}

export interface BlacklistEntry {
  chiral_address: string;
  reason: string;
  timestamp: Date;
}

// Sample dummy data
const dummyFiles: FileItem[] = [
  {
    id: "0",
    name: "Video.mp4",
    hash: "QmZ4tDuvesekqMF",
    size: 50331648,
    status: "paused",
    progress: 30,
    visualOrder: 1,
  },
  {
    id: "1",
    name: "Document.pdf",
    hash: "QmZ4tDuvesekqMD",
    size: 2048576,
    status: "completed",
    progress: 100,
    visualOrder: 2,
  },
  {
    id: "2",
    name: "Archive.zip",
    hash: "QmZ4tDuvesekqMG",
    size: 10485760,
    status: "uploaded",
    progress: 100,
    visualOrder: 3,
  },
];

const dummyProxyNodes: ProxyNode[] = [
  {
    id: "1",
    address: "192.168.1.100:8080",
    status: "online",
    bandwidth: 100,
    latency: 20,
    region: "US-East",
  },
  {
    id: "2",
    address: "10.0.0.50:8080",
    status: "online",
    bandwidth: 50,
    latency: 45,
    region: "EU-West",
  },
  {
    id: "3",
    address: "172.16.0.10:8080",
    status: "offline",
    bandwidth: 0,
    latency: 999,
    region: "Asia-Pacific",
  },
  {
    id: "4",
    address: "192.168.2.25:8080",
    status: "connecting",
    bandwidth: 75,
    latency: 30,
    region: "US-West",
  },
];

const dummyWallet: WalletInfo = {
  address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
  balance: 1000.5,
  pendingTransactions: 2,
  totalEarned: 250.75,
  totalSpent: 45.25,
};

// Additional dummy data
const dummyPeers: PeerInfo[] = [
  {
    id: "peer1",
    address: "192.168.1.50:8080",
    nickname: "AliceNode",
    status: "online",
    reputation: 4.8,
    sharedFiles: 150,
    totalSize: 5368709120,
    joinDate: new Date("2024-01-01"),
    lastSeen: new Date(),
    location: "US-East",
  },
  {
    id: "peer2",
    address: "10.0.0.25:8080",
    nickname: "BobStorage",
    status: "offline",
    reputation: 4.5,
    sharedFiles: 89,
    totalSize: 2147483648,
    joinDate: new Date("2024-02-15"),
    lastSeen: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000), // 3 days ago
    location: "EU-West",
  },
  {
    id: "peer3",
    address: "172.16.0.100:8080",
    nickname: "CharlieShare",
    status: "away",
    reputation: 4.2,
    sharedFiles: 45,
    totalSize: 1073741824,
    joinDate: new Date("2024-03-01"),
    lastSeen: new Date(Date.now() - 3600000),
    location: "Asia-Pacific",
  },
];

const blacklistedPeers: BlacklistEntry[] = [
  {
    chiral_address: "0x702f05Cc6634C599f1293b844Bc9e759ef049891",
    reason: "Unfufilled requests",
    timestamp: new Date("2024-05-01T10:00:00Z"),
  },
];

const dummyNetworkStats: NetworkStats = {
  totalPeers: 1247,
  onlinePeers: 892,
  totalFiles: 45678,
  networkSize: 125899906842624, // ~125TB
  avgDownloadSpeed: 12.5, // MB/s
  avgUploadSpeed: 8.3, // MB/s
  totalTransactions: 98765,
};

// Stores
export const files = writable<FileItem[]>(dummyFiles);
export const proxyNodes = writable<ProxyNode[]>(dummyProxyNodes);
export const wallet = writable<WalletInfo>(dummyWallet);
export const activeDownloads = writable<number>(1);

// Import real network status
import { networkStatus } from "./services/networkService";
export { networkStatus };

export const peers = writable<PeerInfo[]>(dummyPeers);
export const chatMessages = writable<ChatMessage[]>([]);
export const networkStats = writable<NetworkStats>(dummyNetworkStats);
export const downloadQueue = writable<FileItem[]>([]);
export const userLocation = writable<string>("US-East");
export const etcAccount = writable<ETCAccount | null>(null);
export const blacklist = writable<BlacklistEntry[]>(blacklistedPeers);

// File loading functionality
import { FileService } from './services/fileService';

/**
 * Load stored files from the backend and update the files store
 */
export async function loadStoredFiles(): Promise<void> {
  try {
    // Start the file transfer service first
    const serviceStarted = await FileService.startFileTransferService();
    if (!serviceStarted) {
      console.warn('File transfer service not available, using dummy data');
      return;
    }

    // Get stored files from backend
    const storedFiles = await FileService.getStoredFiles();
    
    if (storedFiles.length > 0) {
      // Convert backend files to FileItem format
      const backendFiles: FileItem[] = storedFiles.map((file, index) => ({
        id: `backend-${index}`,
        name: file.name,
        hash: file.hash,
        size: file.size,
        status: 'seeding' as const,
        progress: 100,
        seeders: 1,
        leechers: 0,
        uploadDate: new Date(), // We don't have upload date from backend yet
      }));

      // Replace dummy files with real backend files
      files.set(backendFiles);
      console.log(`Loaded ${backendFiles.length} files from backend`);
    } else {
      // No files in backend, keep dummy data for demonstration
      console.log('No files found in backend, keeping dummy data');
    }
  } catch (error) {
    console.error('Failed to load stored files:', error);
    // Keep dummy data on error
  }
}

interface RecentBlock {
  id: string;
  hash: string;
  reward: number;
  timestamp: Date;
  difficulty: number;
  nonce: number;
}

export interface MiningHistoryPoint {
  timestamp: number;
  hashRate: number;
  power: number;
}

// Mining state
export interface MiningState {
  isMining: boolean;
  hashRate: string;
  totalRewards: number;
  blocksFound: number;
  activeThreads: number;
  minerIntensity: number;
  selectedPool: string;
  sessionStartTime?: number; // Track mining session start time for persistence
  recentBlocks?: RecentBlock[]; // Store recent blocks found
  miningHistory?: MiningHistoryPoint[]; // Store hash rate history for charts
}

export const miningState = writable<MiningState>({
  isMining: false,
  hashRate: "0 H/s",
  totalRewards: 0,
  blocksFound: 0,
  activeThreads: 1,
  minerIntensity: 50,
  selectedPool: "solo",
  sessionStartTime: undefined,
  recentBlocks: [],
  miningHistory: [],
});
