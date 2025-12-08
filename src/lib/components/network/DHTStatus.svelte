<script lang="ts">
  import Card from '$lib/components/ui/card.svelte'
  import Badge from '$lib/components/ui/badge.svelte'
  import Button from '$lib/components/ui/button.svelte'
  import Input from '$lib/components/ui/input.svelte'
  import Label from '$lib/components/ui/label.svelte'
  import { Wifi, UserPlus, Clipboard } from 'lucide-svelte'
  import { t } from 'svelte-i18n'
  import { showToast } from '$lib/toast'
  import type { DhtHealth as DhtHealthSnapshot, NatReachabilityState, NatConfidence } from '$lib/dht'
  import type { AppSettings } from '$lib/stores'

  // Props
  interface Props {
    dhtStatus: 'disconnected' | 'connecting' | 'connected'
    dhtPeerId: string | null
    dhtPort: number
    dhtBootstrapNode: string
    dhtEvents: string[]
    dhtPeerCount: number
    dhtHealth: DhtHealthSnapshot | null
    dhtError: string | null
    publicMultiaddrs: string[]
    settings: AppSettings
    lastNatState: NatReachabilityState | null
    lastNatConfidence: NatConfidence | null
    cancelConnection: boolean
    onStartDht: () => Promise<void>
    onStopDht: () => Promise<void>
    onCancelConnection: () => void
    onCopyObservedAddr: (addr: string) => Promise<void>
    onPortChange: (port: number) => void
  }

  let {
    dhtStatus,
    dhtPeerId,
    dhtPort,
    dhtBootstrapNode,
    dhtEvents,
    dhtPeerCount,
    dhtHealth,
    dhtError,
    publicMultiaddrs,
    settings,
    lastNatState,
    lastNatConfidence,
    cancelConnection,
    onStartDht,
    onStopDht,
    onCancelConnection,
    onCopyObservedAddr,
    onPortChange
  }: Props = $props()

  let copiedPeerId = $state(false)
  let copiedBootstrap = $state(false)
  let copiedListenAddr: string | null = $state(null)

  const tr = (k: string, params?: Record<string, any>): string => $t(k, params)

  async function copy(text: string | null | undefined) {
    if (!text) return
    try {
      await navigator.clipboard.writeText(text)
      copiedPeerId = true
      setTimeout(() => copiedPeerId = false, 2000)
    } catch (e) {
      console.error('Copy failed:', e)
    }
  }

  async function copyBootstrap() {
    try {
      await navigator.clipboard.writeText(dhtBootstrapNode)
      copiedBootstrap = true
      setTimeout(() => copiedBootstrap = false, 2000)
    } catch (e) {
      console.error('Copy failed:', e)
    }
  }

  async function copyListenAddr(addr: string) {
    try {
      await navigator.clipboard.writeText(addr)
      copiedListenAddr = addr
      setTimeout(() => copiedListenAddr = null, 2000)
    } catch (e) {
      console.error('Copy failed:', e)
    }
  }

  function formatReachabilityState(state?: NatReachabilityState | null): string {
    switch (state) {
      case 'public':
        return tr('network.dht.reachability.state.public')
      case 'private':
        return tr('network.dht.reachability.state.private')
      default:
        return tr('network.dht.reachability.state.unknown')
    }
  }

  function formatNatConfidence(confidence?: NatConfidence | null): string {
    switch (confidence) {
      case 'high':
        return tr('network.dht.reachability.confidence.high')
      case 'medium':
        return tr('network.dht.reachability.confidence.medium')
      default:
        return tr('network.dht.reachability.confidence.low')
    }
  }

  function reachabilityBadgeClass(state?: NatReachabilityState | null): string {
    switch (state) {
      case 'public':
        return 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-300'
      case 'private':
        return 'bg-amber-500/10 text-amber-600 dark:text-amber-300'
      default:
        return 'bg-muted text-muted-foreground'
    }
  }

  function formatHealthTimestamp(epoch: number | null): string {
    if (!epoch) return tr('network.dht.health.never')
    return new Date(epoch * 1000).toLocaleString()
  }

  function formatHealthMessage(value: string | null): string {
    return value ?? tr('network.dht.health.none')
  }
</script>

<Card class="p-6">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-lg font-semibold">{$t('network.dht.title')}</h2>
    <div class="flex items-center gap-2">
      {#if dhtStatus === 'connected'}
        <div class="h-2 w-2 bg-green-500 rounded-full animate-pulse"></div>
        <span class="text-sm text-green-600">{$t('network.status.connected')}</span>
      {:else if dhtStatus === 'connecting'}
        <div class="h-2 w-2 bg-yellow-500 rounded-full animate-pulse"></div>
        <span class="text-sm text-yellow-600">{$t('network.status.connecting')}</span>
      {:else}
        <div class="h-2 w-2 bg-red-500 rounded-full"></div>
        <span class="text-sm text-red-600">{$t('network.status.disconnected')}</span>
      {/if}
    </div>
  </div>
  
  <div class="space-y-3">
    {#if dhtStatus === 'disconnected'}
      <div class="text-center py-4">
        <Wifi class="h-12 w-12 text-muted-foreground mx-auto mb-2" />
        <p class="text-sm text-muted-foreground mb-3">{$t('network.dht.notConnected')}</p>
        <div class="px-8 my-4 text-left">
          <div class="mb-3">
            <Label for="dht-port" class="text-xs">{$t('network.dht.port')}</Label>
            <Input
              id="dht-port"
              type="number"
              value={dhtPort}
              onchange={(e: Event) => onPortChange(Number((e.target as HTMLInputElement).value))}
              class="mt-1 text-sm"
            />
          </div>
          <div>
            <Label class="text-xs">{$t('network.dht.bootstrap')}</Label>
            <div class="flex items-center gap-2 mt-1">
              <Input
                value={dhtBootstrapNode}
                readonly
                class="text-sm bg-muted"
              />
              <Button
                size="sm"
                variant="ghost"
                class="flex-shrink-0"
                onclick={() => copyBootstrap()}
              >
                {#if copiedBootstrap}
                  ✓
                {:else}
                  <Clipboard class="h-4 w-4" />
                {/if}
              </Button>
            </div>
          </div>
        </div>
        <Button onclick={() => onStartDht()}>
          {$t('network.dht.startDht')}
        </Button>
      </div>
    {:else if dhtStatus === 'connecting'}
      <div class="text-center py-4">
        <div class="animate-spin h-8 w-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-3"></div>
        <p class="text-sm text-muted-foreground mb-3">{$t('network.dht.connecting')}</p>
        <Button variant="outline" onclick={() => onCancelConnection()}>
          {$t('network.dht.cancel')}
        </Button>
      </div>
    {:else}
      <div class="space-y-3">
        <div>
          <Label class="text-xs text-muted-foreground">{$t('network.dht.peerId')}</Label>
          <div class="flex items-center gap-2 mt-1">
            <Input
              value={dhtPeerId || ''}
              readonly
              class="font-mono text-sm bg-muted"
            />
            <Button
              size="sm"
              variant="ghost"
              class="flex-shrink-0"
              onclick={() => copy(dhtPeerId)}
            >
              {#if copiedPeerId}
                ✓
              {:else}
                <Clipboard class="h-4 w-4" />
              {/if}
            </Button>
          </div>
        </div>

        <div class="grid grid-cols-2 gap-4 text-sm">
          <div>
            <p class="text-xs text-muted-foreground">{$t('network.dht.port')}</p>
            <p class="font-medium">{dhtPort}</p>
          </div>
          <div>
            <p class="text-xs text-muted-foreground">{$t('network.dht.peers')}</p>
            <p class="font-medium">{dhtPeerCount}</p>
          </div>
        </div>

        {#if publicMultiaddrs.length > 0}
          <div>
            <Label class="text-xs text-muted-foreground">{$t('network.dht.listenAddresses')}</Label>
            <div class="space-y-1 mt-1">
              {#each publicMultiaddrs as addr}
                <div class="flex items-center gap-2">
                  <Input
                    value={addr}
                    readonly
                    class="font-mono text-xs bg-muted"
                  />
                  <Button
                    size="sm"
                    variant="ghost"
                    class="flex-shrink-0"
                    onclick={() => copyListenAddr(addr)}
                  >
                    {#if copiedListenAddr === addr}
                      ✓
                    {:else}
                      <Clipboard class="h-4 w-4" />
                    {/if}
                  </Button>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        {#if dhtHealth}
          <div class="pt-3 border-t">
            <h4 class="text-sm font-semibold mb-2">{$t('network.dht.reachability.title')}</h4>
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted-foreground">{$t('network.dht.reachability.state.label')}</span>
                <Badge class={reachabilityBadgeClass(lastNatState)}>
                  {formatReachabilityState(lastNatState)}
                </Badge>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-sm text-muted-foreground">{$t('network.dht.reachability.confidence.label')}</span>
                <Badge variant="outline">{formatNatConfidence(lastNatConfidence)}</Badge>
              </div>
            </div>
          </div>
        {/if}

        {#if dhtEvents.length > 0}
          <div>
            <Label class="text-xs text-muted-foreground">{$t('network.dht.events')}</Label>
            <div class="mt-1 space-y-1 max-h-32 overflow-y-auto bg-muted/50 rounded p-2">
              {#each dhtEvents.slice(-5) as event}
                <p class="text-xs font-mono">{event}</p>
              {/each}
            </div>
          </div>
        {/if}

        {#if dhtError}
          <div class="bg-destructive/10 text-destructive rounded p-3 text-sm">
            {dhtError}
          </div>
        {/if}

        <Button variant="outline" onclick={() => onStopDht()} class="w-full">
          {$t('network.dht.stopDht')}
        </Button>
      </div>
    {/if}
  </div>
</Card>
