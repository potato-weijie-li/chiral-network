/**
 * Transaction-backed reputation service
 * Based on docs/reputation.md specification
 */

import { invoke } from '@tauri-apps/api/core';
import {
  type TransactionVerdict,
  type SignedTransactionMessage,
  type PeerReputationSummary,
  type ReputationConfig,
  type CachedScore,
  type BlacklistEntry,
  VerdictOutcome,
  getTrustLevelFromScore,
  DEFAULT_REPUTATION_CONFIG,
} from '$lib/types/reputation';

// ============================================================================
// REPUTATION SERVICE
// ============================================================================

class ReputationService {
  private config: ReputationConfig = DEFAULT_REPUTATION_CONFIG;
  private scoreCache: Map<string, CachedScore> = new Map();
  private issuerSeqNo: number = 0;

  /**
   * Initialize the reputation service with configuration
   */
  async initialize(config?: Partial<ReputationConfig>): Promise<void> {
    if (config) {
      this.config = { ...this.config, ...config };
    }
    // Load configuration from backend if available
    try {
      const backendConfig = await invoke<ReputationConfig>('get_reputation_config');
      this.config = { ...this.config, ...backendConfig };
    } catch (error) {
      console.warn('Could not load reputation config from backend, using defaults:', error);
    }
  }

  /**
   * Get configuration
   */
  getConfig(): ReputationConfig {
    return { ...this.config };
  }

  /**
   * Update configuration
   */
  async updateConfig(updates: Partial<ReputationConfig>): Promise<void> {
    this.config = { ...this.config, ...updates };
    try {
      await invoke('update_reputation_config', { config: this.config });
    } catch (error) {
      console.error('Failed to update reputation config:', error);
      throw error;
    }
  }

  /**
   * Publish a transaction verdict to the DHT
   */
  async publishVerdict(
    targetId: string,
    outcome: VerdictOutcome,
    txHash: string | null,
    details?: string,
    evidenceBlobs?: string[]
  ): Promise<void> {
    const verdict: Partial<TransactionVerdict> = {
      targetId,
      txHash,
      outcome,
      details,
      issuedAt: Math.floor(Date.now() / 1000),
      metric: 'transaction',
      evidenceBlobs,
    };

    try {
      // Backend will sign and publish
      await invoke('publish_transaction_verdict', { verdict });
      this.issuerSeqNo++;
      
      // Invalidate cache for this peer
      this.scoreCache.delete(targetId);
    } catch (error) {
      console.error('Failed to publish verdict:', error);
      throw error;
    }
  }

  /**
   * File a complaint (non-payment or other)
   */
  async fileComplaint(
    targetId: string,
    complaintType: 'non-payment' | 'non-delivery' | 'other',
    evidence: {
      signedTransactionMessage?: SignedTransactionMessage;
      deliveryProof?: any;
      transferCompletionLog?: any;
      protocolLogs?: any;
    },
    submitOnChain: boolean = false
  ): Promise<void> {
    const evidenceBlobs: string[] = [];

    // Serialize evidence
    if (evidence.signedTransactionMessage) {
      evidenceBlobs.push(JSON.stringify({
        type: 'signed_transaction_message',
        data: evidence.signedTransactionMessage,
      }));
    }
    if (evidence.deliveryProof) {
      evidenceBlobs.push(JSON.stringify({
        type: 'delivery_proof',
        data: evidence.deliveryProof,
      }));
    }
    if (evidence.transferCompletionLog) {
      evidenceBlobs.push(JSON.stringify({
        type: 'transfer_completion_log',
        data: evidence.transferCompletionLog,
      }));
    }
    if (evidence.protocolLogs) {
      evidenceBlobs.push(JSON.stringify({
        type: 'protocol_logs',
        data: evidence.protocolLogs,
      }));
    }

    const details = `Complaint: ${complaintType}`;

    // Publish to DHT
    await this.publishVerdict(
      targetId,
      VerdictOutcome.Bad,
      null, // No tx_hash for non-payment
      details,
      evidenceBlobs
    );

    // Submit on-chain if requested
    if (submitOnChain) {
      try {
        await invoke('submit_complaint_onchain', {
          targetId,
          complaintType,
          evidence: evidenceBlobs,
        });
      } catch (error) {
        console.error('Failed to submit complaint on-chain:', error);
        // Don't throw - DHT submission succeeded
      }
    }
  }

  /**
   * Retrieve transaction verdicts for a peer
   */
  async getVerdicts(peerId: string): Promise<TransactionVerdict[]> {
    try {
      const verdicts = await invoke<TransactionVerdict[]>('fetch_transaction_verdicts', {
        peerId,
      });
      return verdicts;
    } catch (error) {
      console.error('Failed to fetch verdicts:', error);
      return [];
    }
  }

  /**
   * Calculate reputation score for a peer
   */
  async getPeerScore(peerId: string, useCache: boolean = true): Promise<number> {
    // Check cache first
    if (useCache) {
      const cached = this.scoreCache.get(peerId);
      if (cached) {
        const age = Math.floor(Date.now() / 1000) - cached.cachedAt;
        if (age < this.config.cacheTtl) {
          return cached.score;
        }
      }
    }

    // Fetch verdicts and calculate score
    const verdicts = await this.getVerdicts(peerId);
    const score = this.calculateScore(verdicts);
    const trustLevel = getTrustLevelFromScore(score);

    // Cache the result
    this.scoreCache.set(peerId, {
      score,
      trustLevel,
      cachedAt: Math.floor(Date.now() / 1000),
    });

    return score;
  }

  /**
   * Get complete reputation summary for a peer
   */
  async getPeerReputation(peerId: string): Promise<PeerReputationSummary> {
    const verdicts = await this.getVerdicts(peerId);
    const score = this.calculateScore(verdicts);
    const trustLevel = getTrustLevelFromScore(score);
    const [successful, failed] = this.countTransactions(verdicts);

    return {
      peerId,
      score,
      trustLevel,
      successfulTransactions: successful,
      failedTransactions: failed,
      totalVerdicts: verdicts.length,
      lastUpdated: Math.floor(Date.now() / 1000),
    };
  }

  /**
   * Calculate weighted reputation score from verdicts
   */
  private calculateScore(verdicts: TransactionVerdict[]): number {
    if (verdicts.length === 0) {
      return 0.0;
    }

    let totalWeight = 0.0;
    let weightedSum = 0.0;

    for (const verdict of verdicts) {
      const value = this.getVerdictValue(verdict.outcome);
      const weight = this.config.decayHalfLife > 0
        ? this.calculateTimeDecayWeight(verdict.issuedAt)
        : 1.0;

      weightedSum += value * weight;
      totalWeight += weight;
    }

    return totalWeight > 0 ? weightedSum / totalWeight : 0.0;
  }

  /**
   * Get numeric value for verdict outcome
   */
  private getVerdictValue(outcome: VerdictOutcome): number {
    switch (outcome) {
      case VerdictOutcome.Good:
        return 1.0;
      case VerdictOutcome.Disputed:
        return 0.5;
      case VerdictOutcome.Bad:
        return 0.0;
      default:
        return 0.0;
    }
  }

  /**
   * Calculate time decay weight for a verdict
   */
  private calculateTimeDecayWeight(issuedAt: number): number {
    const now = Math.floor(Date.now() / 1000);
    const ageSeconds = now - issuedAt;
    const ageDays = ageSeconds / 86400;
    const halfLife = this.config.decayHalfLife;

    return Math.pow(0.5, ageDays / halfLife);
  }

  /**
   * Count successful vs failed transactions
   */
  private countTransactions(verdicts: TransactionVerdict[]): [number, number] {
    let successful = 0;
    let failed = 0;

    for (const verdict of verdicts) {
      if (verdict.outcome === VerdictOutcome.Good) {
        successful++;
      } else if (verdict.outcome === VerdictOutcome.Bad) {
        failed++;
      }
      // Disputed verdicts don't count towards either
    }

    return [successful, failed];
  }

  /**
   * Create a signed transaction message
   */
  async createSignedTransactionMessage(
    from: string,
    to: string,
    amount: number,
    fileHash: string,
    deadline?: number
  ): Promise<SignedTransactionMessage> {
    const nonce = this.generateNonce();
    const finalDeadline = deadline || (Math.floor(Date.now() / 1000) + this.config.paymentDeadlineDefault);

    const message: Omit<SignedTransactionMessage, 'downloaderSignature'> = {
      from,
      to,
      amount,
      fileHash,
      nonce,
      deadline: finalDeadline,
    };

    try {
      // Backend will sign the message
      const signed = await invoke<SignedTransactionMessage>('sign_transaction_message', {
        message,
      });
      return signed;
    } catch (error) {
      console.error('Failed to sign transaction message:', error);
      throw error;
    }
  }

  /**
   * Verify a signed transaction message
   */
  async verifySignedTransactionMessage(
    message: SignedTransactionMessage,
    fromPublicKey: string
  ): Promise<boolean> {
    try {
      const isValid = await invoke<boolean>('verify_transaction_message', {
        message,
        publicKey: fromPublicKey,
      });
      return isValid;
    } catch (error) {
      console.error('Failed to verify transaction message:', error);
      return false;
    }
  }

  /**
   * Check if downloader has sufficient balance
   */
  async checkDownloaderBalance(
    downloaderAddress: string,
    filePrice: number
  ): Promise<boolean> {
    try {
      const balance = await invoke<number>('get_wallet_balance', {
        address: downloaderAddress,
      });
      const requiredBalance = filePrice * this.config.minBalanceMultiplier;
      return balance >= requiredBalance;
    } catch (error) {
      console.error('Failed to check downloader balance:', error);
      return false;
    }
  }

  /**
   * Validate handshake before accepting transfer
   */
  async validateHandshake(
    signedMessage: SignedTransactionMessage,
    downloaderPublicKey: string
  ): Promise<{ valid: boolean; reason?: string }> {
    // 1. Verify signature
    const sigValid = await this.verifySignedTransactionMessage(
      signedMessage,
      downloaderPublicKey
    );
    if (!sigValid) {
      return { valid: false, reason: 'Invalid signature' };
    }

    // 2. Check downloader's reputation
    const reputation = await this.getPeerScore(signedMessage.from);
    if (reputation < 0.2) { // Below Unknown threshold
      return { valid: false, reason: 'Downloader reputation too low' };
    }

    // 3. Check downloader's balance
    const hasBalance = await this.checkDownloaderBalance(
      signedMessage.from,
      signedMessage.amount
    );
    if (!hasBalance) {
      return { valid: false, reason: 'Insufficient balance' };
    }

    // 4. Check deadline is reasonable
    const now = Math.floor(Date.now() / 1000);
    const minTransferTime = 300; // 5 minutes minimum
    if (signedMessage.deadline < now + minTransferTime) {
      return { valid: false, reason: 'Deadline too soon' };
    }

    return { valid: true };
  }

  /**
   * Generate a unique nonce for signed messages
   */
  private generateNonce(): string {
    const timestamp = Date.now();
    const random = Math.random().toString(36).substring(2, 15);
    return `${timestamp}-${random}`;
  }

  /**
   * Clear score cache
   */
  clearCache(): void {
    this.scoreCache.clear();
  }

  /**
   * Clean up stale cache entries
   */
  cleanupCache(): void {
    const now = Math.floor(Date.now() / 1000);
    for (const [peerId, cached] of this.scoreCache.entries()) {
      const age = now - cached.cachedAt;
      if (age >= this.config.cacheTtl) {
        this.scoreCache.delete(peerId);
      }
    }
  }
}

// ============================================================================
// BLACKLIST SERVICE
// ============================================================================

class BlacklistService {
  /**
   * Manually blacklist a peer
   */
  async addManual(peerId: string, reason: string): Promise<void> {
    try {
      await invoke('blacklist_peer_manual', { peerId, reason });
    } catch (error) {
      console.error('Failed to blacklist peer:', error);
      throw error;
    }
  }

  /**
   * Remove peer from blacklist
   */
  async remove(peerId: string): Promise<void> {
    try {
      await invoke('blacklist_peer_remove', { peerId });
    } catch (error) {
      console.error('Failed to remove peer from blacklist:', error);
      throw error;
    }
  }

  /**
   * Check if peer is blacklisted
   */
  async isBlacklisted(peerId: string): Promise<boolean> {
    try {
      const blacklisted = await invoke<boolean>('blacklist_peer_check', { peerId });
      return blacklisted;
    } catch (error) {
      console.error('Failed to check blacklist:', error);
      return false;
    }
  }

  /**
   * List all blacklisted peers
   */
  async listAll(): Promise<BlacklistEntry[]> {
    try {
      const entries = await invoke<BlacklistEntry[]>('blacklist_peer_list');
      return entries;
    } catch (error) {
      console.error('Failed to list blacklist:', error);
      return [];
    }
  }

  /**
   * Clean up expired automatic blacklist entries
   */
  async cleanupExpired(): Promise<number> {
    try {
      const removed = await invoke<number>('blacklist_cleanup_expired');
      return removed;
    } catch (error) {
      console.error('Failed to cleanup blacklist:', error);
      return 0;
    }
  }
}

// ============================================================================
// EXPORTS
// ============================================================================

export const reputationService = new ReputationService();
export const blacklistService = new BlacklistService();
