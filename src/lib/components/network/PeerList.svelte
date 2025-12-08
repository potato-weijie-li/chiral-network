<script lang="ts">
  import Card from '$lib/components/ui/card.svelte'
  import Badge from '$lib/components/ui/badge.svelte'
  import Button from '$lib/components/ui/button.svelte'
  import Label from '$lib/components/ui/label.svelte'
  import DropDown from '$lib/components/ui/dropDown.svelte'
  import { RefreshCw, UserMinus } from 'lucide-svelte'
  import { t } from 'svelte-i18n'
  import type { PeerInfo } from '$lib/stores'
  import { normalizeRegion, UNKNOWN_REGION_ID } from '$lib/geo'
  import type { GeoRegionConfig } from '$lib/geo'
  import { calculateRegionDistance } from '$lib/services/geolocation'

  // Props
  interface Props {
    peers: PeerInfo[]
    currentUserRegion: GeoRegionConfig
    dhtStatus: 'disconnected' | 'connecting' | 'connected'
    isTauri: boolean
    onDisconnectPeer: (peerId: string) => Promise<void>
    onRefreshPeers: () => Promise<void>
  }

  let {
    peers,
    currentUserRegion,
    dhtStatus,
    isTauri,
    onDisconnectPeer,
    onRefreshPeers
  }: Props = $props()

  const UNKNOWN_DISTANCE = 1_000_000

  let sortBy: 'reputation' | 'sharedFiles' | 'totalSize' | 'nickname' | 'location' | 'joinDate' | 'lastSeen' | 'status' = $state('reputation')
  let sortDirection: 'asc' | 'desc' = $state('desc')
  let currentPage = $state(1)
  let peersPerPage = 5

  // Reset to page 1 when sorting changes
  $effect(() => {
    if (sortBy || sortDirection) {
      currentPage = 1
    }
  })

  // Update sort direction when category changes to match the default
  $effect(() => {
    if (sortBy) {
      const defaults: Record<typeof sortBy, 'asc' | 'desc'> = {
        reputation: 'desc',
        sharedFiles: 'desc',
        totalSize: 'desc',
        joinDate: 'desc',
        lastSeen: 'desc',
        location: 'asc',
        status: 'asc',
        nickname: 'asc'
      }
      sortDirection = defaults[sortBy]
    }
  })

  const sortedPeers = $derived.by(() => {
    return [...peers].sort((a, b) => {
      let aVal: any, bVal: any

      switch (sortBy) {
        case 'reputation':
          aVal = a.reputation
          bVal = b.reputation
          break
        case 'sharedFiles':
          aVal = a.sharedFiles
          bVal = b.sharedFiles
          break
        case 'totalSize':
          aVal = a.totalSize
          bVal = b.totalSize
          break
        case 'nickname':
          aVal = (a.nickname || 'zzzzz').toLowerCase()
          bVal = (b.nickname || 'zzzzz').toLowerCase()
          break
        case 'location':
          const getLocationDistance = (peerLocation: string | undefined) => {
            if (!peerLocation) return UNKNOWN_DISTANCE
            const peerRegion = normalizeRegion(peerLocation)
            if (peerRegion.id === UNKNOWN_REGION_ID) return UNKNOWN_DISTANCE
            if (currentUserRegion.id === UNKNOWN_REGION_ID) {
              return peerRegion.id === UNKNOWN_REGION_ID ? 0 : UNKNOWN_DISTANCE
            }
            if (peerRegion.id === currentUserRegion.id) return 0
            return Math.round(calculateRegionDistance(currentUserRegion, peerRegion))
          }
          aVal = getLocationDistance(a.location)
          bVal = getLocationDistance(b.location)
          break
        case 'joinDate':
          aVal = new Date(a.joinDate).getTime()
          bVal = new Date(b.joinDate).getTime()
          break
        case 'lastSeen':
          aVal = new Date(a.lastSeen).getTime()
          bVal = new Date(b.lastSeen).getTime()
          break
        case 'status':
          aVal = a.status === 'online' ? 0 : a.status === 'away' ? 1 : 2
          bVal = b.status === 'online' ? 0 : b.status === 'away' ? 1 : 2
          break
        default:
          return 0
      }

      if (typeof aVal === 'string' && typeof bVal === 'string') {
        if (aVal < bVal) return sortDirection === 'asc' ? -1 : 1
        if (aVal > bVal) return sortDirection === 'asc' ? 1 : -1
        return 0
      }

      if (typeof aVal === 'number' && typeof bVal === 'number') {
        const result = aVal - bVal
        return sortDirection === 'asc' ? result : -result
      }

      return 0
    })
  })

  const totalPages = $derived(Math.ceil(sortedPeers.length / peersPerPage))
  const startIndex = $derived((currentPage - 1) * peersPerPage)
  const endIndex = $derived(Math.min(startIndex + peersPerPage, sortedPeers.length))
  const paginatedPeers = $derived(sortedPeers.slice(startIndex, endIndex))

  function formatSize(bytes: number | undefined): string {
    if (bytes === undefined || bytes === null || isNaN(bytes)) {
      return '0 B'
    }

    const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB']
    let size = bytes
    let unitIndex = 0

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024
      unitIndex++
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`
  }

  function formatPeerDate(date: Date | string | number | null | undefined): string {
    if (!date) {
      return $t('network.connectedPeers.unknown')
    }
    try {
      const d = new Date(date)
      if (isNaN(d.getTime())) return $t('network.connectedPeers.unknown')
      
      const showYear = d.getFullYear() !== new Date().getFullYear()
      
      return d.toLocaleString(undefined, {
        month: 'short',
        day: 'numeric',
        year: showYear ? 'numeric' : undefined,
        hour: 'numeric',
        minute: '2-digit'
      })
    } catch (e) {
      return $t('network.connectedPeers.unknown')
    }
  }
</script>

<Card class="p-6">
  <div class="flex flex-wrap items-center justify-between gap-2 mb-4">
    <h2 class="text-lg font-semibold">{$t('network.connectedPeers.title', { values: { count: peers.length } })}</h2>
    <div class="flex items-center gap-2">
      <Label for="sort" class="flex items-center">
        <span class="text-base">{$t('network.connectedPeers.sortBy')}</span>
      </Label>
      <div class="w-40 flex-shrink-0">
        <DropDown
          id="sort"
          options={[
            { value: 'reputation', label: $t('network.connectedPeers.reputation') },
            { value: 'sharedFiles', label: $t('network.connectedPeers.sharedFiles') },
            { value: 'totalSize', label: $t('network.connectedPeers.totalSize') },
            { value: 'nickname', label: $t('network.connectedPeers.name') },
            { value: 'location', label: $t('network.connectedPeers.location') },
            { value: 'joinDate', label: $t('network.connectedPeers.joinDate') },
            { value: 'lastSeen', label: $t('network.connectedPeers.lastSeen') },
            { value: 'status', label: $t('network.connectedPeers.status') }
          ]}
          bind:value={sortBy}
        />
      </div>
      <div class="w-40 flex-shrink-0">
        <DropDown
          id="sort-direction"
          options={
            sortBy === 'reputation'
            ? [ { value: 'desc', label: $t('network.connectedPeers.highest') }, { value: 'asc', label: $t('network.connectedPeers.lowest') } ]
            : sortBy === 'sharedFiles'
            ? [ { value: 'desc', label: $t('network.connectedPeers.most') }, { value: 'asc', label: $t('network.connectedPeers.least') } ]
            : sortBy === 'totalSize'
            ? [ { value: 'desc', label: $t('network.connectedPeers.largest') }, { value: 'asc', label: $t('network.connectedPeers.smallest') } ]
            : sortBy === 'joinDate'
            ? [ { value: 'desc', label: $t('network.connectedPeers.newest') }, { value: 'asc', label: $t('network.connectedPeers.oldest') } ]
            : sortBy === 'lastSeen'
            ? [ { value: 'desc', label: $t('network.connectedPeers.mostRecent') }, { value: 'asc', label: $t('network.connectedPeers.leastRecent') } ]
            : sortBy === 'location'
            ? [ { value: 'asc', label: $t('network.connectedPeers.closest') }, { value: 'desc', label: $t('network.connectedPeers.farthest') } ]
            : sortBy === 'status'
            ? [ { value: 'asc', label: $t('network.connectedPeers.online') }, { value: 'desc', label: $t('network.connectedPeers.offline') } ]
            : sortBy === 'nickname'
            ? [ { value: 'asc', label: $t('network.connectedPeers.aToZ') }, { value: 'desc', label: $t('network.connectedPeers.zToA') } ]
            : [ { value: 'desc', label: 'Desc' }, { value: 'asc', label: 'Asc' } ]
          }
          bind:value={sortDirection}
        />
      </div>
    </div>
  </div>

  <!-- Controls bar -->
  <div class="flex items-center justify-between mb-4 gap-4">
    <div class="text-sm text-muted-foreground flex-shrink-0">
      {#if sortedPeers.length > 0}
        Showing {startIndex + 1}-{endIndex} of {sortedPeers.length} peers
      {:else}
        No peers
      {/if}
    </div>

    <div class="flex items-center justify-center flex-1">
      {#if sortedPeers.length > peersPerPage}
        <div class="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onclick={() => {
              if (currentPage > 1) currentPage--
            }}
            disabled={currentPage === 1}
          >
            Previous
          </Button>
          <div class="flex items-center gap-1">
            {#each Array.from({ length: totalPages }, (_, i) => i + 1) as page}
              {#if page === 1 || page === totalPages || (page >= currentPage - 1 && page <= currentPage + 1)}
                <Button
                  size="sm"
                  variant={page === currentPage ? 'default' : 'outline'}
                  class="w-10"
                  onclick={() => currentPage = page}
                >
                  {page}
                </Button>
              {:else if page === currentPage - 2 || page === currentPage + 2}
                <span class="px-2 text-muted-foreground">...</span>
              {/if}
            {/each}
          </div>
          <Button
            size="sm"
            variant="outline"
            onclick={() => {
              if (currentPage < totalPages) currentPage++
            }}
            disabled={currentPage === totalPages}
          >
            Next
          </Button>
        </div>
      {/if}
    </div>

    <div class="flex-shrink-0">
      <Button
        size="sm"
        variant="outline"
        onclick={onRefreshPeers}
        disabled={!isTauri || dhtStatus !== 'connected'}
      >
        <RefreshCw class="h-4 w-4 mr-2" />
        Refresh Peers
      </Button>
    </div>
  </div>

  <!-- Peer list -->
  <div class="space-y-3">
    {#each paginatedPeers as peer}
      <div class="p-4 bg-secondary rounded-lg">
        <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between mb-2 gap-2">
          <div class="flex items-start gap-3 min-w-0">
            <div class="w-2 h-2 rounded-full flex-shrink-0 {
              peer.status === 'online' ? 'bg-green-500' :
              peer.status === 'away' ? 'bg-yellow-500' :
              'bg-red-500'
            }"></div>
            <div>
              <p class="font-medium">{peer.nickname || $t('network.connectedPeers.anonymous')}</p>
              <p class="text-xs text-muted-foreground break-all">{peer.address}</p>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-2 justify-end">
            <Badge variant="outline" class="flex-shrink-0">
              ⭐ {(peer.reputation ?? 0).toFixed(1)}
            </Badge>
            <Badge variant={peer.status === 'online' ? 'default' : 'secondary'}
                   class={
                      peer.status === 'online' ? 'bg-green-500 text-white' :
                      peer.status === 'away' ? 'bg-yellow-500 text-white' :
                      'bg-red-500 text-white'
                    }
                    style="pointer-events: none;"
            >
              {peer.status}
            </Badge>
            <Button
              size="sm"
              variant="outline"
              class="h-8 px-2"
              onclick={() => onDisconnectPeer(peer.address)}
            >
              <UserMinus class="h-3.5 w-3.5 mr-1" />
              {$t('network.connectedPeers.disconnect')}
            </Button>
          </div>
        </div>
        
        <div class="grid grid-cols-2 md:grid-cols-5 gap-4 text-sm">
          <div>
            <p class="text-xs text-muted-foreground">{$t('network.connectedPeers.sharedFiles')}</p>
            <p class="font-medium">{peer.sharedFiles}</p>
          </div>
          <div>
            <p class="text-xs text-muted-foreground">{$t('network.connectedPeers.totalSize')}</p>
            <p class="font-medium">{formatSize(peer.totalSize)}</p>
          </div>
          <div>
            <p class="text-xs text-muted-foreground">{$t('network.connectedPeers.location')}</p>
            <p class="font-medium">{peer.location || $t('network.connectedPeers.unknown')}</p>
          </div>
          <div>
            <p class="text-xs text-muted-foreground">{$t('network.connectedPeers.joined')}</p>
            <p class="font-medium">{formatPeerDate(peer.joinDate)}</p>
          </div>
          <div>
            <p class="text-xs text-muted-foreground">{$t('network.connectedPeers.lastSeen')}</p>
            <p class="font-medium">
              {#if peer.status === 'online'}
                {$t('network.connectedPeers.now')}
              {:else}
                {formatPeerDate(peer.lastSeen)}
              {/if}
            </p>
          </div>
        </div>
      </div>
    {/each}

    {#if peers.length === 0}
      <p class="text-center text-muted-foreground py-8">{$t('network.connectedPeers.noPeers')}</p>
    {/if}
  </div>
</Card>
