# Cache storage and concurrency implementation contract

## Key and result model

Compute action keys only after prerequisite outputs are available. Hash canonical recipe, input tree digests, dependency snapshot digests, locked tool identities, builder/adapter digests, declared environment, execution/target ABI, sandbox semantics and computation-changing policy requirements. Secret values, authorization expiry, host paths and run identifiers never enter keys. Names of required secret scopes enter noncacheable action metadata; they do not make the action safely cacheable.

A result descriptor contains format, action key, output tree references, report references, actual isolation properties and producer evidence references. A receipt of reuse records the current run separately. A report can be reused only if the required check's freshness and producer constraints permit it. Cache hits do not claim a test or scanner executed again. Failed/cancelled/incomplete results are never uploaded as reusable successes; diagnostics may be retained locally without a hit locator.

Local CAS commits verified blobs by digest under a process lock; result records are atomically published after every referenced blob is committed. Store leases protect active readers from GC. Check compressed and uncompressed digests; enforce unpack limits and contained paths. A cache hit materializes private writable worktrees where needed, never writable references to the shared CAS.

## OCI wire representation

Use OCI image manifests with `artifactType: application/vnd.oyzu.action-result.v1alpha1`, a result JSON config blob with media type `application/vnd.oyzu.action-result.config.v1alpha1+json`, and zstd-compressed canonical tree archives as layers with media type `application/vnd.oyzu.tree.v1alpha1.tar+zstd`. These vendor types are draft interoperability identifiers, not a registered standard. Archives sort paths, normalize metadata and retain safe executable/symlink semantics. OCI layer digests address compressed bytes; tree descriptors also bind logical uncompressed tree identity. Do not conflate image digests, archive digests and tree digests.

A configured repository prefix is sharded by the first two action-key hex characters: `<prefix>/v1alpha1/<shard>`. Each candidate has an immutable-by-convention tag `k-<64-hex-key>-<first-40-hex-result-manifest-digest>` (107 characters). The complete manifest digest is verified after lookup; detect truncated-suffix collision and fail. Optional `lookup-<64-hex-key>` is a last-writer hint only, never an authoritative result index. Writers cannot assume the registry enforces tag immutability.

Read the hint for ordinary fast reuse, verify content and producer authorization. For strict reproducibility comparison, enumerate all candidate tags with the exact key prefix using bounded paginated tag listing, then verify every eligible candidate descriptor. Identical output trees converge; differing outputs for the same computation key are a conflict and no candidate is silently selected. Require both the necessary API capability and a complete bounded enumeration before claiming a comparison; exceeding the scan bound yields a miss/diagnostic, not false certainty. A concurrently uploaded result can appear after enumeration: this is an observed-conflict check, not global serializable consensus. Release authorization must not depend on a registry tag list proving uniqueness.

Upload blobs, config and result manifest first; publish candidate tag only when all are retrievable and verified; publish the optional hint last. Read-after-write verifies candidate identity and detects replacement when observed. No lock service or conditional tag update is required. A registry lacking listing still supports validated hint reuse; strict candidate comparison is unsupported there. Referrers are optional, never a universal prerequisite. Connector capability probes cover media types, push/pull, nested repository naming, tag listing/pagination, size limits and authentication.

## Access, failures and retention

Standalone defaults: local read/write on; configured remote read on, remote write off until explicitly enabled. Managed rules can set separate local-development and CI repository scopes. Server/upstream authorization is checked independently of client flags. A local-origin candidate never gains production provenance because it shares content with CI.

HTTP timeout/unavailable cache → compute if inputs/executor are available. Permission denied → report denied cache route and compute only if policy permits the build without that cache; never try another public route. Invalid digest → quarantine local data and reject candidate, then compute if permitted. Evidence violation → miss unless policy requires that evidence source, in which case deny. Limit transfers to 4 in parallel, at most 3 attempts for explicitly retryable network failures with exponential backoff and jitter; do not retry authentication denial blindly.

Local GC is mark-and-sweep from active run leases, installed-tool references and retained bundles. Default budget is 10 GiB for expendable action cache, with explicit user override. Do not delete installed tools or published bundle references to meet this budget. Remote retention is registry/operator policy; the CLI prunes only its explicit authorized cache namespace and never shared release repositories automatically. No durability promise for cache content.

## Verification and rollout

CACHE-01–06 plus two writers, crash before tag creation, replaced hint, forged producer, incomplete pagination, changed output for same key, private-content authorization, report freshness and independent GC leases. Interoperability must be demonstrated against a reference OCI registry and at least one Artifactory/Nexus configured route before support claims. Performance prototypes measure request count, candidate-list scaling and compression cost. The implementation may optimize lookup later while retaining immutable verified result identities.

The storage design uses the [OCI Distribution 1.1.1 API](https://github.com/opencontainers/distribution-spec/blob/v1.1.1/spec.md); support categories are capability-tested rather than assumed from the word OCI.
