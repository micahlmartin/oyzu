---
id: OEP-0004
title: Tool acquisition and locking
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002, OEP-0003]
tracking-issue: null
---

# Tool acquisition and locking

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.


## Problem and intent

`oyzu install` and build preflight must resolve exact approved toolchains without making developers manage download URLs. Supporting many tools requires on-demand acquisition; advance mirroring and repackaging of every version is not the architecture.

## Entities

A tool identity includes backend, publisher/namespace, and tool. A distribution adds exact version, OS, architecture, ABI constraints, and content identity. A catalog maps requests to candidates; a connector supplies acquisition routes and credential strategies. A catalog entry does not itself authorize an arbitrary network destination.

Proposed lock entries record logical identity, resolved version, platform-specific distribution digests, backend implementation identity, verification evidence references, and relevant dependency closure. They MUST NOT contain credentials, bearer URLs, local agent ports, or machine paths. Distinguish immutable package identity from mutable mirror routing.

## Resolution and installation

State sequence: discover request → select context → resolve permitted candidate → authorize route → acquire → verify → stage → validate layout → atomically commit → activate.

An existing valid lock selects exact content. An initial interactive install may resolve and write a lock. Explicit update changes resolution. Proposed CI behavior is frozen: missing or incompatible lock entries fail rather than silently select versions. Shared locks contain explicit per-platform records; an unsupported platform requires a deliberate lock expansion.

Package installation uses per-distribution locking and a content store. Verify digests before extraction; reject absolute paths, traversal, escaping links, decompression bombs, and unexpected file types. Preserve executable bits and legitimate internal links. Completed installation records are distinct from incomplete staging directories. Cancellation must leave previous installations usable.

## Standalone and managed acquisition

Standalone uses configured supported backends without Oyzu login. Managed resolution is filtered by tool/publisher/version allowlists from the platform, and acquisition goes directly from agent to an approved corporate connector or approved proxy. No hosted Oyzu package transit or customer gateway is required.

A proxy/cache fetches missing content on demand where the configured upstream supports that behavior. Existing Nexus/Artifactory storage is reused. No blanket promise that an arbitrary repository can proxy every URL format is made. Unsupported routing fails or requests administrator configuration.

The platform may mediate backend discovery metadata, but client-side download authorization cannot rely solely on a filtered search list. Exact requests, cached installs, and lock entries are rechecked against applicable current restrictions.

## Verification and upstream changes

Use publisher signatures/attestations when supported and authenticated digest metadata. A self-supplied checksum from the same untrusted response is not publisher identity. Record verification strength instead of pretending all backends offer equivalent evidence.

If an upstream changes bytes for an exact locked version, fail. If content was evicted and disappears upstream, report unavailability; a lock is not storage. Retention for historical release toolchains is a separate operational requirement.

## Node installation walkthrough

For a proposed `oyzu install node@22` invocation, the selected backend interprets the request as a version constraint, not a download URL. It enumerates Node distribution metadata for the host platform. Standalone discovery uses the supported backend's configured sources; managed discovery uses the organization's filtered catalog and current exact-install authorization.

The resolver selects a permitted exact version and platform archive, verifies the metadata's integrity basis, and binds the request to an approved route. In a managed installation that route might be an Artifactory remote/generic repository configured to proxy the permitted Node distribution source. The agent obtains a narrowly scoped read lease and requests that archive directly from Artifactory. Artifactory can fetch a cache miss from its approved upstream on demand.

The agent does not accept an arbitrary upstream URL supplied by the project. If the repository cannot proxy that backend's distribution format, installation fails with an administrator-facing routing requirement. Oyzu does not quietly contact the public source or pre-publish a substitute OCI image.

After verification, Oyzu extracts into staging, commits the tool installation, records exact version/platform/content identity in the lock, and exposes the selected executable through activation or shims. Subsequent preflight verifies the locked installation and current eligibility. A tool revoked by policy is not allowed merely because the bytes are already on disk.

## Catalog and installed-tool lifecycle

The CLI should support searching tools, listing permitted versions, inspecting backend/origin/verification status, installing, explicitly updating locks, showing active resolution, uninstalling, and pruning unused content. Exact command names beyond existing examples remain a UX decision.

Catalog precedence must preserve canonical backend/publisher identity. A shorthand alias may select an identity but cannot replace its verification or policy scope. Multiple configured sources claiming conflicting identities or bytes produce a conflict, not a silent first-match install. Metadata freshness and unavailable version reasons should be inspectable.

Uninstall/prune must not delete distributions held by active processes or concurrent installations. Reference tracking and process leases distinguish unused cached content from in-use tool installations. Platform-specific executable locking is handled without changing another terminal's selected version.

## Acceptance criteria

- TOOL-01: initial install writes exact resolution; reinstall preserves it.
- TOOL-02: cached content still undergoes policy eligibility checks.
- TOOL-03: managed backend aliases and explicit backend syntax cannot bypass the identity allowlist.
- TOOL-04: corrupt archives, path escapes, revoked versions, and disallowed redirects fail safely.
- TOOL-05: concurrent same-tool installs converge on one valid installation.
- TOOL-06: Windows/macOS/Linux platform variants are selected and recorded separately.

## Open decisions

Exact lock schema and integrity root, catalog metadata update-signing protocol, installation directory conventions, default optional-platform lock coverage, and handling source-built tools need prototypes. Source-built tools must declare their own build dependencies and use the controlled execution model.
