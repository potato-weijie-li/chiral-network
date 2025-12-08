<script lang="ts">
  import Button from '$lib/components/ui/button.svelte'

  // Props
  interface Props {
    currentPage: number
    totalItems: number
    itemsPerPage: number
    onPageChange: (page: number) => void
  }

  let {
    currentPage,
    totalItems,
    itemsPerPage,
    onPageChange
  }: Props = $props()

  const totalPages = $derived(Math.ceil(totalItems / itemsPerPage))
  const startIndex = $derived((currentPage - 1) * itemsPerPage)
  const endIndex = $derived(Math.min(startIndex + itemsPerPage, totalItems))
</script>

{#if totalItems > 0}
  <div class="flex items-center justify-between gap-4">
    <!-- Left: Item counter -->
    <div class="text-sm text-muted-foreground flex-shrink-0">
      Showing {startIndex + 1}-{endIndex} of {totalItems} items
    </div>

    <!-- Center: Pagination controls -->
    <div class="flex items-center justify-center flex-1">
      {#if totalItems > itemsPerPage}
        <div class="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onclick={() => {
              if (currentPage > 1) onPageChange(currentPage - 1)
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
                  onclick={() => onPageChange(page)}
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
              if (currentPage < totalPages) onPageChange(currentPage + 1)
            }}
            disabled={currentPage === totalPages}
          >
            Next
          </Button>
        </div>
      {/if}
    </div>
  </div>
{:else}
  <div class="text-sm text-muted-foreground">
    No items
  </div>
{/if}
