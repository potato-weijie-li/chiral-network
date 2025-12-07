import { describe, it, expect } from 'vitest';
import { scoreVerdicts } from '../src/lib/services/reputationService';
import type { TransactionVerdict } from '../src/lib/types/reputation';

describe('scoreVerdicts', () => {
  const now = 1_700_000_000; // fixed timestamp for deterministic tests

  const baseVerdict = (overrides: Partial<TransactionVerdict>): TransactionVerdict => ({
    target_id: 'peer-b',
    tx_hash: null,
    outcome: 'good',
    details: undefined,
    metric: 'transaction',
    issued_at: now,
    issuer_id: 'peer-a',
    issuer_seq_no: 1,
    issuer_sig: 'deadbeef',
    ...overrides,
  });

  it('returns zero score when there are no verdicts', () => {
    const result = scoreVerdicts([], 30, now);
    expect(result.score).toBe(0);
    expect(result.total).toBe(0);
  });

  it('applies weights for disputed outcomes', () => {
    const verdicts: TransactionVerdict[] = [
      baseVerdict({ outcome: 'good', issuer_seq_no: 1 }),
      baseVerdict({ outcome: 'disputed', issuer_seq_no: 2 }),
      baseVerdict({ outcome: 'bad', issuer_seq_no: 3 }),
    ];

    const result = scoreVerdicts(verdicts, 30, now);
    expect(result.total).toBe(3);
    expect(result.score).toBeCloseTo((1 + 0.5) / 3, 4);
  });

  it('deduplicates by issuer, tx_hash, and seq', () => {
    const verdicts: TransactionVerdict[] = [
      baseVerdict({ issuer_seq_no: 1 }),
      baseVerdict({ issuer_seq_no: 1, details: 'duplicate' }),
      baseVerdict({ issuer_seq_no: 2 }),
    ];

    const result = scoreVerdicts(verdicts, 30, now);
    expect(result.total).toBe(2); // duplicate removed
  });

  it('drops verdicts older than TTL', () => {
    const verdicts: TransactionVerdict[] = [
      baseVerdict({ issued_at: now - 10 }), // recent
      baseVerdict({ issued_at: now - 10_000 }), // expired with TTL 100
    ];

    const result = scoreVerdicts(verdicts, 100, now);
    expect(result.total).toBe(1);
    expect(result.score).toBe(1);
  });
});

