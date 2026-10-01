# Glossary

| Term | Meaning |
| --- | --- |
| Oyzu | The unified developer tool and optional enterprise platform. The executable is `oyzu`. |
| Builder | Versioned ecosystem knowledge that discovers projects and supplies tasks, preparation, outputs, and reporting; for example `python/app`. |
| Target | A named project/build instance, for example `api`. It forms a task group. |
| Task | A named developer operation, implicit, native, or explicit; for example `api:test`. |
| Hook | An optional `pre_` or `post_` task attached by name to a primary task. Post is success-only in the proposed semantics. |
| Action | A planned execution unit with declared inputs/outputs and execution properties. One task can yield multiple actions. |
| Engine guard | Non-overridable integrity, isolation, authorization, or evidence validation enforced outside task bodies. |
| Preparation | Approved acquisition and capture of tools/dependencies before isolated action execution. |
| Plan | The resolved graph and intended artifact identities from captured inputs and policy. |
| Bundle | The portable `dist/` output containing a manifest, artifacts, reports, and evidence references. |
| Manifest | The record of actual build outcomes, including failures and cached origins. |
| Artifact | A digest-identified output such as a binary, package, image, or chart. |
| Evidence | Verifiable observations about source, execution, checks, and artifact origin. |
| Attestation | A statement bound to subjects and a producer; trust requires verification and authority, not just JSON. |
| Connector | A system/endpoint/repository/credential-strategy definition with declared capabilities. |
| Catalog | Metadata identifying available tools/versions/distributions; availability does not itself grant authorization. |
| Lease | A time- and scope-bounded authorization/credential grant. |
| Agent | Headless background mode of the same CLI binary, owning local routing and credential brokerage. |
| Desktop | Optional native UI that communicates with the agent. |
| Standalone | Operation without mandatory enterprise management; no account required. |
| Managed | Operation bound by protected organization settings independently of login state. |
| Policy | Centrally distributed defaults and mandatory constraints in the managed product. OSS uses local configuration without requiring a policy service. |
| Profile | A convenient selection of behavior; it cannot assert trust or bypass policy. |
| Trusted execution evidence | Verified properties and identity of a particular execution, not a separate implementation of the builder. |
| Release eligible | Authorized for a specific production operation after evidence evaluation. A version or protected branch alone is insufficient. |
| Snapshot | A nonrelease artifact identity/classification following ecosystem conventions. |
| Promotion | Publishing or selecting the same verified artifact for another authorized destination; not rewriting origin. |
| OCI cache | Action-result/output storage using OCI registry objects; distinct from publishing final product images. |
| Hermetic action | An action constrained to declared inputs/environment without undeclared network or host reads. This alone does not prove reproducibility. |
| Reproducible build | A build whose declared reproducibility claim is demonstrated by repeated equivalent inputs and outputs. |
