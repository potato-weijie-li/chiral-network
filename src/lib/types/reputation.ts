/**
 * Transaction-backed reputation system types
 * Based on docs/reputation.md specification
 */

// ============================================================================
// CORE REPUTATION TYPES
// ============================================================================

export enum VerdictOutcome {
  Good = 'good',
  Disputed = 'disputed',
  Bad = 'bad',
}

export interface TransactionVerdict {
  /** Peer whose reputation is being updated */
  targetId: string;
  /** Blockchain transaction hash (null for non-payment complaints) */
  txHash: string | null;
  /** Outcome: good, bad, or disputed */
  outcome: VerdictOutcome;
  /** Optional plain-text metadata (≤ 1 KB) */
  details?: string;
  /** Optional metric label (defaults to "transaction") */
  metric?: string;
  /** Unix timestamp when verdict was issued */
  issuedAt: number;
  /** Peer ID of the issuer */
  issuerId: string;
  /** Monotonic counter per issuer to prevent duplicates */
  issuerSeqNo: number;
  /** Hex-encoded signature over the canonical signable payload */
  issuerSig: string;
  /** Optional on-chain receipt pointer */
  txReceipt?: string;
  /** Optional evidence blobs (critical for non-payment complaints) */
  evidenceBlobs?: string[];
}

export interface SignedTransactionMessage {
  /** Downloader's blockchain address */
  from: string;
  /** Seeder's blockchain address */
  to: string;
  /** Payment amount */
  amount: number;
  /** File hash being transferred */
  fileHash: string;
  /** Unique identifier to prevent replay attacks */
  nonce: string;
  /** Unix timestamp deadline for transfer completion */
  deadline: number;
  /** Cryptographic signature from downloader */
  downloaderSignature: string;
}

export enum TrustLevel {
  Unknown = 'Unknown',   // 0.0 - 0.2
  Low = 'Low',           // 0.2 - 0.4
  Medium = 'Medium',     // 0.4 - 0.6
  High = 'High',         // 0.6 - 0.8
  Trusted = 'Trusted',   // 0.8 - 1.0
}

export interface BlacklistEntry {
  peerId: string;
  reason: string;
  blacklistedAt: number;
  isAutomatic: boolean;
  evidence?: string[];
}

export interface ReputationConfig {
  /** Blocks required before transaction counts for reputation */
  confirmationThreshold: number;
  /** Max duration to keep verdict pending (seconds) */
  confirmationTimeout: number;
  /** Transactions needed to reach max base score */
  maturityThreshold: number;
  /** Half-life for time decay (days), 0 = disabled */
  decayHalfLife: number;
  /** Duration to keep accepted verdicts (days) */
  retentionPeriod: number;
  /** Max bytes in details field */
  maxVerdictSize: number;
  /** Duration to cache scores locally (seconds) */
  cacheTtl: number;
  /** Blacklist mode: manual, automatic, hybrid */
  blacklistMode: 'manual' | 'automatic' | 'hybrid';
  /** Enable automatic blacklisting */
  blacklistAutoEnabled: boolean;
  /** Score threshold for auto-blacklist (0.0-1.0) */
  blacklistScoreThreshold: number;
  /** Bad verdicts needed for auto-blacklist */
  blacklistBadVerdictsThreshold: number;
  /** Blacklist retention period (days) */
  blacklistRetention: number;
  /** Default deadline for signed messages (seconds) */
  paymentDeadlineDefault: number;
  /** Grace period after deadline (seconds) */
  paymentGracePeriod: number;
  /** Min balance as multiple of file price */
  minBalanceMultiplier: number;
}

export interface CachedScore {
  score: number;
  trustLevel: TrustLevel;
  cachedAt: number;
}

export interface PeerReputationSummary {
  peerId: string;
  score: number;
  trustLevel: TrustLevel;
  successfulTransactions: number;
  failedTransactions: number;
  totalVerdicts: number;
  lastUpdated: number;
}

export interface ReputationAnalytics {
  totalPeers: number;
  averageScore: number;
  trustLevelDistribution: Record<TrustLevel, number>;
  recentVerdicts: TransactionVerdict[];
  topPerformers: PeerReputationSummary[];
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

export function getTrustLevelFromScore(score: number): TrustLevel {
  if (score >= 0.8) return TrustLevel.Trusted;
  if (score >= 0.6) return TrustLevel.High;
  if (score >= 0.4) return TrustLevel.Medium;
  if (score >= 0.2) return TrustLevel.Low;
  return TrustLevel.Unknown;
}

export function getTrustLevelScoreRange(level: TrustLevel): [number, number] {
  switch (level) {
    case TrustLevel.Trusted:
      return [0.8, 1.0];
    case TrustLevel.High:
      return [0.6, 0.8];
    case TrustLevel.Medium:
      return [0.4, 0.6];
    case TrustLevel.Low:
      return [0.2, 0.4];
    case TrustLevel.Unknown:
      return [0.0, 0.2];
  }
}

export function getTrustLevelColor(level: TrustLevel): string {
  switch (level) {
    case TrustLevel.Trusted:
      return '#10b981'; // green-500
    case TrustLevel.High:
      return '#3b82f6'; // blue-500
    case TrustLevel.Medium:
      return '#f59e0b'; // amber-500
    case TrustLevel.Low:
      return '#ef4444'; // red-500
    case TrustLevel.Unknown:
      return '#6b7280'; // gray-500
  }
}

export function getVerdictOutcomeColor(outcome: VerdictOutcome): string {
  switch (outcome) {
    case VerdictOutcome.Good:
      return '#10b981'; // green-500
    case VerdictOutcome.Disputed:
      return '#f59e0b'; // amber-500
    case VerdictOutcome.Bad:
      return '#ef4444'; // red-500
  }
}

export const DEFAULT_REPUTATION_CONFIG: ReputationConfig = {
  confirmationThreshold: 12,
  confirmationTimeout: 3600,
  maturityThreshold: 100,
  decayHalfLife: 90,
  retentionPeriod: 90,
  maxVerdictSize: 1024,
  cacheTtl: 600,
  blacklistMode: 'hybrid',
  blacklistAutoEnabled: true,
  blacklistScoreThreshold: 0.2,
  blacklistBadVerdictsThreshold: 3,
  blacklistRetention: 30,
  paymentDeadlineDefault: 3600,
  paymentGracePeriod: 1800,
  minBalanceMultiplier: 1.2,
};
