# Spec: Chunk Download — Bitswap Primary, HTTP Fallback

Purpose
-------
This document specifies a robust, production-focused implementation strategy for downloading file chunks in the Chiral Network. The system's primary mechanism for chunk delivery is Bitswap/P2P; however, this spec makes HTTP a first-class fallback transport for chunk retrieval. The goal is to ensure high availability and reliability for chunk retrieval while preserving integrity, security, and performance.

Scope
-----
- Define when and how HTTP fallback should be used alongside Bitswap.
- Define HTTP endpoints and URL formats for chunk retrieval.
- Specify verification, retry, timeout, and backoff policies.
- Detail integration points and implementation steps in the codebase (recommended files and APIs).
- Describe metrics, monitoring, and testing strategies.

Goals
-----
- Avoid data unavailability when Bitswap peers are slow, unreachable, or misbehaving.
- Provide a secure HTTP-based fallback that can be used with CDNs, S3, FTP-to-HTTP gateways, or relay servers.
- Preserve content integrity using cryptographic verification (Merkle root, chunk hashes).
- Limit the performance and security impact of fallback usage with adaptive selection and throttling.

Actors & Concepts
-----------------
- Peer / P2P source — primary Bitswap peer that can serve chunks.
- HTTP source — a hosted HTTP(S) endpoint that serves one or more chunks.
- Chunk — a contiguous byte-range of the original file, identified by chunk index and associated hash.
- File metadata — contains file size, merkle root, chunk table (chunk_id → hash, offset, size), and optional `http_sources` list.

Design Principles
-----------------
1. Verification-first: Accept data only after verifying chunk hash and its inclusion in the file Merkle root.
2. Minimal trust: Treat HTTP sources as untrusted (verify every chunk) and prefer HTTPS.
3. Fail fast: Use conservative short timeouts and exponential backoff for HTTP; escalate to more retries only when necessary.
4. Resource aware: Limit concurrent HTTP requests per host and per download to avoid overload.
5. Observability: Track metrics for fallback events (how often, latencies, success rate).

When to use HTTP fallback
-------------------------
HTTP should serve as a fallback in these cases (ordered):

1. No Bitswap peers are available (peer discovery returns zero results).
2. Bitswap peers are available but cannot establish connections within a short connection timeout (configurable, e.g., 10–30s).
3. Bitswap connection is established but chunk delivery stalls (no progress for a configured window).
4. Certain chunks repeatedly fail from Bitswap sources after the configured retry threshold.

Behavioral rules
----------------
- The download orchestrator attempts Bitswap first for each chunk up to N retries (configurable).
- When fallback is triggered for a chunk, the orchestrator records the event, attempts HTTP download, and verifies the chunk.
- The orchestrator should prefer Bitswap for future chunks unless HTTP demonstrates consistently better performance.

HTTP Source discovery & URL formats
-----------------------------------
File metadata may expose zero or more HTTP sources. Each HTTP source entry SHOULD include:

- base_url: string — base HTTP(S) URL (e.g., https://cdn.example.org/files/)
- path_template: optional string — template for chunk URL, e.g. `{base}/{merkle_root}/chunks/{chunk_id}`
- supports_range: bool — whether server supports Range requests (recommended)
- auth: optional — type: none | bearer | signed-url | basic; details for auth
- priority: optional integer — weight/priority when selecting among multiple HTTP sources
- cache_control: optional string — server cache control hint

Canonical URL examples

- Chunk by ID (path):

  https://cdn.example.org/files/ABCD1234/chunks/0001

- Chunk by byte-range (single-file):

  https://cdn.example.org/files/ABCD1234/file.bin

  with header: Range: bytes=65536-131071

- S3 signed URL example (pre-signed GET):

  https://s3.aws-region.amazonaws.com/bucket/path?X-Amz-Algorithm=...

For maximum interoperability the HTTP client SHOULD support both chunk-per-URL and byte-range approaches. When `supports_range` is true, prefer a single-file URL with Range headers as it is more cache/CDN friendly.

HTTP Request/Response contract
------------------------------
- Request headers the client should send: `Accept: application/octet-stream`, `User-Agent: chiral/1.0`, optional `Range` for byte ranges.
- Response codes:
  - 200 OK — full entity (if the response body is exactly the chunk bytes expected, acceptable for chunk-per-URL).
  - 206 Partial Content — response to Range header; body contains requested range.
  - 404/410 — missing resource; treat as non-recoverable for that HTTP source.
  - 5xx — server error; subject to retry/backoff.
- Response validation: length, SHA-256 or configured chunk hash check, and Merkle inclusion.

Chunk verification
------------------
All HTTP-delivered chunks MUST be verified before accepted into the file assembly:

1. Compute the chunk hash using the same algorithm as the file metadata.
2. Compare computed hash to metadata chunk hash.

If verification fails, mark the HTTP source as suspicious (reduce priority) and continue retries with other sources. Do NOT accept unverified chunks.

Performance & concurrency
------------------------
- Per-download concurrent HTTP requests: default 4 (configurable).
- Per-host/concurrency limit: 8 concurrent connections (configurable) to avoid saturating single servers.
- Chunk-level parallelization: allow parallel downloads if file has >= MIN_CHUNKS_FOR_PARALLEL (existing constant) and sources > 1.
- Use async HTTP client (reqwest or hyper) with connection pooling and keep-alive.

Timeouts, retries, and backoff
-----------------------------
- Connection timeout: 5–15 seconds (configurable). Shorter timeouts encourage fallback only when connection is impossible.
- Read timeout: 15–60 seconds (configurable).
- Retries for 5xx: exponential backoff, e.g., initial 500ms, multiplier 2, max attempts 3.
- For transient network errors (timeouts), apply immediate fallback to next source while retaining an overall file-level retry budget.
- For 404/410: do not retry; mark resource absent on that source.

Integration with Bitswap
-----------------------
The orchestrator must maintain a per-chunk state machine:

- Desired state transitions:
  - Unassigned → Assigned-to-Bitswap → Downloading → Completed
  - Unassigned → Assigned-to-HTTP → Downloading → Completed
  - If Assigned-to-Bitswap fails after X attempts → reassign to HTTP (fallback)

Policy suggestions:
- Maintain a global retry budget per chunk: e.g., 3 Bitswap attempts before HTTP fallback.
- For large files, implement adaptive scheduling: prefer Bitswap initially; if overall throughput is low, opportunistically use HTTP for remaining chunks.

Cache & CDN considerations
-------------------------
- Prefer byte-range HTTP on single-file URLs to increase cacheability (CDNs can cache ranges if configured).
- Respect Cache-Control headers when deciding whether to re-request a chunk from HTTP; still verify hashes even if served from cache.

Security considerations
---------------------
- Prefer HTTPS and validate TLS certificates by default.
- For private HTTP sources, use signed URLs (S3 pre-signed URLs) or bearer tokens.
- Consider certificate pinning for curated HTTP sources (optional, higher operational cost).
- Log and metric suspicious HTTP sources that repeatedly deliver invalid or mismatched chunks.

Metrics & telemetry
-------------------
Track the following metrics for observability and future tuning:

- fallback_http_triggered_total{file_hash,chunk_id} — counter for fallback events.
- http_chunk_latency_seconds — histogram of HTTP chunk retrieval latencies.
- http_chunk_verification_failures_total{source} — counter when validation fails.
- http_chunk_success_total{source} — successful HTTP chunk downloads.
- bitswap_vs_http_bytes_total{protocol} — bytes served by each transport.

Errors & remediation
--------------------
- If no HTTP sources are available and Bitswap fails, surface `DownloadFailed` event with clear error codes: e.g., `NoSources`, `BitswapTimeout`, `HttpAllFailed`.
- Allow manual retry of failed downloads; orchestrator should be able to persist and resume download state to avoid re-downloading completed chunks.

Implementation plan (step-by-step)
---------------------------------
This section gives a practical roadmap for implementing HTTP fallback in the existing codebase.

1) Data model & metadata
   - Extend `FileMetadata` (in `crate::dht`) to include `http_sources: Option<Vec<HttpSourceInfo>>` where `HttpSourceInfo` contains `base_url`, `supports_range`, `auth`, `priority`, and optional `headers`.

2) HTTP client module
   - Add `src-tauri/src/http_source.rs` (or reuse existing `download_source` module) with an async client wrapper around `reqwest::Client`.
   - Client responsibilities:
     - Perform GET/Range requests with configured timeouts and retries.
     - Respect per-host and per-download concurrency limits.
     - Return raw bytes and HTTP response metadata (status, headers).

3) Orchestrator integration (`multi_source_download.rs`)
   - Detect fallback conditions in the chunk state machine.
   - When fallback is triggered for chunk(s):
     - Query `FileMetadata::http_sources` for candidate HTTP sources.
     - Iterate candidate sources respecting priority and per-host concurrency.
     - Call HTTP client to fetch chunk bytes.
     - Verify chunk bytes (hash, Merkle inclusion).
     - On success, insert `CompletedChunk` into `ActiveDownload` and emit `ChunkCompleted` event.
     - On verification failure, mark source as suspicious and continue.

   - Suggested insertion points (existing file):
     - `start_source_connections` / `start_http_download` — extend to implement HTTP behavior instead of returning not implemented.
     - `handle_retry_failed_chunks` — choose HTTP sources when Bitswap retries are exhausted.

4) Error mapping & events
   - Add more granular events in `MultiSourceEvent`, e.g., `HttpFallbackTriggered`, `HttpChunkVerified`, `HttpSourceBlacklisted`.

5) Tests
   - Unit tests for HTTP client: success, 206 partial, 404, 5xx.
   - Integration tests: local HTTP server (e.g., `warp` or `hyper` test server) that serves chunked files; verify the orchestrator accepts valid chunks and rejects invalid ones.
   - Fuzz test: corrupt chunk data from HTTP and ensure verification and source blacklisting behavior.

6) Monitoring
   - Wire metrics to the existing telemetry pipeline.

Detailed flow example (pseudocode)
---------------------------------
  // Attempt Bitswap first
  for chunk in chunks {
      if try_bitswap(chunk) { continue; }

      // Bitswap failed after retries -> attempt HTTP fallback
      for http_src in metadata.http_sources.sorted_by_priority() {
          let result = http_client.fetch_chunk(http_src, chunk.offset, chunk.size).await;
          if result.is_ok() {
              if verify_chunk(&result.bytes, chunk.hash) {
                  mark_completed(chunk, result.bytes);
                  break; // move to next chunk
              } else {
                  mark_source_suspicious(http_src);
                  continue; // next HTTP source
              }
          } else {
              if result.is_non_recoverable() { break; }
              // otherwise try next HTTP source
          }
      }
  }

Edge cases & trade-offs
-----------------------
- Fighting heterogeneity: HTTP sources might provide different URL schemas. Use `path_template` plus a small templating engine to construct URLs safely.
- Cost: HTTP fallback may incur egress charges (S3/CND). Prioritize Bitswap where cost is a concern.
- Privacy: HTTP fallback leaks access patterns to HTTP providers. Document this and allow opt-out in privacy-sensitive contexts.

Security & compliance notes
-------------------------
- Ensure TLS certificate validation is strict.
- For enterprise deployments that require private HTTP mirrors, support signed URLs or token-based auth.

Appendix — Example HTTP requests
--------------------------------
GET chunk-by-id (chunk-per-URL):

  GET /files/ABCD1234/chunks/0001 HTTP/1.1
  Host: cdn.example.org
  Accept: application/octet-stream
  User-Agent: chiral/1.0

Response (200): body contains exact chunk bytes. Verify hash.

GET with Range (byte-range):

  GET /files/ABCD1234/file.bin HTTP/1.1
  Host: cdn.example.org
  Range: bytes=65536-131071
  Accept: application/octet-stream

Response (206): body contains requested range. Verify hash.

Implementation checklist
------------------------
- [ ] Add `HttpSourceInfo` to `FileMetadata`.
- [ ] Implement `http_source.rs` client with reqwest wrapper and concurrency control.
- [ ] Implement HTTP fallback in `multi_source_download.rs` (`start_http_download`, `handle_retry_failed_chunks`).
- [ ] Add chunk verification utilities (if not present).
- [ ] Add metrics and log events for fallback operations.
- [ ] Add unit and integration tests.
- [ ] Add feature flag + rollout plan.

References
----------
- RFCs on HTTP Range Requests: https://datatracker.ietf.org/doc/html/rfc7233
- S3 pre-signed URL docs
- Bitswap/Ipfs chunking best practices

