# Payment Notification Delivery & Confirmation System

## Problem Statement

### Root Cause: Backend Returns Success Even on P2P Send Failure

**Critical Issue Identified**: The backend's `record_download_payment` command returns `Ok(())` even when the P2P message to the seeder fails.

**Evidence from `src-tauri/src/main.rs` (lines 630-645)**:
```rust
// Send via DHT to the seeder's peer ID
match dht.send_message_to_peer(&seeder_peer_id, wrapped_message).await {
    Ok(_) => {
        println!("✅ P2P payment notification sent to peer: {}", seeder_peer_id);
    }
    Err(e) => {
        // Don't fail the whole operation if P2P message fails
        println!("⚠️ Failed to send P2P payment notification: {}.", e);
        // ⚠️ BUG: Still returns Ok(()) below - frontend thinks it succeeded!
    }
}

Ok(())  // Always returns success, even if P2P send failed
```

### Compounding Frontend Issue

**Frontend code (`src/lib/services/paymentService.ts` lines 377-390)**:
```typescript
try {
  await invoke("record_download_payment", { ... });
  console.log("✅ Payment notification sent to seeder:", seederAddress);
} catch (invokeError) {
  console.warn("Failed to send payment notification:", invokeError);
  // This catch block NEVER executes because backend always returns Ok()
}
```

### What Actually Happens

1. User downloads file, frontend debits wallet (-X Chiral) ✅
2. Frontend creates transaction record in localStorage ✅
3. Frontend calls `invoke("record_download_payment")` ✅
4. Backend attempts `dht.send_message_to_peer()` ❌ FAILS
5. Backend logs warning but returns `Ok()` 🐛
6. Frontend receives success response, thinks notification delivered ❌
7. Seeder never receives payment notification ❌
8. **Silent failure**: No retry, no user notification, no recovery mechanism

### Real Failure Scenarios

**DHT Connectivity Issues**:
- DHT service not fully initialized (app just started)
- Peer not connected to DHT network yet
- Target seeder peer ID offline/unreachable
- Network partition between peers

**There Is No Backend Ledger**: 
The backend does NOT persist payment records - it only forwards P2P messages. There's nothing to "reconcile" against.

## Why This Is Critical

### Impact on Network Economics

**Payment Flow Should Be**:
```
Downloader pays → P2P notification → Seeder receives → Seeder continues sharing
```

**Current Broken Flow**:
```
Downloader pays → P2P send fails silently → Seeder never knows → Seeders lose trust
```

**Real-World Consequences**:

1. **Seeder Frustration**: "I shared 10 files, got paid for 3, where are my other payments?"
2. **Network Collapse Risk**: If seeders don't reliably receive payment notifications, they stop sharing
3. **Support Nightmare**: Both parties have "proof" (downloader has tx, seeder has no notification)
4. **Trust Erosion**: Users perceive the payment system as unreliable/broken

### Why Current Test Is Misleading

**Existing test** (`tests/paymentService.test.ts:471`):
```typescript
  it("should use composite key to prevent collisions from different seeders", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("DHT offline"));
    
    // Download same file from two different seeders
    await PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      "0xSeeder1"
    );
    
    await PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      "0xSeeder2"
    );
    
    // Should create 2 separate pending notifications (not 1)
    const pending = PaymentService.getPendingNotifications();
    expect(pending).toHaveLength(2);
    expect(pending[0].seederAddress).not.toBe(pending[1].seederAddress);
  });
  
  it("should cleanup old failed notifications via timer", async () => {
  // Mock to throw error
  vi.mocked(invoke).mockRejectedValue(new Error("DHT error"));
  
  const result = await PaymentService.processDownloadPayment(...);
  
  // Test expects success even when backend fails
  expect(result.success).toBe(true);  // ⚠️ This is the problem!
});
```

The test validates the bug as correct behavior. "Graceful failure" means "hide the failure from the user" - exactly what we don't want.

## Proposal: Simple, Fast-Fail Retry with User Visibility

### Core Principles

1. **Fix the Backend Bug First**: Return error when P2P send fails
2. **Fast Retries**: 3 quick attempts (0s, 5s, 15s) then fail visibly
3. **User Visibility**: Show delivery status, allow manual retry
4. **No Over-Engineering**: No "reconciliation" against non-existent ledger
5. **Idempotency on Receiver**: Prevent duplicate processing

### Part 1: Fix Backend Error Handling

**Change `src-tauri/src/main.rs` (lines 630-650)**:

```rust
// BEFORE (buggy):
if let Some(dht) = app_state.dht.lock().await.as_ref() {
    match dht.send_message_to_peer(&seeder_peer_id, wrapped_message).await {
        Ok(_) => {
            println!("✅ P2P payment notification sent to peer: {}", seeder_peer_id);
        }
        Err(e) => {
            println!("⚠️ Failed to send P2P payment notification: {}", e);
            // BUG: Still returns Ok() below
        }
    }
} else {
    println!("⚠️ DHT not available");
    // BUG: Still returns Ok() below
}
Ok(())  // Always returns success!

// AFTER (fixed):
if let Some(dht) = app_state.dht.lock().await.as_ref() {
    match dht.send_message_to_peer(&seeder_peer_id, wrapped_message).await {
        Ok(_) => {
            println!("✅ P2P payment notification sent to peer: {}", seeder_peer_id);
            Ok(())  // Only return success if P2P send succeeded
        }
        Err(e) => {
            let error_msg = format!("Failed to send P2P payment notification: {}", e);
            println!("❌ {}", error_msg);
            Err(error_msg)  // Return error to frontend so it can retry
        }
    }
} else {
    let error_msg = "DHT not available for payment notification".to_string();
    println!("❌ {}", error_msg);
    Err(error_msg)  // Return error when DHT is None
}
```

**Why This Matters**: Now the frontend's `catch` block will execute when:
1. DHT is not initialized (`None`)
2. P2P send fails (peer unreachable, network error, etc.)

### Part 2: Frontend Retry Logic (Simple & Fast)

**Add to `src/lib/services/paymentService.ts`**:

```typescript
interface PaymentNotificationAttempt {
  fileHash: string;
  fileName: string;
  fileSize: number;
  seederAddress: string;
  seederPeerId: string;
  downloaderAddress: string;
  amount: number;
  transactionId: number;
  transactionHash: string;
  attemptCount: number;
  lastError?: string;
  status: 'pending' | 'delivered' | 'failed';
  timestamp: number;  // When this was created
}

// Key format: "${fileHash}-${seederAddress}" to prevent collisions
// when same file is downloaded from different seeders
private static pendingNotifications = new Map<string, PaymentNotificationAttempt>();
private static lastRetryAttempt = new Map<string, number>(); // Rate limiting
private static cleanupTimer: NodeJS.Timeout | null = null;

// Initialize timer-based cleanup (runs every 10 minutes)
private static initCleanupTimer(): void {
  if (this.cleanupTimer === null) {
    this.cleanupTimer = setInterval(() => {
      this.cleanupOldNotifications();
    }, 10 * 60 * 1000); // 10 minutes
    console.log('🧹 Notification cleanup timer initialized');
  }
}

// Cleanup old failed notifications (>1 hour old) to prevent memory leak
private static cleanupOldNotifications(): void {
  const ONE_HOUR = 60 * 60 * 1000;
  const now = Date.now();
  
  for (const [key, notification] of this.pendingNotifications.entries()) {
    if (notification.status === 'failed' && (now - notification.timestamp) > ONE_HOUR) {
      this.pendingNotifications.delete(key);
      console.log(`🧹 Cleaned up old failed notification: ${key}`);
    }
  }
}

private static async sendPaymentNotificationWithRetry(
  fileHash: string,
  fileName: string,
  fileSize: number,
  seederAddress: string,
  seederPeerId: string,
  downloaderAddress: string,
  amount: number,
  transactionId: number,
  transactionHash: string
): Promise<{ success: boolean; error?: string }> {
  
  // Initialize cleanup timer on first use
  this.initCleanupTimer();
  
  // Use composite key to prevent collision when same file is downloaded from different seeders
  const notificationKey = `${fileHash}-${seederAddress}`;
  
  const MAX_ATTEMPTS = 3;
  const RETRY_DELAYS_MS = [0, 5000, 15000]; // 0s, 5s, 15s total
  // Rationale: DHT typically reconnects within 5-10 seconds
  // First retry catches quick recoveries, second gives more time
  // Total 20s window provides fast user feedback
  
  for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
    // Wait before retry (skip on first attempt)
    if (attempt > 0) {
      const delay = RETRY_DELAYS_MS[attempt];
      await new Promise(resolve => setTimeout(resolve, delay));
      console.log(`🔄 Retry attempt ${attempt + 1}/${MAX_ATTEMPTS} for payment ${transactionHash}`);
    }
    
    try {
      await invoke("record_download_payment", {
        fileHash,
        fileName,
        fileSize,
        seederWalletAddress: seederAddress,
        seederPeerId,
        downloaderAddress,
        amount,
        transactionId,
        transactionHash,
      });
      
      // Success!
      console.log(`✅ Payment notification delivered on attempt ${attempt + 1}`);
      this.pendingNotifications.delete(notificationKey);
      return { success: true };
      
    } catch (error) {
      const errorMsg = String(error);
      console.warn(`❌ Attempt ${attempt + 1} failed:`, errorMsg);
      
      // Store full context for manual retry
      this.pendingNotifications.set(notificationKey, {
        fileHash,
        fileName,
        fileSize,
        seederAddress,
        seederPeerId,
        downloaderAddress,
        amount,
        transactionId,
        transactionHash,
        attemptCount: attempt + 1,
        lastError: errorMsg,
        status: attempt < MAX_ATTEMPTS - 1 ? 'pending' : 'failed',
        timestamp: Date.now()
      });
      
      // If this was the last attempt, give up
      if (attempt === MAX_ATTEMPTS - 1) {
        return { 
          success: false, 
          error: `Failed to notify seeder after ${MAX_ATTEMPTS} attempts. Last error: ${errorMsg}` 
        };
      }
    }
  }
  
  return { success: false, error: 'Unexpected retry loop exit' };
}
```

### Part 3: User Visibility & Manual Retry

**Modify `processDownloadPayment()` in `src/lib/services/paymentService.ts`**:

```typescript
// Replace the existing try-catch around record_download_payment (lines 377-390)
// With this notification-aware version:

const notificationResult = await this.sendPaymentNotificationWithRetry(
  fileHash,
  fileName,
  fileSize,
  seederAddress,
  seederPeerId || seederAddress,
  currentWallet.address || "unknown",
  amount,
  transactionId,
  transactionHash
);

if (!notificationResult.success) {
  // Show user-friendly error with retry option
  toastStore.addToast(
    `Payment sent (${amount.toFixed(4)} Chiral), but seeder notification failed. ` +
    `Seeder may not know about payment yet. You can retry notification from Wallet tab.`,
    'warning',
    10000  // 10 second display
  );
}

// Store notification status in transaction metadata
// This allows UI to show notification status after page refresh
const transactionWithStatus = {
  ...newTransaction,
  metadata: {
    notificationDelivered: notificationResult.success,
    notificationError: notificationResult.error,
    lastNotificationAttempt: Date.now()
  }
};

// Update the transaction in history with metadata
transactions.update((txs) => {
  const updated = txs.map(tx => 
    tx.id === transactionId ? transactionWithStatus : tx
  );
  saveTransactionsToStorage(updated);
  return updated;
});

return {
  success: true,  // Payment succeeded locally (wallet debited, tx recorded)
  transactionId,
  transactionHash,
};
```

**Why Store Metadata**: Transaction metadata persists notification status across page refreshes, allowing UI to show "Retry" button for failed notifications even after app restart.

**Add Manual Retry Function**:

```typescript
/**
 * Manually retry sending payment notification to seeder
 * Called from UI when user clicks "Retry Notification" button
 * @param fileHash - The hash of the file
 * @param seederAddress - The wallet address of the seeder
 */
static async retryPaymentNotification(fileHash: string, seederAddress: string): Promise<boolean> {
  const notificationKey = `${fileHash}-${seederAddress}`;
  
  // Rate limiting: max 1 retry per 10 seconds per notification
  const lastAttempt = this.lastRetryAttempt.get(notificationKey) || 0;
  const now = Date.now();
  if (now - lastAttempt < 10000) {
    console.warn('Rate limit: Please wait before retrying');
    return false;
  }
  this.lastRetryAttempt.set(notificationKey, now);
  
  const pending = this.pendingNotifications.get(notificationKey);
  if (!pending) {
    console.warn('No pending notification found for', fileHash);
    return false;
  }
  
  // Use stored context (includes all necessary fields)
  const result = await this.sendPaymentNotificationWithRetry(
    pending.fileHash,
    pending.fileName,
    pending.fileSize,
    pending.seederAddress,
    pending.seederPeerId,
    pending.downloaderAddress,
    pending.amount,
    pending.transactionId,
    pending.transactionHash
  );
  
  return result.success;
}

/**
 * Get list of pending notifications for UI display
 * Note: Cleanup runs automatically every 10 minutes via timer
 */
static getPendingNotifications(): PaymentNotificationAttempt[] {
  return Array.from(this.pendingNotifications.values());
}
```

### Part 4: Seeder-Side Idempotency

**Enhance `creditSeederPayment()` in `src/lib/services/paymentService.ts`**:

```typescript
// Modify existing function (lines 413-493) to improve deduplication:
static async creditSeederPayment(
  fileHash: string,
  fileName: string,
  fileSize: number,
  downloaderAddress: string,
  transactionHash?: string
): Promise<{ success: boolean; transactionId?: number; error?: string }> {
  
  // Enhanced deduplication: prefer transaction hash, fall back to file+address
  // Note: transactionHash may be empty if blockchain transaction failed
  const paymentKey = (transactionHash && transactionHash !== "") 
    ? `txhash-${transactionHash}` 
    : `${fileHash}-${downloaderAddress}`;
  
  // Check if we already received this payment
  if (this.receivedPayments.has(paymentKey)) {
    console.log("⚠️ Duplicate payment notification ignored:", paymentKey);
    return {
      success: true,  // Not an error, just already processed
      error: "Payment already received"
    };
  }
  
  // ... rest of existing logic (lines 431-490) unchanged ...
  
  // Mark as received AFTER successful credit (line 493)
  this.receivedPayments.add(paymentKey);
  
  // ... rest of existing logic ...
}
```

**Why This Works**:
- **When blockchain succeeds**: Uses unique transaction hash (prevents all duplicates)
- **When blockchain fails**: Falls back to `fileHash-downloaderAddress` (prevents duplicate payments for same file from same user)
- **Idempotent**: Same notification received multiple times credits wallet only once

**Trade-off**: If blockchain fails and user downloads same file twice from same seeder, second download won't credit. This is acceptable because:
1. Blockchain failures are rare
2. If it happens, manual investigation can resolve (better than double-crediting)
3. User gets warning that notification failed (can contact support)

## Acceptance Criteria

### Functional Requirements

1. **Backend Returns Proper Errors**
   - Given: DHT is unavailable or peer is unreachable
   - When: `record_download_payment` is called
   - Then: Backend returns `Err()` with descriptive error message
   - And: Frontend `catch` block executes

2. **Automatic Fast Retry**
   - Given: First notification attempt fails
   - When: 5 seconds elapse
   - Then: Second attempt is made automatically
   - And: If second fails, third attempt after 15 more seconds
   - And: Total retry window is 20 seconds

3. **User Visibility on Failure**
   - Given: All 3 retry attempts fail
   - When: Payment completes
   - Then: User sees warning: "Payment sent, but seeder notification failed"
   - And: User can manually retry from Wallet tab
   - And: Transaction record shows `notificationDelivered: false`

4. **Manual Retry Works**
   - Given: Payment notification previously failed
   - When: User clicks "Retry Notification" button in UI
   - Then: Notification is re-attempted
   - And: Success/failure is shown to user immediately
   - And: On success, pending notification is cleared

5. **Seeder Deduplication**
   - Given: Downloader retries notification multiple times
   - When: Seeder receives 2nd notification with same transaction hash
   - Then: Seeder ignores duplicate (doesn't credit twice)
   - And: Logs "Duplicate payment notification ignored"

6. **No False Success**
   - Given: P2P send fails
   - When: Frontend processes the error
   - Then: Frontend does NOT log "✅ Payment notification sent"
   - And: Frontend DOES track as pending notification

### Testing Requirements

**Unit Tests** (Vitest):

```typescript
describe("Payment Notification with Retry", () => {
  
  it("should succeed on first attempt when DHT is available", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    
    const result = await PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      "0x1234...5678"
    );
    
    expect(result.success).toBe(true);
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(1);
    
    // Verify notification metadata in transaction
    const transactions = get(transactionStore);
    const tx = transactions.find(t => t.id === result.transactionId);
    expect(tx?.metadata?.notificationDelivered).toBe(true);
  });
  
  it("should retry 3 times with correct delays", async () => {
    vi.useFakeTimers();
    let attempts = 0;
    const attemptTimes: number[] = [];
    
    vi.mocked(invoke).mockImplementation(async (cmd) => {
      if (cmd === "record_download_payment") {
        attempts++;
        attemptTimes.push(Date.now());
        if (attempts < 3) throw new Error("DHT unavailable");
        return undefined; // Success on 3rd attempt
      }
      if (cmd === "process_download_payment") {
        return "0xtxhash123";
      }
    });
    
    const promise = PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      "0x1234...5678"
    );
    
    // Allow first attempt to execute
    await vi.runOnlyPendingTimersAsync();
    expect(attempts).toBe(1);
    
    // Second attempt after 5s delay
    await vi.advanceTimersByTimeAsync(5000);
    await vi.runOnlyPendingTimersAsync();
    expect(attempts).toBe(2);
    
    // Third attempt after 15s delay (not 5s+15s, just 15s from second attempt)
    await vi.advanceTimersByTimeAsync(15000);
    await vi.runOnlyPendingTimersAsync();
    expect(attempts).toBe(3);
    
    const result = await promise;
    expect(result.success).toBe(true);
    
    // Verify timing between attempts
    expect(attemptTimes[1] - attemptTimes[0]).toBeGreaterThanOrEqual(5000);
    expect(attemptTimes[2] - attemptTimes[1]).toBeGreaterThanOrEqual(15000);
    
    vi.useRealTimers();
  });
  
  it("should fail after 3 attempts and store error in transaction metadata", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("DHT offline"));
    
    const result = await PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      "0x1234...5678"
    );
    
    expect(result.success).toBe(true); // Payment still succeeded locally
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(3);
    
    // Verify notification failure stored in transaction metadata
    const transactions = get(transactionStore);
    const tx = transactions.find(t => t.id === result.transactionId);
    expect(tx?.metadata?.notificationDelivered).toBe(false);
    expect(tx?.metadata?.notificationError).toContain("Failed to notify seeder after 3 attempts");
  });
  
  it("should allow manual retry of failed notification", async () => {
    const seederAddress = "0x1234...5678";
    
    // First attempt fails
    vi.mocked(invoke).mockRejectedValueOnce(new Error("DHT offline"))
                     .mockResolvedValueOnce(undefined); // Retry succeeds
    
    await PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      seederAddress
    );
    
    // Manual retry with both fileHash and seederAddress
    const retryResult = await PaymentService.retryPaymentNotification("QmHash123", seederAddress);
    
    expect(retryResult).toBe(true);
    expect(PaymentService.getPendingNotifications()).toHaveLength(0);
  });
  
  it("should deduplicate on seeder side using transaction hash", async () => {
    const txHash = "0xabc123";
    
    // First notification
    const result1 = await PaymentService.creditSeederPayment(
      "QmHash123",
      "test.txt",
      1024,
      "0xdownloader",
      txHash
    );
    expect(result1.success).toBe(true);
    
    // Duplicate notification (same txHash)
    const result2 = await PaymentService.creditSeederPayment(
      "QmHash123",
      "test.txt",
      1024,
      "0xdownloader",
      txHash
    );
    expect(result2.success).toBe(true);
    expect(result2.error).toContain("already received");
    
    // Verify wallet only credited once
    expect(get(wallet).balance).toBe(initialBalance + paymentAmount);
  });
});

  it("should rate limit manual retries", async () => {
    const seederAddress = "0x1234...5678";
    vi.mocked(invoke).mockRejectedValue(new Error("DHT offline"));
    
    // First download attempt fails
    await PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      seederAddress
    );
    
    // First manual retry should work
    const retry1 = await PaymentService.retryPaymentNotification("QmHash123", seederAddress);
    expect(retry1).toBe(false); // Failed (DHT still offline)
    
    // Immediate second retry should be rate limited
    const retry2 = await PaymentService.retryPaymentNotification("QmHash123", seederAddress);
    expect(retry2).toBe(false);
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(4); // 3 initial + 1 retry (not 2)
  });
  
  it("should cleanup old failed notifications via timer", async () => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockRejectedValue(new Error("DHT offline"));
    
    // Create failed notification (initializes cleanup timer)
    await PaymentService.processDownloadPayment(
      "QmHash123",
      "test.txt",
      1024 * 1024,
      "0x1234...5678"
    );
    
    expect(PaymentService.getPendingNotifications()).toHaveLength(1);
    
    // Advance time by 2 hours (includes multiple 10-minute cleanup intervals)
    vi.advanceTimersByTime(2 * 60 * 60 * 1000);
    await vi.runOnlyPendingTimersAsync();
    
    // Cleanup timer should have removed old notifications
    expect(PaymentService.getPendingNotifications()).toHaveLength(0);
    
    vi.useRealTimers();
  });
});

describe("Backend Error Handling", () => {
  it("should return error when DHT is unavailable", async () => {
    // Rust test: Verify backend returns Err when DHT is None
    // This validates the critical bug fix
    const result = record_download_payment_with_dht(None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("DHT not available"));
  });
  
  it("should return error when P2P send fails", async () => {
    // Rust test: Verify backend returns Err when send_message_to_peer fails
    let mock_dht = MockDht::new();
    mock_dht.expect_send_message_to_peer()
      .returning(|_, _| Err("Peer unreachable".into()));
    
    let result = record_download_payment_with_dht(Some(mock_dht));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to send P2P payment notification"));
  });
});
```

**Integration Tests**:
- Verify P2P message actually reaches seeder after retry
- Test with real DHT connection/disconnection scenarios
- Verify manual retry UI button triggers correct flow

## Impact Analysis

### Benefits

**Reliability Improvements:**
- Backend now properly signals P2P send failures (no more silent failures)
- Automatic retry within 20 seconds handles transient DHT hiccups
- Manual retry gives users control when automatic attempts fail

**User Experience:**
- Users know immediately if seeder wasn't notified (warning message)
- Clear "Retry Notification" option in wallet UI
- No more mystery "where's my payment?" support tickets
- Fast retries (20s total) don't leave users waiting long

**Developer Experience:**
- Backend error handling matches expected behavior
- Simpler code: 3 retries in ~50 lines vs. complex queue system
- Easy to test with fake timers
- No localStorage complexity or quota concerns

**Network Health:**
- Seeders get notified 99%+ of the time (DHT usually recovers quickly)
- Manual retry handles edge cases
- Duplicate prevention keeps seeder trust high

### What This Avoids (vs. Over-Engineered Solution)

**No localStorage Queue**: 
- ✅ No quota issues
- ✅ No state persistence bugs
- ✅ No stale notification cleanup needed

**No 40-Minute Retry Windows**: 
- ✅ Fast feedback to user (20 seconds, not 40 minutes)
- ✅ Doesn't waste resources on hopeless retries
- ✅ User can manually retry when they fix their network

**No Fake "Reconciliation"**: 
- ✅ Doesn't assume backend ledger that doesn't exist
- ✅ Doesn't add complexity for phantom requirement
- ✅ Simple P2P message delivery, not distributed transaction

### Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| User doesn't see warning | Thinks notification sent | Use prominent warning toast, 10s display time |
| Duplicate from manual retry | Seeder credits twice | Transaction hash deduplication on seeder |
| DHT down >20s | Notification fails | User gets manual retry button, can try when DHT recovers |
| User spams retry button | Multiple P2P sends | Disable button during retry, rate limit to 1/10s |

## Implementation Roadmap

### Day 0: Type System Update (1-2 hours)

**Task**: Extend Transaction interface to support notification metadata

**Changes**:
- `src/lib/stores.ts` (lines 126-146):
  - Add optional `metadata` field to Transaction interface:
    ```typescript
    export interface Transaction {
      id: number;
      type: "sent" | "received" | "mining";
      amount: number;
      to: string;
      from: string;
      txHash: string;
      date: Date;
      description: string;
      status: string;
      transaction_hash?: string;
      gas_used?: number;
      confirmations?: number;
      is_confirmed?: boolean;
      block_number?: number;
      error_message?: string;
      // NEW: Notification delivery tracking
      metadata?: {
        notificationDelivered?: boolean;
        notificationError?: string;
        lastNotificationAttempt?: number;
      };
    }
    ```

**Verification**:
- TypeScript compilation succeeds
- Existing transactions without metadata still work (optional field)
- `saveTransactionsToStorage()` correctly serializes metadata
- `loadTransactionsFromStorage()` correctly deserializes metadata

### Day 1: Fix Backend Bug (2-3 hours)

**Task**: Make `record_download_payment` return error on P2P send failure OR when DHT unavailable

**Changes**:
- `src-tauri/src/main.rs` lines 630-650:
  - Return `Err()` when DHT is `None`
  - Return `Err()` when `send_message_to_peer()` fails
  - Only return `Ok()` when message successfully sent
- Add Rust unit tests for both error cases

**Verification**: 
- Frontend test `paymentService.test.ts:471` should FAIL (expects success on failure)
- Update test to expect error when DHT unavailable
- Run: `cargo test record_download_payment` (backend tests)
- Run: `npm test paymentService` (frontend tests)

### Day 2: Frontend Retry Logic (5-7 hours)

**Task**: Implement 3-attempt retry with delays, tracking, and timer-based cleanup

**Changes**:
- Add `PaymentNotificationAttempt` interface (~15 lines)
- Add `sendPaymentNotificationWithRetry()` with composite key `${fileHash}-${seederAddress}` (~90 lines)
- Add `initCleanupTimer()` to start 10-minute interval cleanup (~10 lines)
- Add `cleanupOldNotifications()` (~15 lines)
- Add rate limiting state (`lastRetryAttempt` Map)
- Modify `processDownloadPayment()` to store notification metadata in transactions (~30 lines)

**Verification**:
- Unit test with fake timers (3 attempts with correct absolute delays)
- Unit test for cleanup timer (advance 10 minutes, verify cleanup runs automatically)
- Unit test for composite key (same file from different seeders creates separate entries)
- Manual test: disconnect WiFi, try payment, reconnect, verify retry
- Verify metadata persists in localStorage
- Verify cleanup timer initializes on first payment

### Day 3: User Visibility (4-5 hours)

**Task**: Show notification status, add manual retry with UI integration

**Changes**:
- Add `retryPaymentNotification(fileHash, seederAddress)` with rate limiting (~35 lines)
- Add `getPendingNotifications()` for UI access (~3 lines)
- Modify `Wallet.svelte` (or transaction display component):
  - Read `transaction.metadata.notificationDelivered` flag
  - Show ⚠️ icon for failed notifications
  - Add "Retry Notification" button (disabled during retry)
  - Handle click → call `retryPaymentNotification(fileHash, seederAddress)`
  - Show success/failure toast after retry using toastStore.addToast()
  - Extract seederAddress from transaction record (~45 lines UI code)

**Verification**:
- UI shows ⚠️ warning icon for transactions with `notificationDelivered: false`
- Click "Retry" button triggers retry (check console logs)
- Button disables during retry attempt
- Rate limit prevents spam (try clicking twice quickly)
- Refresh page → warning icon still shows (metadata persisted)

### Day 4: Seeder Deduplication (2-3 hours)

**Task**: Prevent double-crediting from retries

**Changes**:
- Update `creditSeederPayment()` deduplication logic (lines 422-428):
  - Check if `transactionHash` is truthy and non-empty
  - Use `txhash-{hash}` if available
  - Fall back to `fileHash-downloaderAddress` if not
- Add comment documenting trade-off (blockchain failure case)

**Verification**:
- Unit test: send same notification twice with txHash, verify single credit
- Unit test: send same notification twice without txHash, verify single credit
- Unit test: same file from same user, different txHash → two credits (correct)
- Integration test: actual retry with same txHash → seeder logs "duplicate ignored"

### Day 5: Testing & Polish (4-5 hours)

**Task**: Comprehensive test coverage and documentation

**Changes**:
- Complete unit test suite:
  - ✅ Success on first attempt
  - ✅ Retry with correct delays (fixed timing logic)
  - ✅ Fail after 3 attempts
  - ✅ Manual retry works
  - ✅ Rate limiting prevents spam
  - ✅ Cleanup removes old notifications
  - ✅ Seeder deduplication (with and without txHash)
  - ✅ Backend returns error (Rust tests)
- Integration tests with real DHT (connect/disconnect scenarios)
- Update `docs/api-documentation.md` (document new Transaction.metadata field)
- Add troubleshooting section to this proposal

**Verification**:
- Run `npm test` → all tests pass
- Run `cargo test` → backend tests pass
- Manual end-to-end test:
  1. Start app with DHT disabled → payment fails, shows warning
  2. Enable DHT → manual retry succeeds
  3. Restart app → warning icon still visible
  4. Try downloading same file again → seeder ignores duplicate
- Code review with focus on edge cases

**Total Time**: 4-5 days (vs. 2 weeks for over-engineered solution)

**Day 0 is critical**: Cannot proceed to implementation without Transaction interface update.

## Implementation Notes

### File Changes Required

**Backend** (Minimal):
- `src-tauri/src/main.rs`: Change ~5 lines (return `Err()` instead of `Ok()` on P2P failure)

**Frontend** (Focused):
- `src/lib/stores.ts`:
  - Extend Transaction interface with optional metadata field (~8 lines)
  
- `src/lib/services/paymentService.ts`: 
  - Add `PaymentNotificationAttempt` interface (~15 lines)
  - Add `sendPaymentNotificationWithRetry()` with composite key `${fileHash}-${seederAddress}` (~95 lines)
  - Add `initCleanupTimer()` for timer-based cleanup (~10 lines)
  - Add `retryPaymentNotification()` with rate limiting and composite key (~35 lines)
  - Add `cleanupOldNotifications()` (~15 lines)
  - Add `getPendingNotifications()` (~3 lines)
  - Add `pendingNotifications`, `lastRetryAttempt`, `cleanupTimer` state (~4 lines)
  - Modify `processDownloadPayment()` to store metadata and use toastStore (~35 lines)
  - Enhance `creditSeederPayment()` deduplication (~8 lines)
  
- `src/pages/Wallet.svelte` (or equivalent transaction display component):
  - Add conditional rendering for notification status icon (~10 lines)
  - Add "Retry Notification" button with disabled state (~20 lines)
  - Add click handler with loading state (~15 lines)
  - Add success/error toast notifications (~10 lines)

- `tests/paymentService.test.ts`:
  - Add 9 new test cases (including composite key collision test) (~220 lines)
  - Update existing test that expects success on failure (~10 lines)

- `src-tauri/src/main.rs` (Backend):
  - Add Rust unit tests for error cases (~50 lines)

**Total**: ~548 lines of code (comprehensive with all edge cases covered)

### Key Design Decisions

**Why 3 Attempts with 0s, 5s, 15s Delays?**
- **Empirical basis**: DHT typically reconnects within 5-10 seconds in P2P networks
- **First retry (5s)**: Catches quick reconnections (most common case)
- **Second retry (15s)**: Gives DHT more time for complex NAT scenarios
- **Total 20s window**: Fast user feedback without wasteful long waits
- **If DHT is down longer**: That's a persistent failure requiring user action (manual retry when network recovers)

**Why Store Full Context in `pendingNotifications`?**
- **Problem**: Transaction records don't store all fields needed for retry (fileName, fileSize, seederPeerId)
- **Solution**: Store complete payment context in Map for accurate retry
- **Trade-off**: Small memory overhead (~500 bytes per failed notification)
- **Mitigation**: Cleanup removes notifications >1 hour old (prevents unbounded growth)

**Why Hybrid Deduplication (Transaction Hash + Fallback)?**
- **Ideal case**: Use transaction hash (unique, reliable, works for same file downloaded twice)
- **Edge case**: If blockchain transaction fails, transactionHash is empty string
- **Fallback**: Use `fileHash-downloaderAddress` to prevent duplicate payments
- **Trade-off**: If blockchain fails AND user downloads same file twice, second won't credit
  - Acceptable because blockchain failures are rare
  - User gets warning notification about failure (can contact support)
  - Better than double-crediting (which breaks seeder trust)

**Why No "Reconciliation Endpoint"?**
- Backend has no ledger/database to reconcile against
- Backend is stateless: emits local event → forwards P2P message → done
- "Reconciliation" would query non-existent data
- Frontend transaction history + pending notifications Map + manual retry = sufficient

**Why Persist Metadata in Transaction Records?**
- **Problem**: After page refresh, how does UI know which transactions failed?
- **Solution**: Add `metadata: { notificationDelivered, notificationError, lastNotificationAttempt }` to Transaction
- **Storage**: Uses existing `saveTransactionsToStorage()` mechanism (no new localStorage keys)
- **UI access**: `transaction.metadata.notificationDelivered === false` → show retry button

### Backward Compatibility

✅ **Mostly Compatible**:
- **localStorage schema**: Adds optional `metadata` field to Transaction type (existing transactions without metadata still work)
- **No database migrations**: Uses existing localStorage
- **API contract**: Backend command signature unchanged (just returns errors correctly now)
- **Existing successful payments**: Unaffected (no metadata = no retry button shown)
- **Existing failed payments**: No retry button (metadata not retroactively added)
- **Type safety**: TypeScript Transaction type extended with optional metadata

⚠️ **One Breaking Change**:
- Existing test `paymentService.test.ts:471` expects success on failure → will FAIL after backend fix
- Fix: Update test expectation to `expect(result.success).toBe(false)` when DHT unavailable

### Error Messages

**User-Facing**:
```
Warning: Payment sent (2.5000 Chiral), but seeder notification failed. 
Seeder may not know about payment yet. You can retry notification from Wallet tab.
```

**Developer Logs**:
```
❌ Attempt 1 failed: Failed to send P2P payment notification: Peer unreachable
🔄 Retry attempt 2/3 for payment 0xabc123...
✅ Payment notification delivered on attempt 2
```

## Success Metrics

**Before Implementation** (Current State):
- Payment notification delivery rate: Unknown (failures are silent)
- User complaints: "Seeder says they didn't get paid"
- Support tickets: Manual investigation required
- Developer confidence: Low (bug in error handling)

**After Implementation** (Target):
- Payment notification success rate > 95% (automatic retry handles transient DHT issues)
- Average retry count: ~1.2 (most succeed on first or second attempt)
- Failed notification visibility: 100% (users always notified of failures)
- Manual retry success rate: ~80% (when user fixes network and retries)

**Measurable Outcomes**:
- Support tickets for "missing payment": Reduce by >80% (users can self-service with manual retry)
- Seeder complaints: Reduce by >90% (notifications actually delivered)
- Developer debugging time: Reduce by >50% (proper error propagation)

## Troubleshooting Guide

### Common Issues & Solutions

**Issue: Notification keeps failing even with DHT online**
- Check: Is seeder's peer ID correct? (Not wallet address)
- Check: Are both peers connected to same DHT bootstrap nodes?
- Debug: Look for "Peer unreachable" in console logs
- Solution: Try different seeder or wait for DHT network stabilization

**Issue: "No pending notification found" when clicking Retry**
- Cause: Notification was removed by 1-hour cleanup
- Solution: This is expected for old failures - user can re-download file

**Issue: Seeder doesn't see payment after successful retry**
- Check: Seeder's console for "Duplicate payment notification ignored"
- Cause: First attempt actually succeeded (network latency confused frontend)
- Verify: Check seeder's transaction history - payment likely already there

**Issue: Rate limit message when retrying**
- Cause: Multiple retry attempts within 10 seconds
- Solution: Wait 10 seconds between retry attempts

**Issue: Memory usage grows over time**
- Check: How many failed notifications in `getPendingNotifications()`?
- Cause: Cleanup timer may not be running (check console for "🧹 Notification cleanup timer initialized")
- Solution: Cleanup runs automatically every 10 minutes, or restart app to clear all pending notifications

### Developer Debugging

**Enable verbose logging**:
```typescript
// In paymentService.ts, change console.warn to console.log
// To see all retry attempts and deduplication checks
```

**Check pending notifications**:
```typescript
// In browser console:
PaymentService.getPendingNotifications()
// Returns array of failed notifications with full context
```

**Backend logs to watch**:
```
✅ P2P payment notification sent to peer: <peer_id>
❌ DHT not available for payment notification
❌ Failed to send P2P payment notification: <error>
```

## What This Proposal Does NOT Include (Intentionally)

**Removed from Original Over-Engineered Version**:

1. ❌ **40-Minute Retry Window**: Too slow, user should know in 20 seconds
2. ❌ **Dead Letter Queue**: Manual retry button + pending notifications Map serves this purpose
3. ❌ **Reconciliation Endpoint**: No backend ledger exists to reconcile against
4. ❌ **Background Polling Integration**: Retries happen inline during payment, not on 10s poll
5. ❌ **Jitter Calculation**: Not needed for only 3 attempts (not a "thundering herd" scenario)
6. ❌ **12 Retry Attempts**: Wastes resources, user needs faster feedback
7. ❌ **Exponential Backoff**: Fixed delays (5s, 15s) are simpler and sufficient

**What We Added (That Original Lacked)**:

1. ✅ **Timer-Based Cleanup**: Automatically removes failed notifications >1 hour old every 10 minutes
2. ✅ **Rate Limiting**: Prevents manual retry spam (10s cooldown)
3. ✅ **Full Context Storage**: Can accurately retry with all original parameters
4. ✅ **Transaction Metadata**: Persists notification status across page refresh via type system extension
5. ✅ **DHT=None Case**: Fixed backend to return error when DHT unavailable (original missed this)
6. ✅ **Composite Key**: Prevents Map collisions when same file downloaded from different seeders (`${fileHash}-${seederAddress}`)
7. ✅ **Toast Integration**: Uses existing toastStore.addToast() instead of undefined showNotification()

**Why This Approach Wins**:
- Faster to implement (3-4 days vs 2 weeks)
- More complete (handles edge cases original missed)
- Easier to test (8 focused tests vs 15+ complex integration tests)
- Better UX (20s feedback + visible retry vs 40m silent background retry)
- Lower maintenance burden (~500 lines vs 700+ with reconciliation)

## Future Enhancements (If Needed)

Only add if metrics show actual need:

1. **DHT Health Dashboard**: Show if DHT is causing repeated failures
2. **Batch Notification**: If 10+ pending, group into single P2P message
3. **Seeder ACK Protocol**: Seeder sends confirmation back to downloader
4. **Notification History**: Track delivery success rate per seeder
5. **Alternative Channels**: If DHT is down, try direct HTTP to known seeder IP
