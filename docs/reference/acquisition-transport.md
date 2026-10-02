# Scoped dependency acquisition transport

The experimental dependency broker provides the approved HTTP fetch boundary used by captured builder preparation. Native managers retain lockfile interpretation, dependency resolution and package integrity verification. The broker fetches bytes from admitted routes; it does not run a resolver or install tools. This reference describes the current engine transport, not the planned corporate agent or a complete managed registry service.

## Routing and credential boundary

Builder preparation supplies explicit source routes. A route binds an identifier, HTTPS origin/path prefix and optional host-held authorization header. HTTP is allowed only for loopback test/local endpoints. Credential-bearing base URLs, query/fragment-bearing bases and encoded path escapes are rejected. Every request and redirect destination must match an admitted route. A redirect uses the destination route's credentials; the original token never follows it implicitly. Ambient HTTP proxy settings are not used by this transport.

Offline preparation containers use a private file channel to request downloads; they receive status, content type, source identity and body bytes, without upstream authorization or response headers such as cookies. Native Maven and Gradle adapters translate their loopback repository requests into this channel. Admitted standalone sources are currently selected by adapters; arbitrary private registries, corporate connector discovery and general managed credential refresh remain separate unfinished integrations.

`oyzu build` performs acquisition before freezing the execution plan. Later build actions use prepared inputs and denied networking. Retries described here occur only during approved acquisition; a missing input or integrity failure during offline execution never triggers a download fallback. Development commands such as `oyzu run install` retain their documented host/native execution behavior and are not made isolated by this broker.

## Retry behavior and limits

One logical fetch has at most two retries in addition to its initial attempt. The retry count and a 45-second network deadline are shared across redirect destinations; redirecting does not renew either budget. The existing maximum of five HTTP redirect-chain requests remains bounded separately. Each actual request also consumes the session's 4096-request allowance, including retries and redirects. The HTTP library's automatic protocol retries are disabled so they cannot silently extend this budget.

These are per-fetch broker bounds. A native manager can submit another logical request; its own retry behavior remains native and is constrained by the session and enclosing acquisition limits. This is not a per-package attempt counter or a persistent upstream cooldown shared across requests.

Retryable upstream statuses are 408, 429, 502, 503 and 504. Connection/request/body transport failures can also retry; partial transfers are discarded before a fresh request. A complete response still goes through its adapter's integrity checks. Statuses such as 401, 403, 404 and 500 are returned immediately. Denied routes, invalid redirects and resource-limit failures are terminal in the broker. Package checksum failures remain terminal at their adapter boundary.

Backoff is 200 ms, then 400 ms. A supplied `Retry-After` delay or HTTP date sets a longer minimum wait, using the standard [HTTP semantics](https://www.rfc-editor.org/rfc/rfc9110.html#name-retry-after). Invalid headers or waits that cannot fit the remaining deadline return the current upstream failure instead of retrying early. A timeout may exhaust the whole deadline before any retry is possible. There is no new project setting for these bounds.

Successful-response bodies are limited to 128 MiB each and 1 GiB read per session. Bytes from partial/failed attempts count toward the session total. Upstream error bodies are discarded without reading or relaying them; clients receive a fixed message containing the HTTP status. These are broker payload bounds, not a claim about total TCP traffic or native resolver memory use. Native acquisition has additional enclosing execution limits.

## Failure interpretation and recovery

The private channel now preserves the difference between denied access and unavailable transport. Broker-generated failures include a sanitized `errorCode` and a fixed body, with no requested URL, query, token or upstream error text:

| HTTP status | Broker code | Meaning and next step |
| --- | --- | --- |
| 400 | `REQUEST_INVALID` | Malformed channel input; correct the adapter/request contract |
| 403 | `SOURCE_DENIED` | No approved route permits the request or redirect; fix the declared source integration |
| 413 | `BYTE_LIMIT` | Payload/session bounds exceeded; inspect dependency size and acquisition scope |
| 429 | `REQUEST_LIMIT` | Session request allowance exhausted; inspect resolution scope rather than retrying that exhausted session |
| 502 | `SOURCE_UNAVAILABLE` | Transport failed after eligible retries; check availability/connectivity and rerun preparation |
| 502 | `REDIRECT_INVALID`, `REDIRECT_LIMIT` | Invalid or excessive redirect chain; correct the source endpoint |
| 504 | `SOURCE_TIMEOUT` | Network deadline expired; investigate the source/connectivity before rerunning |

Upstream HTTP failures retain their actual status and a redacted body; they do not acquire a fabricated broker code. Native managers determine how they display those statuses. Maven now requests native error stack traces (`-e`) to retain nested resolution causes in failed-build diagnostics; debug logging (`-X`) is not enabled. A failed preparation produces no successful dependency snapshot or executable build plan. A later explicit build prepares fresh inputs and evidence.

Previously, the channel reported every internal failure as 403. Consumers must handle non-success statuses generally rather than assuming every unsuccessful request is a permission denial. The optional `errorCode` field is additive; existing native clients still receive the original status/body fields. This does not change successful dependency records or artifact identities.

The channel still services requests serially. Its client timeout and resolver concurrency can expose queue delays under slow upstreams; this change does not establish concurrent transport or eliminate every acquisition outage. Active blocking requests are not immediately interruptible on session shutdown. General cancellation, queue scheduling and managed retry-policy delivery remain work. A successful retry does not authenticate a dependency or establish release eligibility.

## Verification

`cargo test --locked --test broker` uses real local HTTP servers to verify retryable statuses, partial-body replacement, retry exhaustion, terminal statuses, authorization on retries/redirects, a retry budget shared across hosts, error-body redaction and channel failure classification. Owner-local tests verify delay/date/deadline decisions and terminal byte/request limits. These checks run in the Windows, macOS and Linux CLI jobs before captured scenarios.

The `httpdate` dependency owns HTTP-date parsing instead of a handwritten parser. Version 1.0.3 is MIT OR Apache-2.0, declares Rust 1.56 support and uses portable standard-library time types; its lock entry is generated by Cargo. It does not change route authorization or clocks used for monotonic request deadlines.

See [implementation status](../implementation-status.md) for completed checks. Controlled HTTP failures prove transport behavior; they do not establish the cause or resolution of an unrelated Maven Central incident. Actual Java/native scenario success remains separate CI evidence.
