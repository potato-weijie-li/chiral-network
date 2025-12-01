<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import {
    TrustLevel,
    type PeerReputationSummary,
    type TransactionVerdict,
    type ReputationAnalytics,
    VerdictOutcome,
  } from '$lib/types/reputation';
  import {
    getTrustLevelColor,
    getVerdictOutcomeColor,
  } from '$lib/types/reputation';
  import { reputationService, blacklistService } from '$lib/services/reputationService';
  import { peerReputations, reputationAnalytics, updateMultiplePeerReputations } from '$lib/reputationStore';
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import ComplaintDialog from '$lib/components/ComplaintDialog.svelte';
  import { AlertTriangle } from 'lucide-svelte';

  // State
  let isLoading = true;
  let searchQuery = '';
  let sortBy: 'score' | 'successful' | 'failed' = 'score';
  let selectedTrustLevels: TrustLevel[] = [];
  let currentPage = 1;
  const peersPerPage = 10;

  // Complaint dialog
  let showComplaintDialog = false;
  let complaintTargetPeer = '';

  // Peer list derived from store
  let peerList: PeerReputationSummary[] = [];
  $: peerList = Array.from($peerReputations.values());

  // Filtered and sorted peers
  let filteredPeers: PeerReputationSummary[] = [];
  $: {
    filteredPeers = peerList.filter((peer) => {
      // Search filter
      if (searchQuery && !peer.peerId.toLowerCase().includes(searchQuery.toLowerCase())) {
        return false;
      }
      // Trust level filter
      if (selectedTrustLevels.length > 0 && !selectedTrustLevels.includes(peer.trustLevel)) {
        return false;
      }
      return true;
    });

    // Sort
    filteredPeers.sort((a, b) => {
      if (sortBy === 'score') {
        return b.score - a.score;
      } else if (sortBy === 'successful') {
        return b.successfulTransactions - a.successfulTransactions;
      } else if (sortBy === 'failed') {
        return b.failedTransactions - a.failedTransactions;
      }
      return 0;
    });
  }

  // Paginated peers
  let paginatedPeers: PeerReputationSummary[] = [];
  $: {
    const start = (currentPage - 1) * peersPerPage;
    const end = start + peersPerPage;
    paginatedPeers = filteredPeers.slice(start, end);
  }

  $: totalPages = Math.ceil(filteredPeers.length / peersPerPage);

  // Load peer reputations
  async function loadReputations() {
    isLoading = true;
    try {
      // For demo, we'll fetch from mock data
      // In production, this would fetch from connected peers
      const mockPeerIds = ['peer1', 'peer2', 'peer3', 'peer4', 'peer5'];
      await updateMultiplePeerReputations(mockPeerIds);
    } catch (error) {
      console.error('Failed to load reputations:', error);
    } finally {
      isLoading = false;
    }
  }

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  function getTrustLevelLabel(level: TrustLevel): string {
    return level;
  }

  function toggleTrustLevelFilter(level: TrustLevel) {
    if (selectedTrustLevels.includes(level)) {
      selectedTrustLevels = selectedTrustLevels.filter((l) => l !== level);
    } else {
      selectedTrustLevels = [...selectedTrustLevels, level];
    }
  }

  function clearFilters() {
    selectedTrustLevels = [];
    searchQuery = '';
  }

  function openComplaintDialog(peerId: string) {
    complaintTargetPeer = peerId;
    showComplaintDialog = true;
  }

  function handleComplaintSubmitted() {
    // Refresh reputation data after complaint filed
    loadReputations();
  }

  onMount(() => {
    loadReputations();
    
    // Auto-refresh every 30 seconds
    const interval = setInterval(loadReputations, 30000);
    return () => clearInterval(interval);
  });
</script>

<svelte:head>
  <title>{$t('nav.reputation')} - Chiral Network</title>
</svelte:head>

<div class="container mx-auto p-4 space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <h1 class="text-3xl font-bold">{$t('nav.reputation')}</h1>
    <Button on:click={loadReputations} disabled={isLoading}>
      {$t('common.refresh')}
    </Button>
  </div>

  <!-- Analytics Overview -->
  <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
    <Card class="p-4">
      <div class="text-sm text-gray-500">{$t('reputation.analytics.totalPeers')}</div>
      <div class="text-2xl font-bold">{$reputationAnalytics.totalPeers}</div>
    </Card>
    <Card class="p-4">
      <div class="text-sm text-gray-500">{$t('reputation.analytics.averageScore')}</div>
      <div class="text-2xl font-bold">{$reputationAnalytics.averageScore.toFixed(2)}</div>
    </Card>
    <Card class="p-4">
      <div class="text-sm text-gray-500">{$t('reputation.trustLevel.trusted')}</div>
      <div class="text-2xl font-bold" style="color: {getTrustLevelColor(TrustLevel.Trusted)}">
        {$reputationAnalytics.trustLevelDistribution[TrustLevel.Trusted]}
      </div>
    </Card>
    <Card class="p-4">
      <div class="text-sm text-gray-500">{$t('reputation.trustLevel.high')}</div>
      <div class="text-2xl font-bold" style="color: {getTrustLevelColor(TrustLevel.High)}">
        {$reputationAnalytics.trustLevelDistribution[TrustLevel.High]}
      </div>
    </Card>
  </div>

  <!-- Filters and Search -->
  <Card class="p-4">
    <div class="space-y-4">
      <div class="flex flex-wrap gap-4 items-center">
        <!-- Search -->
        <input
          type="text"
          bind:value={searchQuery}
          placeholder={$t('reputation.search.placeholder')}
          class="px-4 py-2 border rounded-md flex-1 min-w-[200px]"
        />

        <!-- Sort -->
        <select
          bind:value={sortBy}
          class="px-4 py-2 border rounded-md"
        >
          <option value="score">{$t('reputation.sortBy.score')}</option>
          <option value="successful">{$t('reputation.sortBy.successful')}</option>
          <option value="failed">{$t('reputation.sortBy.failed')}</option>
        </select>

        <Button on:click={clearFilters} variant="outline">
          {$t('reputation.clearFilters')}
        </Button>
      </div>

      <!-- Trust Level Filters -->
      <div class="flex flex-wrap gap-2">
        {#each Object.values(TrustLevel) as level}
          <button
            on:click={() => toggleTrustLevelFilter(level)}
            class="px-3 py-1 rounded-full text-sm transition-colors"
            style="background-color: {selectedTrustLevels.includes(level) ? getTrustLevelColor(level) : '#e5e7eb'}; color: {selectedTrustLevels.includes(level) ? 'white' : '#374151'}"
          >
            {getTrustLevelLabel(level)}
          </button>
        {/each}
      </div>
    </div>
  </Card>

  <!-- Peer List -->
  <Card class="p-4">
    <h2 class="text-xl font-semibold mb-4">{$t('reputation.peerList.title')}</h2>

    {#if isLoading}
      <div class="text-center py-8">
        <div class="animate-spin h-8 w-8 border-4 border-blue-500 border-t-transparent rounded-full mx-auto"></div>
        <p class="mt-2 text-gray-500">{$t('common.loading')}</p>
      </div>
    {:else if paginatedPeers.length === 0}
      <div class="text-center py-8 text-gray-500">
        {$t('reputation.peerList.noPeers')}
      </div>
    {:else}
      <div class="space-y-4">
        {#each paginatedPeers as peer (peer.peerId)}
          <div class="border rounded-lg p-4 hover:shadow-md transition-shadow">
            <div class="flex justify-between items-start">
              <div class="flex-1">
                <div class="flex items-center gap-3 mb-2">
                  <span class="font-mono text-sm">{peer.peerId}</span>
                  <span
                    class="px-2 py-1 rounded text-xs font-semibold text-white"
                    style="background-color: {getTrustLevelColor(peer.trustLevel)}"
                  >
                    {getTrustLevelLabel(peer.trustLevel)}
                  </span>
                </div>

                <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <span class="text-gray-500">{$t('reputation.score')}:</span>
                    <span class="font-semibold ml-1">{peer.score.toFixed(2)}</span>
                  </div>
                  <div>
                    <span class="text-gray-500">{$t('reputation.successful')}:</span>
                    <span class="font-semibold ml-1 text-green-600">{peer.successfulTransactions}</span>
                  </div>
                  <div>
                    <span class="text-gray-500">{$t('reputation.failed')}:</span>
                    <span class="font-semibold ml-1 text-red-600">{peer.failedTransactions}</span>
                  </div>
                  <div>
                    <span class="text-gray-500">{$t('reputation.totalVerdicts')}:</span>
                    <span class="font-semibold ml-1">{peer.totalVerdicts}</span>
                  </div>
                </div>

                <div class="mt-2 text-xs text-gray-500">
                  {$t('reputation.lastUpdated')}: {formatTimestamp(peer.lastUpdated)}
                </div>
              </div>

              <div class="flex gap-2">
                <Button size="sm" variant="outline">
                  {$t('reputation.viewDetails')}
                </Button>
                <Button 
                  size="sm" 
                  variant="outline"
                  on:click={() => openComplaintDialog(peer.peerId)}
                >
                  <AlertTriangle class="h-4 w-4 mr-1" />
                  {$t('reputation.complaint.title')}
                </Button>
                <Button size="sm" variant="destructive">
                  {$t('reputation.blacklist')}
                </Button>
              </div>
            </div>
          </div>
        {/each}
      </div>

      <!-- Pagination -->
      {#if totalPages > 1}
        <div class="flex justify-center items-center gap-2 mt-6">
          <Button
            on:click={() => currentPage = Math.max(1, currentPage - 1)}
            disabled={currentPage === 1}
            size="sm"
          >
            {$t('common.previous')}
          </Button>
          <span class="text-sm">
            {$t('common.page')} {currentPage} {$t('common.of')} {totalPages}
          </span>
          <Button
            on:click={() => currentPage = Math.min(totalPages, currentPage + 1)}
            disabled={currentPage === totalPages}
            size="sm"
          >
            {$t('common.next')}
          </Button>
        </div>
      {/if}
    {/if}
  </Card>

  <!-- Trust Level Distribution -->
  <Card class="p-4">
    <h2 class="text-xl font-semibold mb-4">{$t('reputation.trustDistribution.title')}</h2>
    <div class="space-y-3">
      {#each Object.values(TrustLevel) as level}
        {@const count = $reputationAnalytics.trustLevelDistribution[level]}
        {@const percentage = $reputationAnalytics.totalPeers > 0
          ? (count / $reputationAnalytics.totalPeers) * 100
          : 0}
        <div>
          <div class="flex justify-between text-sm mb-1">
            <span>{getTrustLevelLabel(level)}</span>
            <span>{count} ({percentage.toFixed(1)}%)</span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all duration-300"
              style="width: {percentage}%; background-color: {getTrustLevelColor(level)}"
            ></div>
          </div>
        </div>
      {/each}
    </div>
  </Card>
</div>

<!-- Complaint Dialog -->
<ComplaintDialog 
  bind:isOpen={showComplaintDialog}
  bind:targetPeerId={complaintTargetPeer}
  on:submitted={handleComplaintSubmitted}
/>

