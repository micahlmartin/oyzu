# Build preflight and tool-lock integration

The implementation-ready [OEP-0003 contract](../OEP-0003-mise-integration/README.md)
now specifies the proposed mise integration, commands, authorization and receipt
store. Its [format-2 lock contract](../OEP-0003-mise-integration/lock-and-store.md)
supersedes this companion's original format-1 sketch. Both remain drafts; no
production installation capability or maintainer acceptance is implied.

## Lock mode and preflight order

`oyzu build` starts with engine preflight: management context, config validity, source boundary, executor capabilities, requested tools, lock compatibility, eligibility, then verified installation. Candidate discovery may read native manifests before tool installation; no project code executes during that read. Tool inspection cannot trust PATH or executable version output alone: installation identity must match a committed, verified store record.

Use `oyzu.lock` as the initial tool-lock filename. `oyzu install` resolves approved requests, writes exact per-platform distributions and commits a replacement lock atomically. An explicit `oyzu install --update` changes version resolution. Ordinary builds are frozen: they install missing locked tools but do not update the source lock. A zero-Oyzu-config project may obtain an ephemeral exact tool snapshot from native exact metadata, user defaults or managed defaults during preparation; it is recorded in the plan and diagnosed as an uncommitted tool selection. CI policy can require a committed Oyzu/native equivalent lock and reject this path. Zero configuration is not an exception to captured tool identities.

Tools without enough version information produce an actionable selection diagnostic. Builtin recommendations are versioned descriptor data, not live `latest` queries. Acquiring a previously absent platform distribution is a lock change, never a silent build-side edit. Offline preflight requires all exact distributions and applicable still-valid authorization. It does not run network probes to discover whether public access might work.

## Lock record and store transaction

The proposed lock is TOML format 2 under OEP-0003. Scoped/profile environment
records select canonical tool keys; exact per-platform distributions bind
backend, content, layout, verification and dependency identities. This permits
different versions in different project scopes while rejecting incompatible
versions in one closure. Source routes and credentials stay outside the lock.
The field schema, canonical digest algorithm, editing transaction and explicit
experimental migration are defined in the linked contract rather than duplicated
here. Verification strength is not an assertion that a checksum proves publisher
identity.

The implementation-owned store follows OEP-0003's versioned layout and receipt
transaction. It verifies content before extraction and publishes payload plus
receipt in one atomic directory operation; readers never select partial state.
Receipt/tree identity and current selection authorization are mandatory even
when mise reports the version installed. Active process/session leases prevent
prune, including Windows open-file cases. Recovery validates containment and
operation ownership before removing abandoned staging.

Policy and content validity are independent checks. Token rotation does not alter a distribution digest. A policy revocation can deny existing bytes without deleting an installation out from under active processes; new actions must reauthorize and currently running operations follow revocation instructions. Source-built tools route through the same captured build model; unsupported backend subprocess downloads are denied rather than trusted because mise supports that backend.

## Implementation work and verification

Implement lock parse/normalize/conflict checks, then content-store commit/recovery, then backend route mediation and platform-specific activation. Use synthetic archives for interrupted transfers, changed bytes, archive traversal, concurrent install/prune, Windows open-file behavior and revoked cached tools. TOOL-01 through TOOL-06 remain authoritative. Pinned mise module/license integration and source verification roots require their own audited implementation evidence before enabling an upstream backend.
