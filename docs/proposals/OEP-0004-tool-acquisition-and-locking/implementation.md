# Build preflight and tool-lock integration

## Lock mode and preflight order

`oyzu build` starts with engine preflight: management context, config validity, source boundary, executor capabilities, requested tools, lock compatibility, eligibility, then verified installation. Candidate discovery may read native manifests before tool installation; no project code executes during that read. Tool inspection cannot trust PATH or executable version output alone: installation identity must match a committed, verified store record.

Use `oyzu.lock` as the initial tool-lock filename. `oyzu install` resolves approved requests, writes exact per-platform distributions and commits a replacement lock atomically. An explicit `oyzu install --update` changes version resolution. Ordinary builds are frozen: they install missing locked tools but do not update the source lock. A zero-Oyzu-config project may obtain an ephemeral exact tool snapshot from native exact metadata, user defaults or managed defaults during preparation; it is recorded in the plan and diagnosed as an uncommitted tool selection. CI policy can require a committed Oyzu/native equivalent lock and reject this path. Zero configuration is not an exception to captured tool identities.

Tools without enough version information produce an actionable selection diagnostic. Builtin recommendations are versioned descriptor data, not live `latest` queries. Acquiring a previously absent platform distribution is a lock change, never a silent build-side edit. Offline preflight requires all exact distributions and applicable still-valid authorization. It does not run network probes to discover whether public access might work.

## Lock record and store transaction

The proposed lock is TOML with `format = 1` and a `[[tool]]` array. Each tool record has `id` (canonical backend/publisher/name), `request`, `version`, `backend_digest` and a `[[tool.distribution]]` list with `platform`, `digest`, `size`, `source_id`, `verification` and `dependencies` (canonical tool identities). Entries and dependency lists are sorted; every dependency resolves to an exact entry. Cycles, conflicting content for the same identity/version/platform and incomplete closures fail. The tool lock excludes mirror URLs with authorization, ports, tokens and absolute installed paths. `verification` is a typed evidence reference and strength classification, not an assertion that a checksum proves publisher identity.

Store layout is implementation-owned under the OS Oyzu data directory: `cas/sha256/<digest>`, `tools/<distribution-key>/`, `staging/<operation-id>/`. Acquire a per-distribution process lock, download into staging, verify bytes and archive limits, extract without following escaping links, validate required entrypoints, atomically rename and write a committed installation descriptor. Readers only select committed records. On Windows, open executable leases prevent removal; prune retries after leases end. Store recovery removes abandoned staging only after verifying no active process owns the lease.

Policy and content validity are independent checks. Token rotation does not alter a distribution digest. A policy revocation can deny existing bytes without deleting an installation out from under active processes; new actions must reauthorize and currently running operations follow revocation instructions. Source-built tools route through the same captured build model; unsupported backend subprocess downloads are denied rather than trusted because mise supports that backend.

## Implementation work and verification

Implement lock parse/normalize/conflict checks, then content-store commit/recovery, then backend route mediation and platform-specific activation. Use synthetic archives for interrupted transfers, changed bytes, archive traversal, concurrent install/prune, Windows open-file behavior and revoked cached tools. TOOL-01 through TOOL-06 remain authoritative. Pinned mise module/license integration and source verification roots require their own audited implementation evidence before enabling an upstream backend.
