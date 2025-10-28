# Mining State Persistence

## Overview

The mining state is now persistent across browser sessions and page navigation. This ensures users don't lose their mining progress if they:
- Close and reopen the browser
- Navigate to different pages within the app
- Refresh the page
- Experience an unexpected crash

## Implementation

### Storage Location
Mining state is stored in `localStorage` under the key `'miningSession'`.

### Persisted Data
The following mining state is automatically saved:
- `isMining` - Whether mining is currently active
- `hashRate` - Current hash rate (e.g., "350 KH/s")
- `totalRewards` - Total Chiral earned (number)
- `blocksFound` - Number of blocks mined (number)
- `activeThreads` - Number of CPU threads used (number)
- `minerIntensity` - Mining intensity percentage (1-100)
- `selectedPool` - Selected mining pool ("solo" or pool ID)
- `sessionStartTime` - Unix timestamp when mining started
- `recentBlocks` - Array of recently mined blocks (up to 50)
- `miningHistory` - Hash rate history for charts (up to 30 points)

### Auto-Save Behavior
Changes to the mining state are automatically saved to localStorage whenever:
- Mining starts or stops
- A block is mined
- Hash rate changes
- Configuration (threads, intensity) is updated
- Mining history is updated

### Auto-Load Behavior
When the app loads:
1. The `loadMiningState()` function checks localStorage for saved state
2. If found, it loads and parses the data (including converting date strings)
3. If not found or corrupted, it uses default initial values
4. The Mining page checks if `isMining` is true and resumes the session

### Cleanup
Mining state is cleared when:
- User logs out (Account page `handleLogout()` function)
- User explicitly clears it
- `localStorage.removeItem('miningSession')` is called

## Code Changes

### `src/lib/stores.ts`
```typescript
// Load initial mining state from localStorage
function loadMiningState(): MiningState {
  if (typeof localStorage !== 'undefined') {
    try {
      const stored = localStorage.getItem('miningSession');
      if (stored) {
        const parsed = JSON.parse(stored);
        // Convert date strings back to Date objects for recentBlocks
        if (parsed.recentBlocks) {
          parsed.recentBlocks = parsed.recentBlocks.map((block: any) => ({
            ...block,
            timestamp: new Date(block.timestamp)
          }));
        }
        return parsed;
      }
    } catch (e) {
      console.error('Failed to load mining state from localStorage:', e);
    }
  }
  return { /* default state */ };
}

export const miningState = writable<MiningState>(loadMiningState());

// Subscribe to changes and persist
if (typeof localStorage !== 'undefined') {
  miningState.subscribe(state => {
    try {
      localStorage.setItem('miningSession', JSON.stringify(state));
    } catch (e) {
      console.error('Failed to save mining state to localStorage:', e);
    }
  });
}
```

### `src/pages/Mining.svelte`
No changes needed! The page already handles restoring the session in `onMount()`:
```typescript
// If mining is already active from before, restore session and update stats
if ($miningState.isMining) {
  if ($miningState.sessionStartTime) {
    sessionStartTime = $miningState.sessionStartTime
  }
  startUptimeTimer()
  await updateMiningStats()
}
```

### `src/pages/Account.svelte`
Already handles cleanup on logout:
```typescript
// Clear mining state completely
miningState.update((state: any) => ({ /* reset values */ }));

// Clear any stored session data
localStorage.removeItem('miningSession');
```

## Testing

### Unit Tests
5 new tests added to `tests/mining.test.ts`:
- ✅ Persist mining state to localStorage when updated
- ✅ Load mining state from localStorage on initialization
- ✅ Handle corrupted localStorage data gracefully
- ✅ Persist mining session across updates
- ✅ Clear persistence when explicitly reset

All 39 mining tests pass.

### Manual Testing
To test the persistence:
1. Start the app and begin mining
2. Mine a few blocks to accumulate rewards
3. Close the browser tab completely
4. Reopen the app
5. Navigate to the Mining page
6. Verify that:
   - Mining status shows as active
   - Total rewards are preserved
   - Blocks found count is correct
   - Recent blocks list is intact
   - Session uptime resumes from stored start time
   - Mining history chart shows previous data

## Edge Cases Handled

1. **Corrupted localStorage data**: If JSON parsing fails, default state is used
2. **Missing localStorage**: Gracefully degrades to default state (for SSR/Node environments)
3. **Date serialization**: Recent block timestamps are properly converted to/from JSON
4. **Logout**: State is fully cleared and localStorage is cleaned up
5. **Multiple tabs**: Each tab will have its own view of the shared localStorage state

## Browser Compatibility

Works in all modern browsers that support localStorage:
- Chrome/Edge (Chromium)
- Firefox
- Safari
- Opera

Note: localStorage is limited to ~5-10MB per origin, which is more than sufficient for mining state.

## Future Enhancements

Potential improvements:
- [ ] Add state versioning for migration between app versions
- [ ] Compress mining history to reduce storage size
- [ ] Add option to export/import mining session
- [ ] Sync across devices via optional cloud backup
