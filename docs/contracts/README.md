# Draft build record schemas

OEP-0003's tool contracts are a separate draft family from the build records below.
They use their own format fields and must not be treated as stable public APIs.

| Tool schema | Current implementation and limits |
| --- | --- |
| [Normalized tool requests](tools-v1/request-identity.schema.json) | Closed identity record with shared schema/Rust malformed-shape fixtures. Sorted sets, UTF-8 byte budgets, catalog membership and recomputed digest are runtime checks; this is not a complete worker resolve payload. |
| [Worker outer envelopes](tools-v1/worker-envelope.schema.json) | Closed request/result/cancel envelopes with Rust response correlation and supervisor-side lifecycle checks; operation-specific payload schemas and worker dispatch remain absent. |
| [Archive layout](tools-v1/archive-layout.schema.json) | Data-only candidate finalization for tar, tar.gz, raw single-file artifacts and bounded ZIP32; optional prefix removal, dot payload subtree and explicit Unix executable paths. The broader schema vocabulary includes tar.xz, which the runtime currently rejects. |
| [Receipt](tools-v1/receipt.schema.json) | Candidate receipt parsing and whole-selection/content verification, including exact locked identities and dependency ownership. A matching receipt does not authorize execution. |
| [Selection grant](tools-v1/selection-grant.schema.json) | Draft payload shape, shared synthetic fixtures and initial Rust signature/context/expiry verification; authenticated agent integration, issuer and execution authority remain absent. |
| [Backend descriptor](tools-v1/backend-descriptor.schema.json) | Bounded descriptor parsing and canonical identity inspection. A structurally valid descriptor is not compiled backend admission or legal approval. |

Run `python tooling/check-tool-contracts.py` with Python 3.11+ and the pinned
`tooling/design-requirements.txt` dependencies. It validates shared valid/invalid
shape fixtures with local-only schema references and duplicate-key rejection.
Whole-string patterns use an absolute end assertion: a trailing newline cannot
make an invalid UUID, digest, target, capability, diagnostic code or path pass.
This does not prohibit newlines in literal values whose grammar allows them.
Rust tests also exercise the relevant fixture corpora, then enforce semantic
requirements the schemas do not express: sorted sets, portable path components,
UTF-8 byte bounds, locked identity binding, content integrity and filesystem rules.
The schema checker performs no downloads, materialization or execution and cannot
qualify a backend. Complete operation-specific worker payload schemas remain outstanding. Grant runtime tests
cover synthetic signatures, bindings, expiry and shared shape fixtures; agent
lifecycle notifications and service interoperability remain unimplemented.

See the [tool reference](../reference/tool-lock-inspection.md) for runtime limits,
usage, failures and measured qualification. In particular, archive expansion
defaults to 200:1; an explicit caller-admitted, identity-bound layout may request
up to 1024:1. Passing schema validation does not admit that layout or grant a
project permission to raise extraction bounds.

These JSON Schema 2020-12 documents are proposed **v1alpha1** contracts, not stable APIs or proof of correct execution. They accompany [OEP-0019](../proposals/OEP-0019-build-records-and-conformance/README.md).

| Schema | Purpose |
| --- | --- |
| [build](v1alpha1/build.schema.json) | Optional build.yaml parsed as data; minimal target + uses remains valid |
| [plan](v1alpha1/plan.schema.json) | Frozen semantic graph with symbolic output references |
| [dependencies](v1alpha1/dependencies.schema.json) | Credential-free captured dependency graph and native-store identity |
| [envelope](v1alpha1/envelope.schema.json) | Run identity, host, timed facts and authorization references outside semantic hashes |
| [event](v1alpha1/event.schema.json) | Typed append-only CLI/journal event envelopes |
| [manifest](v1alpha1/manifest.schema.json) | Finalized build outcomes, artifacts, reports and evidence references |
| [cache](v1alpha1/cache.schema.json) | Action result with verified content references and original producer evidence |
| [receipt](v1alpha1/receipt.schema.json) | Separate publication/signing operation results |

Every fixture under v1alpha1/fixtures is synthetic. Repeated-character digests and sizes are shape-validation identifiers, not hashes of real build outputs. They must not be published as provenance. The fixture suite checks structure and a limited set of semantic invariants, never filesystem integrity, authorization, signatures or sandbox behavior.

Run `python tooling/check-build-contracts.py` using Python 3.11+ and the pinned dependencies in tooling/design-requirements.txt. Schema resolution is local only. The product additionally implements all validation layers in OEP-0019. Build schema changes that affect the existing examples require reviewing their status; proposed new fields do not silently become accepted product constraints.

The implementation currently records explicit application-source coverage inapplicability as a target extension: `oyzu.dev/coverage-applicability` contains `status: "inapplicable"` and a nonempty human-readable `reason`. It is bound by the plan digest and retained on the manifest target. It has no report path, digest, percentage or invented denominator. An absent extension is unspecified, not proof of inapplicability. This initial typed fact does not replace the still-required complete coverage outcome/producer-reference contract in OEP-0012 and OEP-0014.

Materialization receipt inputs now carry `extensions.oyzu.dev/producer-reports`, containing `scope: "producer-target"` and original report references (`report`, `kind`, `digest`, `subjectDigest`). The surrounding receipt identifies the copied artifact and its digest. These are links to original measurements, not new consumer reports or assertions that a target-wide metric describes only the selected binary. Empty references mean no eligible collected evidence was linked. The evidence file itself is content-bound by the manifest.

Directory manifest artifacts now require `entries`, a complete sorted file/directory inventory, and bind its logical tree digest and total file-byte size. The current tree encoding includes all directories explicitly; source exclusions do not apply. File/OCI artifacts cannot carry this field. See [directory artifact integrity](../reference/directory-artifacts.md) for the exact encoding, limits and current implementation scope. Synthetic fixtures validate shape and selected invariants; they remain separate from filesystem integrity tests.
