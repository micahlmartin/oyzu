# Draft build record schemas

OEP-0003's separate [archive layout schema](tools-v1/archive-layout.schema.json)
defines the initial tool finalizer record. Its shared shape fixtures run through
`python tooling/check-tool-contracts.py` (the same pinned design requirements)
and the Rust finalizer tests. The schema is a draft and checks shape only; the
[finalizer contract](../reference/tool-lock-inspection.md#data-only-candidate-finalization)
describes semantic checks, implementation limits and admission responsibilities.

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
