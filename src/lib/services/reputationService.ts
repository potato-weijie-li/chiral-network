/**
 * Reputation Service - Simple wrapper for signed transaction message reputation
 * See docs/SIGNED_TRANSACTION_MESSAGES.md for full documentation
 */

import { invoke } from '@tauri-apps/api/core';
import { DEFAULT_REPUTATION_CONFIG, type TransactionVerdict } from '$lib/types/reputation';
import {
  reputationRateLimiter,
  RateLimitError,
  type RateLimitDecision,
} from './reputationRateLimiter';

// Compute a normalized score from a set of verdicts with TTL, dedup, and weights.
export function scoreVerdicts(
  verdicts: TransactionVerdict[],
  verdictTtlSeconds: number = DEFAULT_REPUTATION_CONFIG.verdictTTL,
  nowSeconds: number = Math.floor(Date.now() / 1000)
): { score: number; total: number } {
  if (!Array.isArray(verdicts) || verdicts.length === 0) {
    return { score: 0, total: 0 };
  }

  const cutoff = Math.max(0, nowSeconds - verdictTtlSeconds);
  const deduped = new Map<string, TransactionVerdict>();

  for (const verdict of verdicts) {
    if (!verdict || typeof verdict.issued_at !== 'number' || verdict.issued_at < cutoff) {
      continue;
    }

    // Deduplicate on issuer + tx + seq so replays/updates do not inflate counts.
    const txKey = verdict.tx_hash ?? 'no_tx';
    const key = `${verdict.issuer_id}:${txKey}:${verdict.issuer_seq_no}`;
    const existing = deduped.get(key);
    if (!existing || existing.issued_at < verdict.issued_at) {
      deduped.set(key, verdict);
    }
  }

  const finalVerdicts = Array.from(deduped.values());
  if (finalVerdicts.length === 0) {
    return { score: 0, total: 0 };
  }

  let good = 0;
  let disputed = 0;

  for (const verdict of finalVerdicts) {
    if (verdict.outcome === 'good') good += 1;
    else if (verdict.outcome === 'disputed') disputed += 1;
  }

  const total = finalVerdicts.length;
  const score = total > 0 ? (good * 1.0 + disputed * 0.5) / total : 0;
  return { score, total };
}

class ReputationService {
  async publishVerdict(verdict: Partial<TransactionVerdict>): Promise<RateLimitDecision> {
    let decision: RateLimitDecision | null = null;
    let completeVerdict: TransactionVerdict | null = null;

    try {
      // Get DHT peer ID - fallback to get_peer_id if get_dht_peer_id returns null
      let issuerId = verdict.issuer_id;
      if (!issuerId) {
        try {
          const dhtPeerId = await invoke<string | null>('get_dht_peer_id');
          issuerId = dhtPeerId || (await invoke<string>('get_peer_id'));
        } catch (err) {
          console.warn('Failed to get DHT peer ID, trying get_peer_id:', err);
          issuerId = await invoke<string>('get_peer_id');
        }
      }

      completeVerdict = {
        target_id: verdict.target_id!,
        tx_hash: verdict.tx_hash || null,
        outcome: verdict.outcome || 'good',
        details: verdict.details,
        metric: verdict.metric || 'transaction',
        issued_at: verdict.issued_at || Math.floor(Date.now() / 1000),
        issuer_id: issuerId,
        issuer_seq_no: verdict.issuer_seq_no || Date.now(),
        issuer_sig: verdict.issuer_sig || '',
        issuer_pubkey: verdict.issuer_pubkey,
        tx_receipt: verdict.tx_receipt,
        evidence_blobs: verdict.evidence_blobs,
      };

      decision = reputationRateLimiter.evaluate(completeVerdict);
      if (!decision.allowed) {
        reputationRateLimiter.recordDecision(completeVerdict, decision, { sent: false });
        const message = `Reputation verdict blocked by rate limiter (${decision.reason ?? 'limit'})`;
        console.warn(message, decision);
        throw new RateLimitError(message, decision);
      }

      console.log('📊 Publishing reputation verdict to DHT:', completeVerdict);
      await invoke('publish_reputation_verdict', { verdict: completeVerdict });
      reputationRateLimiter.recordDecision(completeVerdict, decision, { sent: true });
      console.log('✅ Published reputation verdict to DHT for peer:', completeVerdict.target_id);
      return decision;
    } catch (error) {
      if (completeVerdict && decision && !(error instanceof RateLimitError)) {
        reputationRateLimiter.recordDecision(completeVerdict, decision, { sent: false });
      }
      console.error('❌ Failed to publish reputation verdict:', error);
      throw error;
    }
  }

  async getReputationVerdicts(peerId: string): Promise<TransactionVerdict[]> {
    try {
      const verdicts = await invoke<TransactionVerdict[]>('get_reputation_verdicts', { peerId });
      return verdicts;
    } catch (error) {
      console.error('❌ Failed to get reputation verdicts:', error);
      return [];
    }
  }

  async getPeerScore(peerId: string): Promise<number> {
    const verdicts = await this.getReputationVerdicts(peerId);
    const { score } = scoreVerdicts(verdicts);
    return score;
  }
}

export const reputationService = new ReputationService();
