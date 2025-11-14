/**
 * Svelte store for reputation data
 * Provides reactive access to reputation information
 */

import { writable, get } from 'svelte/store';
import type {
  PeerReputationSummary,
  ReputationAnalytics,
} from '$lib/types/reputation';
import { reputationService } from '$lib/services/reputationService';
import { TrustLevel } from '$lib/types/reputation';

// ============================================================================
// STORES
// ============================================================================

export const peerReputations = writable<Map<string, PeerReputationSummary>>(new Map());
export const reputationAnalytics = writable<ReputationAnalytics>({
  totalPeers: 0,
  averageScore: 0.0,
  trustLevelDistribution: {
    [TrustLevel.Trusted]: 0,
    [TrustLevel.High]: 0,
    [TrustLevel.Medium]: 0,
    [TrustLevel.Low]: 0,
    [TrustLevel.Unknown]: 0,
  },
  recentVerdicts: [],
  topPerformers: [],
});

// ============================================================================
// FUNCTIONS
// ============================================================================

/**
 * Fetch and update reputation for a single peer
 */
export async function updatePeerReputation(peerId: string): Promise<void> {
  try {
    const summary = await reputationService.getPeerReputation(peerId);
    peerReputations.update((reps) => {
      reps.set(peerId, summary);
      return new Map(reps); // Create new map to trigger reactivity
    });
    updateAnalytics();
  } catch (error) {
    console.error(`Failed to update reputation for peer ${peerId}:`, error);
  }
}

/**
 * Fetch and update reputation for multiple peers
 */
export async function updateMultiplePeerReputations(peerIds: string[]): Promise<void> {
  const promises = peerIds.map((id) => reputationService.getPeerReputation(id));
  try {
    const summaries = await Promise.all(promises);
    peerReputations.update((reps) => {
      summaries.forEach((summary: PeerReputationSummary) => {
        reps.set(summary.peerId, summary);
      });
      return new Map(reps); // Create new map to trigger reactivity
    });
    updateAnalytics();
  } catch (error) {
    console.error('Failed to update multiple peer reputations:', error);
  }
}

/**
 * Get cached reputation for a peer (returns immediately)
 */
export function getCachedPeerReputation(peerId: string): PeerReputationSummary | undefined {
  const reps = get(peerReputations);
  return reps.get(peerId);
}

/**
 * Update analytics based on current peer reputations
 */
function updateAnalytics(): void {
  const reps = get(peerReputations);
  const allReps = Array.from(reps.values());

  if (allReps.length === 0) {
    reputationAnalytics.set({
      totalPeers: 0,
      averageScore: 0.0,
      trustLevelDistribution: {
        [TrustLevel.Trusted]: 0,
        [TrustLevel.High]: 0,
        [TrustLevel.Medium]: 0,
        [TrustLevel.Low]: 0,
        [TrustLevel.Unknown]: 0,
      },
      recentVerdicts: [],
      topPerformers: [],
    });
    return;
  }

  // Calculate analytics
  const totalPeers = allReps.length;
  const averageScore = allReps.reduce((sum, rep) => sum + rep.score, 0) / totalPeers;

  // Trust level distribution
  const trustLevelDistribution = {
    [TrustLevel.Trusted]: 0,
    [TrustLevel.High]: 0,
    [TrustLevel.Medium]: 0,
    [TrustLevel.Low]: 0,
    [TrustLevel.Unknown]: 0,
  };

  allReps.forEach((rep) => {
    trustLevelDistribution[rep.trustLevel]++;
  });

  // Top performers (sorted by score, take top 10)
  const topPerformers = [...allReps]
    .sort((a, b) => b.score - a.score)
    .slice(0, 10);

  reputationAnalytics.set({
    totalPeers,
    averageScore,
    trustLevelDistribution,
    recentVerdicts: [],  // Would need to aggregate from all peers
    topPerformers,
  });
}

/**
 * Clear all cached reputation data
 */
export function clearReputationCache(): void {
  peerReputations.set(new Map());
  reputationService.clearCache();
  updateAnalytics();
}

/**
 * Cleanup stale cache entries
 */
export function cleanupReputationCache(): void {
  reputationService.cleanupCache();
}

// Auto-cleanup every 5 minutes
if (typeof window !== 'undefined') {
  setInterval(cleanupReputationCache, 5 * 60 * 1000);
}
