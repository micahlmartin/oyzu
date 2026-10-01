---
id: OEP-0019
title: Versioned build records and implementation conformance
status: draft
implementation: in-progress
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0006, OEP-0011, OEP-0012, OEP-0013, OEP-0017, OEP-0018]
tracking-issue: null
---

# Versioned build records and implementation conformance

> Implementation specification for review. Requirements and v1alpha1 choices remain draft; no maintainer acceptance, implemented support or verified security is implied.


## Purpose and authority

Provide typed storage boundaries for the build engine and public consumers without turning user configuration into a programming language. The checked-in [v1alpha1 schemas](../../contracts/README.md) describe build configuration, semantic plans, dependency snapshots, execution envelopes/events, finalized manifests, cache results and publication receipts. They are draft internal/public interchange contracts, not a stability promise or evidence that a record is true. No private platform implementation is needed to parse or validate them.

The schemas use JSON Schema 2020-12, reject unknown fields by default and reserve explicitly declared `extensions` maps for namespaced optional data. Unsupported schema identifiers fail before interpretation. A future stable version gets a new directory and explicit migration/compatibility suite. Never mutate a signed historic record to make it conform to a newer schema. IDs under `https://oyzu.example.invalid/schemas/` are logical identifiers resolved locally; the validator must not fetch schema code from the network.

## Canonical identities

Digest form is `sha256:<64 lowercase hex>`. Content blobs hash exact bytes. Semantic records use RFC 8785 canonical JSON after domain normalization: unordered collections sorted by documented stable ids, portable paths, integer counters, and no incidental execution data. Prefix hashed bytes with UTF-8 domain plus NUL: `oyzu.plan.v1alpha1`, `oyzu.action.v1alpha1`, `oyzu.dependencies.v1alpha1`, or `oyzu.tree.v1alpha1`. Prefixes prevent cross-type confusion. The [JSON canonicalization specification](https://www.rfc-editor.org/info/rfc8785/) governs serialization; ordinary JSON pretty printing is not a substitute.

Trees use sorted entries of path, type, digest/size or internal link target, and executable boolean. Directories are implied by paths with explicit empty-directory entries where semantically needed. Reject duplicate, escaping, invalid-Unicode and portable case-colliding paths. No timestamps, local absolute paths, UID/GID or mutable registry tags determine tree identity. Native source mode information is explicitly mapped to executable intent during capture.

Plan artifact references are symbolic `(target, variant, name)` until producers finish. The plan hashes those references and output contracts. The scheduler resolves actual input digests before computing each action key. Final manifest digests are computed only after all recorded bytes are finalized; external signatures avoid self-referential hashes. OCI manifest/index digests, compressed blob digests and unpacked tree digests occupy separately typed fields.

## Validation layers

1. Parse with duplicate-key rejection and bounded input size/depth; no network schema resolution.
2. Validate structural types and allowed fields against the exact schema id/version.
3. Validate unique ids, graph references/acyclicity, target/variant/output ownership, portable path containment, platform compatibility and final status invariants.
4. Verify every referenced byte/tree and compare expected digests before reuse, materialization or publication.
5. Verify identity/evidence signatures, authorization scope/freshness and required policy independently of the record's claimed fields.

Passing step 2 cannot be reported as a successful build, hermetic execution or trusted provenance. The documentation checker exercises schema shape and a small cross-record consistency set; the product must implement all layers. Synthetic fixture digests are labelled test identifiers and do not claim actual compiler output or signatures.

## Error and event contract

Diagnostic records contain code, phase, severity, safe message and optional target/action/field origin. Full native errors are redacted bounded attachments. Define stable categories for config, resolution, capability, authorization, integrity, execution, evidence and publication. Unknown codes remain failures and retain their category; consumers must not assume unknown means informational. Native process status is an integer alongside normalized action outcome, not the only outcome source.

Events are append-only run-scoped envelopes with schema version, run id, sequence, timestamp and typed payload. Sequence orders observations; timestamps never determine semantic identity. CLI JSON output is one JSON object per line, with native streams represented as bounded log references/events. A missing run-finished event means unknown/interrupted, never success. Consumers can reconstruct journal progress but only a finalized validated manifest defines bundle completion.

## Implementation module boundaries

Use a Rust workspace with logical modules first; split into crates when dependency boundaries justify it. `contracts` owns types/validation/canonicalization; `config` owns origins and policy constraint application; `discovery` owns native candidate/ownership records; `planner` owns graph/variants; `acquisition` owns adapters/broker sessions; `executor` owns isolation; `cache` owns verified result stores; `reports` owns native parsing; `bundle` owns journal/finalization; `publication` owns immutable transactions. CLI and agent call these libraries. No module reaches around acquisition to download dependencies, and no builder can issue a production signing token.

Builder implementation changes carry a content/version identity that changes affected computation keys. Public APIs use typed errors and cancellation; async operations have bounded concurrency/timeouts. Network and filesystem IO are injected interfaces for tests, not hidden globals. Do not implement a fake Oyzu binary that simply emits these example outputs.

## Acceptance scenarios

- RECORD-01: Every positive contract fixture validates locally and each negative fixture is rejected for its intended structural or semantic error.
- RECORD-02: Canonical hashes agree across independent implementations, map orderings and supported hosts; symbolic outputs require no invented pre-build digest.
- RECORD-03: Unknown versions/fields, duplicate ids, unresolved references, cycles, escaping paths and unfinished success records fail safely.
- RECORD-04: Mutated bytes or forged evidence cannot be made valid merely by schema conformance.
- RECORD-05: Journal recovery and publication receipts preserve historical origin and never mutate signed/finalized bundles.

## Delivery gates and open decisions

The [build implementation roadmap](../../build-implementation.md) breaks contracts into bounded slices with acceptance ids, examples, outputs and prerequisite experiments. Implement pure parsing/graph/hash logic before authenticating to production services. Native manager support, host isolation and registry/signing interoperability graduate only after their own tests pass.

Schema spellings and namespace identifiers are initial draft choices. Review them before a compatibility promise; backend libraries, runtime transport and production identity integrations remain measured release gates. Public license/mise import decisions remain outside this build-specification scope. Draft-ready contracts allow implementation work while these gates are resolved, but must not be represented as an accepted or production-ready system.
