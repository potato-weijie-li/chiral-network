<script lang="ts">
  import Card from '$lib/components/ui/card.svelte'
  import Button from '$lib/components/ui/button.svelte'
  import { Download, Play, Square, Server, Clipboard, AlertCircle } from 'lucide-svelte'
  import { t } from 'svelte-i18n'

  // Props
  interface Props {
    isGethInstalled: boolean
    isGethRunning: boolean
    isStartingNode: boolean
    isDownloading: boolean
    isCheckingGeth: boolean
    downloadProgress: {
      downloaded: number
      total: number
      percentage: number
      status: string
    }
    downloadError: string
    peerCount: number
    chainId: number | null
    nodeAddress: string
    onDownloadGeth: () => Promise<void>
    onStartGethNode: () => Promise<void>
    onStopGethNode: () => Promise<void>
  }

  let {
    isGethInstalled,
    isGethRunning,
    isStartingNode,
    isDownloading,
    isCheckingGeth,
    downloadProgress,
    downloadError,
    peerCount,
    chainId,
    nodeAddress,
    onDownloadGeth,
    onStartGethNode,
    onStopGethNode
  }: Props = $props()

  let copiedNodeAddr = $state(false)

  async function copyNodeAddress() {
    try {
      await navigator.clipboard.writeText(nodeAddress)
      copiedNodeAddr = true
      setTimeout(() => copiedNodeAddr = false, 2000)
    } catch (e) {
      console.error('Copy failed:', e)
    }
  }
</script>

<Card class="p-6">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-lg font-semibold">{$t('network.nodeStatus')}</h2>
    <div class="flex items-center gap-2">
      {#if !isGethInstalled}
        <div class="h-2 w-2 bg-yellow-500 rounded-full"></div>
        <span class="text-sm text-yellow-600">{$t('network.status.notInstalled')}</span>
      {:else if isGethRunning}
        <div class="h-2 w-2 bg-green-500 rounded-full animate-pulse"></div>
        <span class="text-sm text-green-600">{$t('network.status.connected')}</span>
      {:else}
        <div class="h-2 w-2 bg-red-500 rounded-full"></div>
        <span class="text-sm text-red-600">{$t('network.status.disconnected')}</span>
      {/if}
    </div>
  </div>

  <div class="space-y-3">
    {#if !isGethInstalled && !isGethRunning}
      <div class="text-center py-4">
        <Server class="h-12 w-12 text-muted-foreground mx-auto mb-2" />
        <p class="text-sm text-muted-foreground mb-3">{$t('network.notInstalled')}</p>
        
        {#if isDownloading}
          <div class="space-y-2">
            <div class="w-full bg-secondary rounded-full h-2">
              <div 
                class="bg-primary h-2 rounded-full transition-all duration-300"
                style="width: {downloadProgress.percentage}%"
              ></div>
            </div>
            <p class="text-xs text-muted-foreground">
              {downloadProgress.status}
              {#if downloadProgress.total > 0}
                ({downloadProgress.downloaded} / {downloadProgress.total} MB)
              {/if}
            </p>
          </div>
        {:else if downloadError}
          <div class="flex items-start gap-2 p-3 bg-destructive/10 text-destructive rounded text-sm mb-3">
            <AlertCircle class="h-4 w-4 mt-0.5 flex-shrink-0" />
            <p class="text-left">{downloadError}</p>
          </div>
        {/if}
        
        <Button 
          onclick={onDownloadGeth}
          disabled={isDownloading || isCheckingGeth}
        >
          <Download class="h-4 w-4 mr-2" />
          {isCheckingGeth ? $t('network.checking') : $t('network.download')}
        </Button>
      </div>
    {:else if isGethRunning}
      <div class="grid grid-cols-2 gap-4">
        <div>
          <p class="text-xs text-muted-foreground">{$t('network.peers')}</p>
          <p class="text-2xl font-bold">{peerCount}</p>
        </div>
        <div>
          <p class="text-xs text-muted-foreground">{$t('network.chainId')}</p>
          <p class="text-2xl font-bold">{chainId ?? '...'}</p>
        </div>
      </div>
      <div class="pt-2">
        {#if nodeAddress}
          <p class="text-xs text-muted-foreground mb-1">{$t('network.nodeAddress')}</p>
          <div class="flex items-center gap-2">
            <input
              type="text"
              value={nodeAddress}
              readonly
              class="flex-1 px-3 py-2 text-sm bg-muted rounded border font-mono"
            />
            <Button
              size="sm"
              variant="ghost"
              onclick={copyNodeAddress}
            >
              {#if copiedNodeAddr}
                ✓
              {:else}
                <Clipboard class="h-4 w-4" />
              {/if}
            </Button>
          </div>
        {/if}
      </div>
      <Button class="w-full mt-4" variant="outline" onclick={onStopGethNode}>
        <Square class="h-4 w-4 mr-2" />
        {$t('network.stopNode')}
      </Button>
    {:else}
      <div class="text-center py-8">
        <Server class="h-12 w-12 text-muted-foreground mx-auto mb-2" />
        <p class="text-sm text-muted-foreground mb-4">{$t('network.nodeReady')}</p>
        <Button 
          onclick={onStartGethNode}
          disabled={isStartingNode}
        >
          <Play class="h-4 w-4 mr-2" />
          {isStartingNode ? $t('network.starting') : $t('network.startNode')}
        </Button>
      </div>
    {/if}
  </div>
</Card>
