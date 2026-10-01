# Bundle and report implementation contract

## Lifecycle and transaction

Reserve an output destination lease, create a run journal outside source, persist the frozen plan and append action events. Append records have monotonic sequence ids and bounded payloads; credential-bearing output is redacted before persistence. A handled failure finalizes an accurate failed bundle; a storage failure returns a persistence error and never claims a manifest exists. Crash recovery labels an unfinished journal interrupted and may finalize available evidence without reclassifying work as succeeded.

Export into a sibling temporary destination on the same filesystem, verify all files, write manifest last, fsync where supported and atomically select dist under the lease. On systems without replacement of nonempty directories, use a transaction record plus old/new rename sequence and recover deterministically. A previous recognized bundle may be archived; arbitrary user-owned dist contents cause a destination conflict. Concurrency never mixes two runs. Finalized bundles are immutable; do not append publication or signing metadata inside them.

Layout: `manifest.json`, `plan.json`, `envelope.json`, `artifacts/<target>/<variant>/<name>/...`, `reports/<target>/<variant>/<check>/...`, `evidence/...`, and bounded redacted `logs/...`. Paths are portable relative paths validated under OEP-0007. Stable variant ids are canonical axis values hashed to a 16-hex suffix with collision detection; display the full axis map in the manifest. Content paths are assigned by the engine, independent of native target/dist/build directory conventions.

## Record contract

The draft manifest schema is [v1alpha1](../../contracts/v1alpha1/manifest.schema.json). It records runId, plan digest/reference, envelope path/digest, status, source snapshot, targets, actions, artifacts, reports, evidence references and diagnostics. IDs are unique within their typed collection. Every producer/action/target reference must resolve. Artifact outputContract identifies the planned output; path/size/digest bind actual bytes. Directory artifacts use the canonical tree identity and a complete file manifest, not a hash of filesystem enumeration order. OCI artifacts record both exported OCI-layout tree and registry manifest/index digest, because those are different identities.

Each action result records whether it was mandatory, attempt number and duration where known; informational optional-check failures remain visible without necessarily failing the run. Native build/test tasks default mandatory when selected. Action statuses are pending, running, succeeded, cached, failed, cancelled, blocked and skipped. A finalized manifest cannot contain pending/running. Skipped requires a reason such as outside-selection; blocked identifies failed prerequisites. Cached records retain original producer evidence and current reuse decision. Run statuses are succeeded, failed, cancelled and interrupted. Success requires every selected mandatory action/check to be satisfied under policy; artifacts existing on disk is not sufficient.

Failures before planning use null planDigest/planPath and, if capture failed, null source. They have no final execution actions/artifacts and include the preflight diagnostic. A successful bundle requires a frozen plan and source identity. Never fabricate a digest to satisfy a schema. The execution envelope can similarly record an absent plan before planning; finalization binds the final available envelope bytes. The run-started event identifies invocation context without claiming a plan/envelope digest is already final.

Plan digests and output digests exclude the manifest. The manifest is hashed only after finalization; an external signing envelope binds that digest. Keep original serialized manifest bytes for signature verification. A rewritten or migrated manifest is a new artifact and must not inherit an old signature.

## Reports and normalization

Each report has kind, native format, producer action, target/variant, collection status, original bytes digest/path and normalized summary. Collection statuses: collected, not-detected, unsupported, disabled, missing, invalid. Test outcomes within collected reports distinguish passed, failed, skipped and zero-collected. A runner exit failure remains a failed action even if some tests report passed. JUnit readers disable external entities/DTD, cap input size/nesting and preserve duplicate case names with suite and occurrence identity.

Normalize test totals with integer counts; retain raw formats for details. Coverage records covered/total counts per metric and source mapping completeness, not only rounded percentages. Gate comparisons use exact integer fractions; empty denominators mean unavailable, not 100 percent. Merging requires matching source identity, metric semantics and a known adapter; never average percentages across modules or sum duplicated source coverage from matrix runs. Show each variant separately; an optional combined view is not production evidence unless its merge rules are recorded.

Automatic reports come from a pinned adapter that configures native supported flags/plugins. Examples: pytest JUnit, Go test JSON with a pinned converter, Maven/Gradle native test XML, supported Node reporters and explicit Rust reporter integrations. A missing converter is prepared as a tool dependency. Unknown custom runner output is not parsed heuristically into a success report. Overrides inherit required check intents and must emit a supported report or fail the evidence requirement.

A report must bind the exact output/source/action it claims to assess. Scanner findings additionally record scanner version, rules/database identity, scope and scan time. A clean report from another artifact digest cannot satisfy the check. Default retention includes safe original reports and normalized summaries; full logs are bounded to 16 MiB per action by default with truncation metadata. Policy can reduce exposure or require durable external evidence without inserting credential-bearing URLs into the manifest.

## Consumers and verification

`oyzu inspect <bundle>` validates schema, references, paths and digests and summarizes outcome without running code. `oyzu publish <bundle>` performs the same verification plus current authorization. A failed bundle can be inspected and retained; publication eligibility is evaluated per requested artifact closure and managed production rules require complete relevant evidence. No local upload converts its origin.

BUNDLE-01–06 plus zero tests versus no tests, malformed/hostile XML, coverage denominator zero, duplicate names, matrix source collisions, incomplete journal recovery, post-hook failure with outputs, OCI digest versus archive digest, signature-envelope circularity and concurrent destination selection. Machine-readable schemas check shape only; filesystem integrity and evidence truth require executable consumer checks.
