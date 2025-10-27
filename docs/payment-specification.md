# Payment & Pricing Specification

This document specifies the pricing, quoting, invoicing, payment, receipt, and ledger model for Chiral Network. It is intended as a complete reference for backend engineers, adapters (Stripe / onchain / Lightning), QA, and product.

---

## Goals

- Provide a stable API surface for quoting, invoicing, payment intents, and receipts.
- Support pluggable payment adapters; implement `StripeAdapter` first for MVP.
- Ensure integrity (signatures, idempotency), auditability (append-only ledger), and observability (metrics, events).

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

## High-level flows

1. **Quote**: client requests an estimated price (parameters: bytes, region, transfer type). Server returns signed quote with expiration.
2. **Invoice**: client creates invoice from quote; server may reserve funds depending on adapter.
3. **Payment**: client completes payment via adapter (Stripe checkout, client secret, Lightning invoice, or onchain transfer).
4. **Confirmation**: provider webhook marks invoice `PAID`; server issues a receipt and writes ledger entry.
5. **Refunds/Disputes**: handled by admin or automated workflow; append-only ledger entries record adjustments.

Each stage emits audit events: `quote.created`, `invoice.created`, `payment.succeeded`, `receipt.issued`, `invoice.refunded`, etc.

---

## Lifecycle sequence 

1. Client requests a quote via `GET /v1/price` or `POST /v1/quotes`.
2. Server validates inputs, computes price using active price rules, and returns signed `Quote` object with `expiresAt`.
3. Client posts `POST /v1/invoices` with `quoteId` and `idempotency-key`.
4. Server creates `Invoice` in `PENDING` or `RESERVED` state and emits `invoice.created` audit event.
5. Client obtains payment instructions using `POST /v1/payments` (adapter returns client-facing payload).
6. Client completes payment; adapter sends webhook to `POST /v1/webhooks/payment`.
7. Server validates webhook, verifies signature/provider API if desired, marks invoice `PAID`, appends `LedgerEntry`, and issues `Receipt`.
8. Record final audit events and metrics.

---

## API surface

All endpoints require authentication (JWT for users; API keys for services). Use scopes: `billing:read`, `billing:write`, `billing:admin`.

### Public / authenticated

- `GET /v1/price?bytes=&region=&type=download|upload` → returns Quote (estimation)
- `POST /v1/quotes` → create quote (body optional; returns signed quote)
- `GET /v1/quotes/{id}`
- `POST /v1/invoices` → create invoice from quote (requires Idempotency-Key header)
- `GET /v1/invoices/{id}`
- `POST /v1/payments` → create payment intent (adapter-specific payload)
- `GET /v1/payments/{id}`
- `POST /v1/webhooks/payment` → adapter webhook receiver (public endpoint; validate signature)

### Admin (scoped: billing:admin)

- `GET|POST|PUT /v1/price-rules` → CRUD price rules and versioning
- `GET /v1/ledger` → read-only ledger with pagination, filters
- `GET /v1/invoices?status=`
- `POST /v1/refunds` → admin refund operation

### Common headers

- `Authorization: Bearer <JWT>` or `Authorization: ApiKey <key>`
- `Idempotency-Key: <uuid>` — required for `POST /v1/invoices` and `POST /v1/payments`
- `X-Signature: <hex>` — HMAC-SHA256 (server-signed quotes/receipts)

---

## Message formats (examples)

### Quote

```json
{
  "quoteId": "q_01HABC...",
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
  "signature": "hmac-sha256-..."
}
```

### Invoice (server-created)

```json
{
  "invoiceId": "inv_01HABC...",
  "accountId": "acct_001",
  "quoteId": "q_01HABC...",
  "amount_due_microusd": 12345,
  "currency": "USD",
  "status": "PENDING",
  "expiresAt": "2025-11-01T12:10:00Z",
  "paymentMethods": ["stripe","onchain_token"],
  "metadata": {"job":"download", "fileId":"F123"},
  "createdAt": "2025-10-22T15:00:00Z",
  "idempotencyKey": "idem-abc-123"
}
```

### Payment & Receipt (Stripe example)

```json
{
  "paymentId": "pay_01HABC...",
  "invoiceId": "inv_01HABC...",
  "method": "stripe",
  "status": "SUCCEEDED",
  "amount_microusd": 12345,
  "currency": "USD",
  "providerReference": "pi_1G...",
  "receiptUrl": "https://stripe/receipt/...",
  "paidAt": "2025-10-22T15:01:10Z",
  "signature": "hmac-sha256-..."
}
```

---

## Status enums / lifecycle

- Quote: `DRAFT`, `EXPIRES`, `EXPIRED`
- Invoice: `PENDING`, `RESERVED`, `PAID`, `SETTLED`, `FAILED`, `REFUNDED`, `CANCELLED`
- Payment: `INITIATED`, `PENDING`, `SUCCEEDED`, `FAILED`

---

## Adapter interface (abstract)

Adapters must implement these methods so core logic is adapter-agnostic:

- `createPaymentIntent(invoice): Promise<{ adapterPayload }>` — returns client payload (checkout URL, client secret, lightning invoice)
- `verifyWebhook(rawBody, headers): Promise<{ valid: boolean, event: NormalizedEvent }>` — verifies signature and maps to normalized event
- `capturePayment(paymentId): Promise` (if adapter supports capture)
- `refund(paymentId, amount): Promise` — adapter-level refund
- `getPaymentStatus(providerRef): Promise<PaymentStatus>` — polling or verification
- `sandboxMode: boolean` flag support

Adapters must surface errors with machine-readable codes and not mutate canonical ledger directly.

---

## DB model (core tables)

DDL-style model (simplified)

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
- Ledger entries are signed with server key (HMAC-SHA256 or Ed25519) and stored alongside raw data.
- Reconciliation job runs regularly to ensure `accounts.balance_microusd` matches last `balance_after` for that account. Mismatches create incidents.

---

## Security & integrity

- Auth: JWT for users, API keys for internal services. Scopes: `billing:read`, `billing:write`, `billing:admin`.
- Require `Idempotency-Key` for `POST /v1/invoices` and `POST /v1/payments`.
- Sign quotes and receipts with server secret. Attach `X-Signature` header to responses.
- Validate provider webhooks (Stripe signature verification) and optionally confirm events with provider API.
- Store raw webhook payloads for audit and troubleshooting; record verification result.
- Rotate signing secrets every 90 days; allow 7-day overlapping secret acceptance window.

---

## Webhook verification & handling

- Verify `Stripe-Signature` timestamp within ±5 minutes.
- Store provider `event_id` and deduplicate using unique constraint `(adapter_id, provider_event_id)`.
- Retry policy for processing webhooks:
  - Accept retries; process idempotently.
  - Exponential backoff for internal retry (1s, 2s, 5s, 15s, 60s...).
  - After N failed attempts (configurable, default 10), create incident and queue manual review.
- If webhook signature invalid: record in `webhook_failures` table and return HTTP 400.
- Optionally call provider API to confirm event authenticity; accept confirmed events even if signature check failed.

---

## Normalized webhook events

- `invoice.paid` (on provider success)
- `invoice.payment_failed`
- `invoice.refunded`

Adapter-specific events should be normalized to the above types.

---

## Deduplication & Idempotency

- Require `Idempotency-Key` header for POST endpoints that create resources.
- If an idempotency key was already used, return the original response with `409 CONFLICT_IDEMPOTENCY` or 200 + resource depending on design.
- For webhooks, deduplicate using `provider_event_id` + `adapter_id` unique constraint.

---

## Error handling & machine-readable errors

Return JSON `{ "message": "...", "machine_code": "...", "details": {...} }`.

Common machine codes:
- `INVALID_INPUT` (400)
- `PAYMENT_REQUIRED` (402)
- `CONFLICT_IDEMPOTENCY` (409)
- `QUOTE_EXPIRED` (422)
- `INTERNAL` (500)

---

## Observability & testing

### Metrics
Implement metrics for the following (Prometheus style):
- `quotes_created_total` (counter)
- `invoices_paid_total` (counter)
- `payments_failed_total` (counter)
- `quote_latency_seconds` (histogram)
- `webhook_processing_latency_seconds` (histogram)
- `ledger_append_latency_seconds` (histogram)
- `balance_mismatch_detected_total` (counter)
- `refund_processing_total` (counter)
- `adapter_error_total` (counter)
- `sandbox_transaction_success` (gauge)

### Testing guidance
- Unit tests for price computation, tiering, rounding, region overrides, and rule versioning.
- Integration tests using Stripe test keys and replayable test webhooks.
- Tests to simulate duplicate webhooks and idempotent POSTs.
- Simulate network timeouts and verify retry behavior.

---

## Service health checks
- `/health` must return `200` if DB, queue, and adapter are reachable.
- Include synthetic transaction tests for sandbox mode (e.g., hourly sandbox checkout).

---

## Edge cases & business rules
- Partial payments: support only if invoice has `allowPartial` flag; otherwise require full payment.
- Reservation/holding: `RESERVED` reduces available balance but does not finalize until capture.
- Currency conversion: compute canonical amounts in `microusd`; adapters handle conversion and fees.
- Refund windows: default 7 days; configurable per account or product.

---

## Migration to crypto (tokens / Lightning)
- Keep canonical amounts in microUSD.
- Use oracles to quote token amounts for onchain adapters.
- Adapter returns onchain tx hash or lightning invoice; backend watches confirming event.
- Ledger and invoice model remain unchanged.

---

## Minimal OpenAPI sketch (example)

```yaml
openapi: 3.0.1
info:
  title: Chiral Billing API
  version: "1.0"
paths:
  /v1/price:
    get:
      summary: Quote price
      parameters:
        - in: query
          name: bytes
          schema:
            type: integer
        - in: query
          name: region
          schema:
            type: string
        - in: query
          name: type
          schema:
            type: string
            enum: [download, upload]
      responses:
        '200':
          description: Quote
  /v1/quotes:
    post:
      summary: Create quote
  /v1/invoices:
    post:
      summary: Create invoice from quote
  /v1/payments:
    post:
      summary: Create payment intent
  /v1/webhooks/payment:
    post:
      summary: Adapter webhook endpoint
```

---

## Additional recommended items to add (suggestions)

1. **Authorization policy matrix** - detailed mapping of endpoints to roles/scopes.
2. **Rate limits** - per-account and per-IP rate limits for quote and invoice creation to prevent abuse.
3. **Data retention policy** - how long raw provider payloads and signatures are retained.
4. **Operational runbook** - handling stuck payments, ledger mismatches, and webhook floods.
5. **Migration plan** - for rotating signing secrets and migrating price rule versions.
6. **Sample adapter implementation** - a small `StripeAdapter` reference in `src/lib/adapters/stripe.ts`.
7. **OpenAPI/Swagger contract** - full schema for types used by clients.

---

## Questions & TODOs
- Decide whether `POST /v1/invoices` returns `200` for idempotent repeat or `409` with original resource link.
- Decide canonical currency display / exchange rate provider and caching policy.
- Confirm refund windows and partial payment policy.

---

## Contact / ownership
Billing & payments: @payments-team
Security / secrets rotation: @security-team

---

## Client migration notes (mapping existing client `paymentService` to server API)

Many frontend screens already call a local `paymentService` for in-app payments. When migrating to the server-backed billing API, follow these guidelines:

- Replace local compute calls with server endpoints:
  - `paymentService.calculateDownloadCost(bytes)` → `GET /v1/price?bytes=<>&region=<>&type=download` (use returned quote)
  - `paymentService.createInvoice(...)` → `POST /v1/invoices` with `Idempotency-Key`
  - `paymentService.createPaymentIntent(invoice)` → `POST /v1/payments` (adapter-specific payload returned)
  - Event flow: wait for adapter's webhook to mark invoice PAID; client polls `GET /v1/invoices/{id}` or uses websocket/event notification.

- Example minimal client flow (pseudo-code):

```js
// 1. Quote
const quote = await fetch(`/v1/price?bytes=${bytes}&region=${region}&type=download`, { headers }).then(r=>r.json());
// 2. Create invoice (idempotent)
const idempotency = crypto.randomUUID();
const invoice = await fetch('/v1/invoices', { method: 'POST', headers: { 'Idempotency-Key': idempotency, 'Content-Type': 'application/json' }, body: JSON.stringify({ quoteId: quote.quoteId }) }).then(r=>r.json());
// 3. Create payment intent
const paymentIntent = await fetch('/v1/payments', { method: 'POST', headers: { 'Idempotency-Key': crypto.randomUUID(), 'Content-Type': 'application/json' }, body: JSON.stringify({ invoiceId: invoice.invoiceId, method: 'stripe' }) }).then(r=>r.json());
// 4. Redirect / collect payment using adapter-provided client payload
// 5. Poll invoice status or subscribe to notifications
```

- Update UI code paths to handle `409 CONFLICT_IDEMPOTENCY` and treat it as safe (retrieve and use existing resource). Use `Idempotency-Key` consistently across retries.

---

## HMAC signing & verification examples

Server signs quotes and receipts using HMAC-SHA256. Headers used:
- `X-Signature` — hex-encoded HMAC-SHA256 of body
- `X-Signature-Timestamp` — ISO timestamp used when signing
- `X-Signature-Version` — semantic version of signing scheme (e.g., `v1`)

Node.js signing example (server)

```js
import crypto from 'crypto';

function signBodyHmac(secret, body) {
  const ts = new Date().toISOString();
  const payload = typeof body === 'string' ? body : JSON.stringify(body);
  const h = crypto.createHmac('sha256', secret).update(payload).digest('hex');
  return { signature: h, ts };
}

// Usage
const { signature, ts } = signBodyHmac(process.env.BILLING_SIGNING_SECRET, quoteObj);
// Send headers: X-Signature, X-Signature-Timestamp, X-Signature-Version: v1
```

Verification (server receiving webhooks) — pseudo-code

```js
function verifyHmac(secret, rawBody, signatureHex) {
  const expected = crypto.createHmac('sha256', secret).update(rawBody).digest('hex');
  return crypto.timingSafeEqual(Buffer.from(expected,'hex'), Buffer.from(signatureHex,'hex'));
}
```

Key rotation notes
- Keep `BILLING_SIGNING_SECRET_ACTIVE` and `BILLING_SIGNING_SECRET_PREVIOUS` for a 7-day overlap window.
- Accept signatures signed by either active or previous during rotation; log and audit which key verified the signature.

---

## Quick curl examples (end-to-end happy path)

1) Quote (estimation)

```bash
curl -H "Authorization: Bearer $JWT" "https://api.example.com/v1/price?bytes=1048576&region=us&type=download"
```

2) Create quote (POST) — optional

```bash
curl -X POST -H "Authorization: Bearer $JWT" -H "Content-Type: application/json" -d '{"bytes":1048576, "region":"us"}' https://api.example.com/v1/quotes
```

3) Create invoice from quote (idempotent)

```bash
curl -X POST -H "Authorization: Bearer $JWT" -H "Idempotency-Key: $(uuidgen)" -H "Content-Type: application/json" -d '{"quoteId":"q_01..."}' https://api.example.com/v1/invoices
```

4) Create payment intent (Stripe example)

```bash
curl -X POST -H "Authorization: Bearer $JWT" -H "Idempotency-Key: $(uuidgen)" -H "Content-Type: application/json" -d '{"invoiceId":"inv_01...","method":"stripe"}' https://api.example.com/v1/payments
```

5) Simulate webhook (local/testing only) — include provider `event_id` and signature

```bash
RAW='{"id":"evt_test_123","type":"payment_intent.succeeded","data":{"object":{"id":"pi_1...","amount":12345,"metadata":{"invoiceId":"inv_01..."}}}}'
SIG=$(echo -n "$RAW" | openssl dgst -sha256 -hmac "$BILLING_SIGNING_SECRET" | sed 's/^.* //')

curl -X POST -H "Content-Type: application/json" -H "Stripe-Signature: t=12345,v1=$SIG" --data "$RAW" http://localhost:3000/v1/webhooks/payment
```

---

## Required environment variables / secrets

- `DATABASE_URL` — primary DB connection
- `BILLING_SIGNING_SECRET` (or `BILLING_SIGNING_SECRET_ACTIVE` / `_PREVIOUS`) — HMAC key used for quote/receipt signing
- `STRIPE_API_KEY` — Stripe secret for adapter
- `STRIPE_WEBHOOK_SECRET` — Stripe webhook signing secret
- `REDIS_URL` (optional) — idempotency key store / queue backend
- `MIGRATIONS_DIR` — migrations location
- `SENTRY_DSN` / `PROMETHEUS_PUSH_URL` (optional) — observability

Secrets handling
- Rotate `BILLING_SIGNING_SECRET` every 90 days with 7-day overlap.
- Store secrets in a secure vault (HashiCorp Vault, AWS Secrets Manager) and do not commit to repo.

