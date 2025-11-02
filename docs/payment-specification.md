# Payment & Pricing Specification

This document specifies the pricing, quoting, invoicing, payment, receipt, and ledger model for Chiral Network. It is intended as a complete reference for backend engineers, adapters (Stripe / onchain / Lightning), QA, and product.

---

## Goals

- Provide a stable API surface for quoting, invoicing, payment intents, and receipts.
- Support pluggable payment adapters; implement `StripeAdapter` first for MVP.
- Ensure integrity (signatures, idempotency), auditability (append-only ledger), and observability (metrics, events).

---

## DHT-first architecture (summary)

NOTE: The billing API is DHT- and libp2p-first. To support peers behind NATs and intermittent connectivity we rely on a combination of:

- libp2p Kademlia DHT for signed record discovery (price rules, canonical quote/invoice records)
- libp2p request/response protocol for direct RPC-like interactions when peers are reachable (via peer routing or relay)
- libp2p pubsub (gossipsub) topics for asynchronous notifications (invoice/receipt announcements, ledger updates)
- Signed JSON records (Ed25519 / libp2p PeerId signatures) for authenticity and non-repudiation
- Optional centralized adapters (Stripe) that run as stand-alone services and expose their availability via DHT records so peers can find them

Rationale: HTTP endpoints assume stable, public addresses. Chiral Network is P2P and many nodes are behind NATs or are intermittently connected. A DHT-first API allows discovery, store-and-forward, and eventual consistency while preserving end-to-end signatures and idempotency.

---

## Billing model

Primary billing mode: per-byte transferred ("bytes billed"). Optionally support per-job, per-session, and subscription modes.

### Pricing rule fields

- `unit`: `"byte" | "job" | "minute"`
- `base_price_microusd`: integer (price baseline in micro-USD)
- `min_charge_microusd`: integer
- `round_to`: integer (bytes; e.g. 1024)
- `tiers`: array of `{ threshold: number, unit_price_microusd: number }` — thresholds are inclusive lower bounds
- `region`: optional region code or `"*"` for global
- `effectiveFrom`: ISO timestamp
- `effectiveTo`: ISO timestamp | `null` for current
- `version`: semantic version or monotonically increasing integer

### Computation rules

1. Round billed bytes to `round_to` before applying pricing tiers.
2. `Total price = base_price_microusd * quantity + tier adjustments`.
3. Apply tier pricing based on rounded quantity and thresholds.
4. If computed total < `min_charge_microusd`, set total to `min_charge_microusd`.
5. Region-specific rules override global rules.
6. Store all prices in `microusd` (1e-6 USD). Convert to display currency at UI time using exchange rates.
7. Every pricing rule change creates a new `PriceRule` version record with `effectiveFrom`.

---

## High-level flows (DHT-first)

1. Quote: a client computes a quote locally (preferred) using locally cached price rules; if authoritative pricing is required the client issues a DHT request/response to a price-authority peer or queries a signed price rule record stored in the DHT. The response is a signed `Quote` JSON record with `expiresAt`.
2. Invoice: the client constructs a signed `Invoice` record (from the quote) and publishes it to the DHT under a per-account or per-provider key (or sends it directly to the provider's invoice inbox via request/response or pubsub topic). The invoice includes an `idempotencyKey` and is signed by the creator.
3. Payment: the payer completes payment using the chosen adapter. For adapters requiring a third-party (Stripe), the adapter may be hosted by an always-on service discovered via DHT; adapters publish payment-intent payloads back to the payer as a signed record. For onchain/lightning, adapters produce a payment descriptor (tx hash / invoice) and the payer broadcasts it to the network or directly to the payee.
4. Confirmation: once the payment is confirmed (adapter confirms or onchain confirmation observed), the payee writes a signed `Receipt` record and appends a `LedgerEntry`. The payee publishes a notification to the payee/payer pubsub inbox so both parties can reconcile. Records are persisted locally and optionally replicated via the DHT/replication layer.
5. Refunds/Disputes: managed by the party responsible for funds (adapter or payee). Adjustments are recorded as signed ledger entries and announced via pubsub.

Each stage emits local audit events and optional pubsub notifications: `quote.created`, `invoice.created`, `payment.initiated`, `payment.succeeded`, `receipt.issued`, `invoice.refunded`.

---

## Lifecycle sequence (DHT message view)

1. Client constructs or requests a quote using local price rules or via the request/response protocol `/<chiral>/billing/price/1.0.0` to a price rule publisher. The response is a signed `Quote` record with a `quoteId` and `expiresAt`.
2. Client constructs a signed `Invoice` JSON record (status `PENDING`) and publishes it:
   - Preferred: send directly to the payee's invoice inbox protocol `/<chiral>/billing/invoice/1.0.0` using libp2p request/response (fast if reachable).
   - Fallback: store the invoice in the DHT under key `quotes/<invoiceId>` and publish a short pubsub announcement to `billing/invoices/<payeePeerId>` so the payee will fetch it when online.
3. Payee processes the invoice, optionally transitions it to `RESERVED` (hold funds) and emits `invoice.created` event locally and via pubsub to the payer's inbox.
4. Payer requests a payment intent via `/<chiral>/billing/payment-intent/1.0.0` or uses an adapter discovered via DHT. The adapter returns an adapter-specific payload (client secret, checkout URL, or lightning invoice) as a signed JSON message.
5. Payer completes payment via the adapter. Adapter confirms using whichever channel is appropriate (onchain or provider webhook). For centralized adapter webhooks, the adapter publishes a signed confirmation message onto the payee's pubsub inbox or directly to the payee via request/response.
6. Payee validates the adapter confirmation, updates invoice to `PAID`, appends a signed `LedgerEntry`, issues a `Receipt`, and announces the receipt to payer via pubsub.
7. Parties reconcile using the signed records in their local DB and via DHT lookups.

Note: all records (quotes, invoices, payments, receipts, ledger entries) are signed by their issuer (peer key). Where centralized server behavior is required (e.g., Stripe-hosted adapter), that service publishes signed statements to the DHT so payees/payers can verify authenticity.

---

## DHT / Protocol surface (what to implement)

Implement a small, well-documented libp2p protocol and a set of DHT record keys and pubsub topics. Example protocols and topics (names are illustrative):

- Request/Response protocols (direct when peers reachable):
  - `/chiral/billing/price/1.0.0` — request: PriceQuery, response: Quote
  - `/chiral/billing/invoice/1.0.0` — request: InvoicePublish (client->payee) response: InvoiceAck
  - `/chiral/billing/payment-intent/1.0.0` — request: PaymentIntentRequest, response: PaymentIntentResponse
  - `/chiral/billing/payment-confirm/1.0.0` — adapter -> payee (confirmation)

- PubSub topics (asynchronous delivery / inboxes):
  - `billing.invoices.<peerId>` — payee's invoice announcements (new invoice available)
  - `billing.receipts.<peerId>` — receipts delivered to payer
  - `billing.ledger.<peerId>` — ledger entry announcements for account observers (admin/auditors)

- DHT signed record keys (store-and-fetch, canonical reference):
  - `price_rules/<region>/<version>` — contains a signed PriceRule JSON
  - `quote/<quoteId>` — signed quote record
  - `invoice/<invoiceId>` — canonical invoice record; payee may update status in their local DB but append-only records are stored as new signed records (e.g., `invoice/<invoiceId>/status/<seq>`)
  - `payment/<paymentId>` — signed payment record/confirmation
  - `receipt/<receiptId>` — signed receipt record

Design notes:
- Prefer request/response when peers are online and reachable; fall back to pubsub+DHT for store-and-forward.
- All canonical records must be signed; verifiers use the issuer's PeerId / public key for verification.
- Use compact, machine-readable JSON and include the `idempotencyKey` on create operations.
- Use sequence numbers and append-only records for status transitions to preserve auditability.

---

## Message fields & authentication (DHT messages)

Replace HTTP headers with message fields suitable for P2P transport. Every signed message SHOULD include:

- `senderPeerId`: libp2p PeerId string
- `timestamp`: ISO timestamp
- `signature`: signature of the serialized payload (use libp2p keypair; Ed25519 preferred)
- `signatureScheme`: e.g., `ed25519-libp2p` or `hmac-sha256` when a centralized service uses HMAC
- `idempotencyKey`: UUID (when creating resources)
- `auth`: optional token for centralized service interactions (JWT) — only used when interacting with an API gateway or managed adapter

Authentication model:
- End-to-end authenticity achieved by verifying `signature` with `senderPeerId` public key.
- For delegated/central adapters (Stripe), use adapter-signed DHT records and, when centralized, TLS/JWT for control channels.
- Local policy enforces scope/authorization: peers may accept invoices only from known payers or from peers that satisfy local trust rules.

---

## Message formats (examples, DHT messages)

### Quote (signed DHT record)

```json
{
  "quoteId": "q_01HABC...",
  "issuerPeerId": "12D3KooW...",
  "accountId": "acct_001",
  "unit": "byte",
  "bytes": 1048576,
  "amount_microusd": 12345,
  "currency": "USD",
  "expiresAt": "2025-11-01T12:00:00Z",
  "priceBreakdown": [
    {"type":"base", "unit_price_microusd":10, "quantity":1048576}
  ],
  "priceRuleVersion": "v1",
  "timestamp": "2025-10-27T12:00:00Z",
  "idempotencyKey": "idem-quote-abc",
  "signature": "ed25519:..."
}
```

### Invoice (peer-to-peer or DHT-published)

```json
{
  "invoiceId": "inv_01HABC...",
  "issuerPeerId": "12D3KooW...",
  "accountId": "acct_001",
  "quoteId": "q_01HABC...",
  "amount_due_microusd": 12345,
  "amount_paid_microusd": 0,
  "currency": "USD",
  "status": "PENDING",
  "expiresAt": "2025-11-01T12:10:00Z",
  "paymentMethods": ["stripe","onchain_token"],
  "metadata": {"job":"download", "fileId":"F123"},
  "timestamp": "2025-10-22T15:00:00Z",
  "idempotencyKey": "idem-abc-123",
  "signature": "ed25519:..."
}
```

### Payment confirmation / Receipt (signed)

```json
{
  "paymentId": "pay_01HABC...",
  "invoiceId": "inv_01HABC...",
  "issuerPeerId": "12D3KooW...",
  "method": "stripe",
  "status": "SUCCEEDED",
  "amount_microusd": 12345,
  "currency": "USD",
  "providerReference": "pi_1G...",
  "receiptUrl": "https://stripe/receipt/...",
  "paidAt": "2025-10-22T15:01:10Z",
  "timestamp": "2025-10-22T15:01:15Z",
  "signature": "ed25519:..."
}
```

---

## Status enums / lifecycle

- Quote: `DRAFT`, `EXPIRES`, `EXPIRED`
- Invoice: `PENDING`, `RESERVED`, `PAID`, `SETTLED`, `FAILED`, `REFUNDED`, `CANCELLED`
- Payment: `INITIATED`, `PENDING`, `SUCCEEDED`, `FAILED`

---

## Adapter interface (abstract)

Adapters must implement the same logical methods, but transport is required to be P2P-friendly. Adapter implementers MUST:

- publish adapter availability and control endpoint metadata as signed DHT records (e.g., `adapter/stripe/<instanceId>`) that include a verification public key or verification hint and reachable multiaddrs when available
- support libp2p request/response protocols for adapter-driven interactions when the adapter host and client/peer are mutually reachable
- publish all adapter confirmations (payment completions, refunds, disputes) as signed canonical records into the DHT (e.g., `payment/<paymentId>`, `receipt/<receiptId>`) and/or push them to the recipient's pubsub inbox (e.g., `billing.receipts.<peerId>`) so recipients behind NATs can fetch or receive them
- expose machine-readable provider event ids (e.g., Stripe `event.id`) and include these in the published confirmation record so recipients can deduplicate
- never assume the recipient has a public HTTP endpoint; if the adapter receives provider webhooks (e.g., Stripe), the adapter host MUST re-publish a signed confirmation into the DHT/pubsub and not rely solely on HTTP callbacks

Adapter interface methods remain (createPaymentIntent, verifyWebhook/confirm, capture, refund, getPaymentStatus) — signatures and payloads are the same JSON structures above, but transport is libp2p request/response or pubsub messages. If an adapter additionally offers a centralized HTTP gateway for convenience, it MUST still publish canonical signed records into the DHT/pubsub when state changes (payments/refunds) occur.

---

## DB model (core tables)

The DB model remains useful for local persistence by each peer/service instance. Nodes SHOULD persist canonical signed records they issue or receive. In addition:

- When a node issues an append-only state change (invoice status change, ledger entry), it should write a new signed record and optionally publish a replication hint in the DHT so other interested parties can fetch it.
- Centralized services (if used) still maintain their DB but expose signed records into the DHT so other peers can verify.

```sql
-- accounts
CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT,
  balance_microusd BIGINT DEFAULT 0,
  currency TEXT DEFAULT 'USD',
  created_at TIMESTAMP DEFAULT now()
);

-- price rules (versioned)
CREATE TABLE price_rules (
  id TEXT PRIMARY KEY,
  unit TEXT,
  base_price_microusd BIGINT,
  min_charge_microusd BIGINT,
  round_to BIGINT,
  tiers JSONB,
  region TEXT,
  effective_from TIMESTAMP,
  effective_to TIMESTAMP,
  version TEXT,
  created_at TIMESTAMP DEFAULT now()
);

-- quotes
CREATE TABLE quotes (
  id TEXT PRIMARY KEY,
  account_id TEXT REFERENCES accounts(id),
  params JSONB,
  amount_microusd BIGINT,
  expires_at TIMESTAMP,
  price_rule_version TEXT,
  signature TEXT,
  created_at TIMESTAMP DEFAULT now()
);

-- invoices
CREATE TABLE invoices (
  id TEXT PRIMARY KEY,
  quote_id TEXT REFERENCES quotes(id),
  account_id TEXT REFERENCES accounts(id),
  amount_due_microusd BIGINT,
  amount_paid_microusd BIGINT DEFAULT 0,
  status TEXT,
  expires_at TIMESTAMP,
  metadata JSONB,
  idempotency_key TEXT,
  created_at TIMESTAMP DEFAULT now()
);

-- payments
CREATE TABLE payments (
  id TEXT PRIMARY KEY,
  invoice_id TEXT REFERENCES invoices(id),
  amount_microusd BIGINT,
  method TEXT,
  provider_ref TEXT,
  status TEXT,
  raw_provider_payload JSONB,
  created_at TIMESTAMP DEFAULT now()
);

-- ledger (append-only)
CREATE TABLE ledger_entries (
  id TEXT PRIMARY KEY,
  account_id TEXT REFERENCES accounts(id),
  type TEXT,
  amount_microusd BIGINT,
  balance_after BIGINT,
  related_id TEXT,
  raw JSONB,
  created_at TIMESTAMP DEFAULT now()
);
```

---

## Ledger integrity rules

- Ledger is append-only: no updates or deletions allowed.
- Each new ledger entry computes `balance_after` using the previous balance for that account.
- Ledger entries MUST be signed by the issuer's private key (preferably the node's libp2p/Ed25519 keypair). Do not rely on symmetric HMAC for general peer-signed ledger entries; HMAC may be used as an additional envelope when a centralized adapter hosts records but the canonical signature should be the adapter's public-key signature.
- Reconciliation job runs regularly to ensure `accounts.balance_microusd` matches last `balance_after` for that account. Mismatches create incidents.
- Use key-rotation records published into the DHT to announce new verification keys; verifiers should accept signatures from the previous key for a configurable overlap window.

---

## Security & integrity (DHT specifics)

- Authentication uses libp2p PeerIds and public-key signatures (preferred) for message authenticity. HMAC can be used for centralized services but is not suitable for general P2P authentication.
- Require `idempotencyKey` on create operations; nodes must deduplicate by `(issuerPeerId, idempotencyKey)`.
- All important records (quotes, invoices, payments, receipts, ledger entries) must be signed by the issuer's private key and timestamped. Recipients verify signatures and optionally use a trust policy before acting.
- Rotate keys per node as appropriate; publish a signed key-rotation notice in the DHT to allow verifiers to migrate trust.

---

## Webhook / adapter confirmations (DHT-friendly)

- Centralized adapter services (Stripe or other hosted adapters) MUST publish adapter confirmations as signed canonical DHT records (e.g., `payment/<paymentId>`, `receipt/<receiptId>`) and/or push them to the payee's pubsub inbox `billing.receipts.<peerId>`.
- Adapter control records (`adapter/<adapterType>/<instanceId>`) SHOULD include these fields: `instanceId`, `adapterType`, `multiaddrs` (optional), `verificationKey` (public key or key fingerprint), `signatureScheme`, `capabilities` (e.g., supports: payment-intent, refund, capture), `lastSeen`, and a `signature` by the adapter host key. Example:

```json
{
  "instanceId":"stripe-01abc",
  "adapterType":"stripe",
  "multiaddrs":["/ip4/1.2.3.4/tcp/443"],
  "verificationKey":"ed25519:...",
  "signatureScheme":"ed25519-libp2p",
  "capabilities":["payment_intent","refund"],
  "lastSeen":"2025-10-27T12:00:00Z",
  "signature":"ed25519:..."
}
```

- When an adapter host receives a provider webhook (e.g., Stripe `payment_intent.succeeded`), it MUST:
  1. Verify the provider webhook authenticity using the provider's recommended verification mechanism (e.g., Stripe-Signature and stripe SDK).
  2. Construct a signed canonical confirmation record that includes: `adapterInstanceId`, `providerEventId`, `paymentId` (internal), `invoiceId` (if known), `status`, `amount_microusd`, `currency`, `providerReference`, `timestamp`, and the adapter host `signature`.
  3. Publish the canonical confirmation record to the DHT under `payment/<paymentId>` and/or publish a compact notification to `billing.receipts.<payeePeerId>` (payload may be the `paymentId` and adapterInstanceId) to trigger the payee to fetch the full record.
  4. Persist the raw provider payload in the adapter host's local DB for audit; include a pointer (providerEventId) in the canonical record.

- Recipients must deduplicate by `(adapterInstanceId, providerEventId)` and validate the adapter host signature against the adapter control record's `verificationKey` fetched from the DHT.
- If the payee cannot be reached directly, pubsub announcements and DHT storage ensure the payee can fetch the signed confirmation when online.

---

## Stripe adapter design (reference guidance)

For the MVP StripeAdapter implementation (hosted by an always-on service), follow this pattern:

1. Adapter host obtains and verifies Stripe webhooks using `STRIPE_WEBHOOK_SECRET` and the Stripe SDK.
2. On receiving a verified Stripe event that implies payment success/failure/refund, adapter constructs a canonical signed confirmation record (see above) and publishes it to the DHT under `payment/<paymentId>` and/or `receipt/<receiptId>`.
3. Adapter publishes a short pubsub notification to the payee's inbox `billing.receipts.<payeePeerId>` with minimal fields `{ paymentId, adapterInstanceId }` so payee can fetch the full signed record.
4. Adapter includes `providerEventId` (Stripe event id) and `providerReference` (payment_intent id) in the canonical record to support deduplication and reconciliation.
5. Adapter hosts must maintain idempotency when processing provider webhooks: deduplicate by `providerEventId`, persist processing state, and ensure a single canonical confirmation record is published per unique provider event.
6. Adapter control record (`adapter/stripe/<instanceId>`) must be published and signed to DHT so clients can verify adapter signatures and discover multiaddrs or other reachability hints.

Security notes for StripeAdapter:
- The adapter's canonical confirmation MUST be signed with the adapter host's long-lived key that is discoverable via the DHT control record.
- Do not publish raw provider secrets or unverified payloads into the DHT. Only publish signed, minimal canonical confirmations that include a secure pointer to locally-stored raw provider payload (for auditing by the adapter host only).
- The adapter host should provide a mechanism for payees to fetch raw provider payload via an authenticated channel (out-of-band) if required for dispute resolution. However, canonical settlement and ledger reconciliation should rely solely on signed canonical confirmation records published to the DHT.

---

## Reviewer checklist (quick)

- [ ] All API interactions are feasible with DHT + libp2p request/response + pubsub (no dependency on public HTTP endpoints for core flow).
- [ ] Adapter implementations (StripeAdapter included) publish signed canonical confirmation records to the DHT/pubsub and include `providerEventId` for dedup.
- [ ] Ledger entries and canonical records are signed with node/adaptor public-key signatures (Ed25519/libp2p) and key-rotation is supported.
- [ ] Idempotency dedup is defined as `(issuerPeerId, idempotencyKey)` and adapter dedup by `(adapterInstanceId, providerEventId)`.
- [ ] Store-and-forward fallback flow via DHT + pubsub is clearly documented and examples provided.
- [ ] Security reviewers confirm that private keys and adapter secrets are not published and are stored in secure vaults; adapter hosts expose only signed verification keys in DHT records.

---

## Service health & availability (P2P-aware)

- Health checks remain useful for always-on nodes (central adapters, indexing nodes). For peers behind NAT there is no HTTP /health expectation; instead implement a `status` request over the request/response protocol.
- Use probe nodes and monitoring nodes that are globally reachable to validate DHT replication and adapter availability.

---

## Edge cases & business rules

- Partial payments, reserved funds, and refund windows remain the same conceptually. For onchain tokens, adapters must publish confirmations that are verifiable by all peers.
- Conflict resolution: when two differing signed records exist for what should be a canonical state, rely on append-only sequence numbers and deterministic conflict rules (higher sequence number wins; tie-break by signature timestamp and issuer identity as configured).

---

## Client migration notes (mapping local paymentService to DHT-based flows)

Many frontend screens call a local `paymentService`. When migrating client code to use the DHT-based billing model, follow these guidelines:

- Local compute remains first-class: continue to use `paymentService.calculateDownloadCost(bytes)` locally for instant UX.
- To obtain authoritative quotes or when a signed quote is required, use libp2p request/response: send a `PriceQuery` to a price-authority peer or fetch `price_rules/<region>/<version>` from the DHT.
- To create an invoice:
  - Construct a signed `Invoice` object locally and attempt a direct request/response delivery to the payee's `/<chiral>/billing/invoice/1.0.0` protocol.
  - If direct delivery fails (peer unreachable), store the invoice in the DHT under `invoice/<invoiceId>` and publish a short pubsub announcement to `billing.invoices.<payeePeerId>` so the payee will fetch it when online.
- To create a payment intent: query for adapter availability via DHT (e.g., `adapter/stripe` records) and request an intent over the adapter's request/response protocol or use local wallet/onchain path.
- Wait for confirmation:
  - Poll local DB for a signed `payment`/`receipt` record, or
  - Subscribe to the payer/payeepubsub inbox `billing.receipts.<peerId>` to receive notifications when the other party publishes the receipt.

Example pseudo-code (libp2p request/response):

```js
// 1. Request authoritative quote
const quote = await libp2p.request(peerMultiaddr, '/chiral/billing/price/1.0.0', { bytes, region })
verifySignature(quote)

// 2. Create signed invoice and send to payee
const invoice = sign({ invoiceId, quoteId, ... })
try {
  const ack = await libp2p.request(payeeMultiaddr, '/chiral/billing/invoice/1.0.0', invoice)
  verifyAck(ack)
} catch (err) {
  // fallback: DHT store + pubsub announcement
  await dht.put('invoice/' + invoice.invoiceId, invoice)
  await pubsub.publish('billing.invoices.' + payeePeerId, { invoiceId: invoice.invoiceId })
}
```

---

## Signing examples (libp2p / ed25519)

Use the node's libp2p keypair to sign the serialized payload. Keep `signatureScheme` to indicate scheme.

```js
import { signPayload } from 'libp2p-crypto-helper'
const payload = JSON.stringify(quote)
const signature = await signPayload(localKeypair, payload) // produces ed25519:...
quote.signature = signature
```

---

## Example DHT/pubsub flows (happy path)

1) Authoritative quote request

- Client: request `/chiral/billing/price/1.0.0` -> price-authority peer (if reachable)
- Price-authority: returns signed `Quote` record
- Client: verifies signature and uses quote

2) Create invoice (peer-to-peer preferred)

- Client: constructs signed `Invoice` and sends request over `/chiral/billing/invoice/1.0.0` to payee
- Payee: acknowledges; optionally responds with `RESERVED` status record
- If direct fail: client stores `invoice/<invoiceId>` in DHT and publishes a pubsub announcement so payee will fetch

3) Payment & confirmation

- Client: creates payment via adapter (adapter discovered via DHT). Adapter publishes a signed confirmation to `billing.receipts.<payeePeerId>` or directly to the payee's payment-confirm protocol
- Payee: verifies payment confirmation, appends `LedgerEntry`, publishes `receipt/<receiptId>` to DHT, and publishes a pubsub notification to `billing.receipts.<payerPeerId>`

---

## Required environment variables / secrets

- `DATABASE_URL` — primary DB connection (for always-on nodes / adapter hosts)
- `BILLING_SIGNING_KEY` — node's private key for signing records (libp2p keypairs recommended)
- `STRIPE_API_KEY` — Stripe secret (only for centralized adapter hosts)
- `STRIPE_WEBHOOK_SECRET` — Stripe webhook signing secret (only for centralized adapter hosts)
- `REDIS_URL` (optional) — idempotency key store / queue backend for always-on services
- `MIGRATIONS_DIR` — migrations location
- `SENTRY_DSN` / `PROMETHEUS_PUSH_URL` (optional) — observability

Secrets handling
- Store private signing keys securely; rotate keys by publishing signed key-rotation records in the DHT to allow verifiers to update trusted keys.

