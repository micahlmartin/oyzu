# Lock identity and installation transactions

Normative draft companion to [OEP-0003](README.md). Format 2 is proposed, not a
shipped lock format. Native `package-lock.json`, Cargo.lock and other ecosystem
locks retain their native ownership and semantics.

## Selection and resolution

One `oyzu.lock` at the OEP-0002 workspace root contains selections for normalized
workspace-relative scopes and named profiles. Root scope is `.`; default profile
is `default`. Scope paths use `/`, have no absolute/drive/parent components and
resolve within the workspace after symlink resolution. A nested scope can lock a
different Node version without mutating its sibling's selection. Physical cwd
determines containment; a symlink outside the selected root is not a new parent.

Resolve the effective request map first using OEP-0002. Canonicalize aliases
through the pinned catalog; identity includes backend/publisher/name. Alias
changes cannot change policy scope. Use native constraints reported by builders
as additional constraints, not competing discovery by mise. Choose the nearest
locked environment whose scope is an ancestor of cwd and whose profile and
request digest exactly match the effective requirements. A nearer mismatching
entry is an error, never fallback to a farther convenient version. If no scope
entry exists, an ancestor can serve only an identical effective request digest.

Within a resolution transaction: validate all requests and admitted platforms;
obtain authorized candidate metadata; let the pinned backend interpret version
syntax; select the greatest permitted stable matching version unless the request
explicitly names a prerelease; reject incomparable/ambiguous backend versions;
compute dependency closure for every platform; intersect constraints; detect
conflicts/cycles; verify content identities; then propose a whole lock change.
No install happens while discovering whether the dependency graph is valid.
Exact versions stay exact. Tags and ranges are inputs to explicit resolution,
never interpreted again during frozen selection. A managed denial of an exact
request is not permission to substitute another version silently.

Limits: 8 MiB lock input, 1,024 environments, 4,096 tools, 64 distributions per
tool, 16,384 dependency edges, depth 64 and 256 roots per environment. Reject
duplicate TOML keys, unknown fields, invalid UTF-8, unsupported format, cycles,
unreachable records and ambiguous canonical IDs in a selected closure. Limits
are implementation constants; raising them needs a reviewed format-compatible
release, not a project override.

## Format-2 records

All digests are `sha256:` followed by exactly 64 lowercase hexadecimal digits.
Canonical JSON uses RFC 8785 with integers only for numeric fields. A digest is
SHA-256 of the indicated ASCII domain, a zero byte and canonical JSON. TOML
formatting, comments and array order do not affect semantic identity. Normalize
set-like arrays into sorted unique order before hashing; reject duplicates rather
than silently removing them. Do not normalize case of publisher/tool/version
strings beyond the pinned backend's documented canonicalization.

| Record / field | Type and meaning |
| --- | --- |
| Root `format` | Integer, exactly 2 |
| Root `environment`, `tool` | Arrays of the records below; no other root fields; an env-only project need not create a tool lock |
| Environment `scope`, `profile` | Contained path and OEP-0002 profile name; pair unique |
| Environment `request_digest` | Domain `oyzu.tool-requests.v2`; normalized canonical request map, native constraints and required tool capabilities; excludes env values, credentials and policy revision |
| Environment `roots` | Sorted tool-key list, empty only when requests are empty; full closure follows dependency edges |
| Environment `requests` | String map from canonical tool ID to literal effective request; exactly one request per root ID |
| Tool `key` | Domain `oyzu.tool-record.v2`; digest of `id`, `version`, `backend_digest` and normalized `options` only |
| Tool `id`, `version` | Canonical backend identity and exact upstream version strings |
| Tool `backend_digest` | Immutable reviewed backend compatibility descriptor identity; not merely `git:<mise commit>` |
| Tool `options` | String map of adapter-allowlisted identity-bearing options; empty table when absent; no URLs, local paths, code or credentials |
| Tool `distribution` | Nonempty array; exactly one per platform key |
| Distribution `platform` | Canonical `os/arch/abi`; initial OS linux/darwin/windows, arch amd64/arm64, ABI gnu/musl on Linux, native on Darwin, msvc on Windows |
| Distribution `digest`, `size` | Archive/package SHA-256 and exact positive byte count |
| Distribution `source_id`, `artifact_id` | Stable logical source and opaque artifact identity, each 1–256 printable ASCII characters without control characters; neither is a URL or filesystem path |
| Distribution `layout_digest` | Reviewed layout descriptor digest for the exact backend/version/platform |
| Distribution `verification` | Exactly `kind`, `evidence_digest`, `verifier_digest`, `subject_digest`; kind `digest-only`, `publisher-signature` or `attestation`; subject equals distribution digest |
| Distribution `dependencies` | Sorted exact tool-key array for this platform, including build/install/runtime tool dependencies needed by this distribution |
| Distribution `package_closure_digest` | Optional captured native package-closure digest; required for native package-manager tools and forbidden for single-archive descriptors |

The `backend_digest` hashes domain `oyzu.backend.v1` and the canonical descriptor
with fields `source_pin` (40 lowercase Git hex digits), `patch_set_digest`,
`embedding_abi` (positive integer), `adapter_revision` (positive integer),
`registry_digest` and `plugin_digest` (digest or null), `verifier_digest` (including
embedded trust roots), and `features` (sorted string set). Here `source_pin` is the exact consumed fork commit; `patch_set_digest` is the
SHA-256 of the patch register bound by its provenance manifest, as specified in
[upstream maintenance](upstream-maintenance.md). Binary/compiler identity
is additionally recorded in receipts.
A new backend descriptor does not silently reinterpret an old lock. A compiled
release can support an older descriptor only through its reviewed compatibility
manifest and old-descriptor conformance fixtures.

The lock's tool key deliberately excludes distributions: adding another platform
does not rename references in every environment. A **selection digest**, domain
`oyzu.tool-selection.v2`, binds target platform, selected root keys and all exact
selected tool/distribution records including content, layout, verification and
dependency edges. Thus changing an archive digest changes selection and
installation identity even when the tool key and version stay the same.

The exact canonical selection object is `{platform, roots, tools}` where tools is
sorted by key and each element is `{key, id, version, backend_digest, options,
distribution}` with only that platform's distribution. The installation-key
object is `{tool, distribution, dependencies}`: tool contains the five identity
fields `key`, `id`, `version`, `backend_digest`, `options`; dependencies is sorted
by dependency tool key and contains `{key, installation_key}` pairs. All optional
JSON identity fields use explicit null; TOML omission normalizes to that null
except `options`, which normalizes to `{}`. Array order is preserved for argument
vectors in launch descriptors, not treated as a set. Reject integers outside
0 through 9,007,199,254,740,991 in canonical records; individual field bounds still
apply. Hashes are never computed from serialized TOML text.

ABI is the installed tool ABI, not the frontend compiler. An experiment frontend
built with MinGW does not imply Windows GNU tool distributions. Unknown ABI tuples
fail. The initial production Windows frontend qualification uses MSVC; GNU remains
an experiment profile until separately admitted.

The following is a shape example; symbolic values are intentionally not valid
digests and must not be used as a real fixture:

```toml
format = 2

[[environment]]
scope = "."
profile = "default"
request_digest = "<request digest>"
roots = ["<node tool key>"]
[environment.requests]
"core:node" = "22"

[[tool]]
key = "<node tool key>"
id = "core:node"
version = "22.14.0"
backend_digest = "<backend descriptor digest>"
[tool.options]

[[tool.distribution]]
platform = "linux/amd64/gnu"
digest = "<archive digest>"
size = 1 # illustrative only
source_id = "node-distributions"
artifact_id = "node-22.14.0-linux-x64"
layout_digest = "<layout digest>"
dependencies = []
[tool.distribution.verification]
kind = "digest-only"
evidence_digest = "<evidence digest>"
verifier_digest = "<verifier digest>"
subject_digest = "<archive digest>"
```

A requested root must appear once in roots and resolve to a tool with the same
canonical ID. Each dependency key must exist and have a distribution for the
same target platform. Two versions of the same canonical tool in one closure
fail `TOOL_DEPENDENCY_CONFLICT`; separate environments can choose different
versions. Dependencies cannot be satisfied by ambient PATH. Different source IDs
claiming different bytes for the same tool/version/platform are a resolution
conflict unless a reviewed backend identity explicitly distinguishes the variant.

A package closure is a separate CAS record with `format = 1`, `manager_id`, exact
`manager_version`, `native_lock_digest`, `platform`, sorted `packages` and sorted
`edges`. Each package records canonical native name, exact version, native
integrity, SHA-256, byte size, logical source/artifact IDs and verification record;
edges bind exact package identities and native dependency kind. The closure digest
uses domain `oyzu.tool-packages.v1`. Capture platform variants required by the
native lock, preserve the native lock bytes as a separately verified blob, and
record which packages the target installed. No credential-bearing URL is retained.
This separates package dependencies from tool dependencies such as Node without
losing their content identity. Missing closure bytes/evidence prevent offline
installation. The receipt binds the closure digest and resulting installed tree.

Mutable mirror locations, bearer URLs, local ports, install paths, tokens,
authorization grants and timestamps never enter the lock. A source ID changing
physical routing does not change content identity, but still requires route
authorization. Verification evidence lives in a digest-addressed side store and
can be reacquired by logical reference; missing evidence prevents prepare when
the locked verification kind requires it. A lock is not an archive retention
guarantee. Exact byte disappearance is an availability error, not a relock.

## Lock editing and experimental migration

Writers preserve unaffected environment/tool records and comments using a lossless
TOML editor. Validate the entire resulting graph before publication. Capture the
original file digest, take a workspace lock, compare again, write a same-directory
temporary file, flush it, atomically replace, and sync the containing directory
where supported. A concurrent external edit yields `TOOL_LOCK_EDIT_CONFLICT`;
never overwrite it. Windows replacement retries sharing violations for at most
two seconds, then reports the conflict without deleting the original.

Combined `install` installs exact proposed content before publishing its lock.
A failed install leaves the prior lock unchanged. A successful store commit
followed by failed lock publication leaves a valid unreferenced store entry for
later reuse/prune. `lock` alone may publish verified identities without installing
trees. Both operations report a semantic diff. Frozen commands never write the
source lock, including platform expansion and backend upgrades.

Reject experimental format 1 with an actionable migration diagnostic. An explicit
`oyzu lock --migrate` reads it as input hints, displays the old/new resolutions,
revalidates requests and metadata under current policy and emits format 2 through
the same transaction. It must not attest existing experimental directories or
trust their `git:` backend field. No automatic migration on activation/build.
There is currently no shipped tool-lock format whose compatibility is promised.

## Store layout and committed receipts

Use the OEP-0002 user state root, with host-only configured relocation allowed
only before opening store handles. Managed shared stores require separate admin
provisioning; v1 defaults to per-user, never project-writable paths.

```text
tools/v1/
  blobs/sha256/<hex>           # verified acquisition/evidence bytes
  installs/<installation-key>/payload/
  installs/<installation-key>/receipt.json
  staging/<random-operation-id>/
  locks/<installation-key>
  leases/<lease-id>.json
  selections/<selection-digest>.json
  quarantine/<random-id>/
```

Installation key is domain `oyzu.installation.v1` over the tool's `key`, `id`,
`version`, `backend_digest`, `options`, the selected distribution and recursively
ordered dependency installation keys. Unselected platform records are excluded,
so adding a Darwin distribution does not invalidate an existing Linux install.
Source routing/policy is separately checked and is not mutable key material.
The locked logical source/artifact fields remain in the identity. Distinct
archive/backend/layout/dependency identities can never share a version directory.
Mise's expected directory layout is projected into operation-private state
pointing at leased payloads; never adopt a preexisting upstream cache as verified.

Receipt JSON has exactly: `format = 1`, `installation_key`, `tool_key`, `tool_id`,
`version`, `platform`, `backend_digest`, `distribution_digest`, `distribution_size`,
`layout_digest`, `verification` (the complete lock verification record),
`package_closure_digest` (null for single-archive installs),
`dependency_installation_keys`, `tree_digest`, `tree_manifest_digest`,
`entrypoints`, `environment`, `installer_release_digest`. Entrypoints are a sorted
map of command to typed relative launch descriptor; environment is the validated
backend delta using install-relative path references, not absolute machine paths
or secrets. Unknown fields/formats fail. Completion is defined by atomic publication
of the directory containing this receipt and payload, not an extra Boolean flag.

Tree manifest is a sorted array of relative UTF-8 paths, type (`file`, `directory`,
`symlink`), normalized executable permission bits, file size/content digest or
relative symlink target. Include directory entries; exclude timestamps, owner IDs
and Windows ACL serialization from portable tree identity. Domain
`oyzu.tool-tree.v1` hashes its canonical bytes. Reject special files, absolute or
escaping links, Windows ADS/device names, case-fold collisions and hardlinks to
outside objects. Legitimate internal symlinks remain explicit; where the host
cannot safely materialize them, reject that layout instead of silently copying.
Receipt authenticity alone never proves the current payload bytes.

## Transaction, validation and recovery

1. Validate the entire closure, platform/backend admission and current authority
   before acquiring bytes or invoking installer behavior. Topologically prepare
   dependencies. Acquire per-key OS locks in sorted key order when more than one
   is needed; never hold a parent lock while waiting for an earlier dependency.
2. Under the key lock, recheck for an existing valid receipt and tree. If valid,
   obtain a lease and return it. Mismatched receipt fields or bytes are an error;
   quarantine only after ensuring no active lease, otherwise deny new selection
   and retain the live directory until leases end.
3. Create staging on the same filesystem as installs. Download through the broker
   into bounded temporary files; verify exact size/digest and required provenance
   before extraction. A corrupt cached blob is evicted/quarantined, never retried
   against a public fallback. CAS publication is independently atomic.
4. Materialize the admitted layout. Extraction limits default to 200,000 entries,
   8 GiB expanded bytes, 1 GiB per file, depth 64 and expansion ratio 200:1;
   descriptor-specific reviewed bounds may be smaller or explicitly larger.
   Validate paths both lexically and using no-follow opened handles during writes.
   No path-based check followed by an unchecked reopen. No unapproved postinstall.
5. Generate tree/entrypoint/environment records; validate their containment and
   dependencies. In a qualified target runner, verify the declared version/layout
   tests for new admission. Runtime prepare need not execute an unknown binary
   merely to inspect it; content identity is the selection authority.
6. Write/flush receipt and payload in staging, sync as supported, publish by
   atomic directory rename without replacement, then sync parent. If another
   writer won, verify its committed receipt and discard only this operation's
   staging. Readers never select staging or a receipt without its payload.
7. Acquire a process lease before releasing the key lock. Release only when the
   supervising child tree/action finishes. Prune uses the same key lock and cannot
   race selection. Shell-session references protect active PATH directories.

Full tree verification is required before the first selection in a session and
before every managed build materialization. A verified snapshot may be reused
only while an OS change monitor covering the tree is healthy, no event/overflow
occurred, and its store generation is unchanged. Unsupported monitors use a full
scan; mtime-only caches are prohibited. Any change invalidates all dependent
selection snapshots. Standalone users may modify their own files, but modified
trees cease to be verified Oyzu installs. Against malicious same-user racing
processes, use a private verified materialization in the managed executor;
development PATH activation is not a same-user security boundary.

Recovery examines only contained store directories through trusted handles.
Staging has an operation record with owner PID/start time, creation nonce and OS
lock. Remove abandoned staging only after acquiring its lock and confirming the
owner is gone. Never recursively delete a computed unchecked path. Unknown or
malformed directories are quarantined with a diagnostic, not treated as installs.
After power loss, any missing receipt/blob/tree verification fails closed.

Prune retains entries referenced by registered workspace locks, active selection
sessions, running leases, dependencies or pending operations. Stale workspace
references are removed only by explicit forget or confirmed missing workspace,
not merely because its disk is temporarily unavailable. Windows sharing violations
leave a pending-prune record; retry on a later invocation. Release rollback keeps
old payloads and frontend releases while referenced. It does not reinterpret a
new lock with an old backend or weaken policy to make rollback appear successful.
