# Documentation maintenance standard

Oyzu's documentation is a maintained description of the product. Every functionality change includes corresponding documentation in the same PR. This applies to CLI commands, configuration, builders, tasks, execution, acquisition, reports, artifacts, integrations and any future public platform or desktop surfaces. The agent or contributor implementing the change owns the documentation update; review checks both together.

## One home for each kind of information

| Information | Canonical home | Maintenance rule |
| --- | --- | --- |
| How current functionality works and how to use it | [Reference](reference/README.md) | Keep one owning page per coherent feature/subsystem, updated in place and linked from the index |
| Measured support, acceptance evidence and remaining gaps | [Implementation status](implementation-status.md) | Record the scope and revision of evidence; update current claims when support changes |
| Subsystem ownership and internal interface invariants | [Code map](code-organization.md), subsystem guides and comments beside interfaces | Update when responsibilities or contracts change |
| Proposed behavior and unresolved design | [OEPs](proposals/README.md) and [decisions](decisions.md) | Preserve draft/accepted distinctions; link to implemented reference rather than implying proposals are shipped |
| Getting started and navigation | [Repository README](../README.md) and [docs index](README.md) | Keep entry points and introductory examples consistent with reference |

Implementation status is the running capability/evidence record. Reference pages describe the current behavior without requiring readers to reconstruct it from historical checkpoints. Keep historical evidence attributable to its tested revision; add a correction or supersession note when an old limitation is resolved. Avoid duplicating detailed instructions across pages: link to the owning reference instead.

This standard governs this public repository. Document public protocols and client behavior here; private service implementation and operational details belong in their owning private repository. Do not copy private content to satisfy this standard or imply that this repository's instructions automatically govern another repository.

## Required feature coverage

Each owning reference page must let a reader use and troubleshoot the implemented feature. Cover the following where relevant, using sections, examples or tables suited to the feature:

1. **Purpose and status:** what it does, intended users, whether it is experimental or stable, and the boundary between available and deferred behavior.
2. **Prerequisites and support:** required native tools/services, supported versions and host/execution platforms, provisioning, permissions and account requirements. Distinguish standalone and managed operation.
3. **Usage:** a minimal working example, common variations, invocation directory assumptions and expected results. Label illustrative or unverified examples explicitly.
4. **Inputs and resolution:** command options, configuration keys, types, defaults, scopes, precedence, overrides and interactions. Explain validation and constraints that affect choices.
5. **Outputs and effects:** returned data, files and their locations, persistent state, network calls and changes to the workspace or environment. Explain evidence semantics when producing reports or artifacts.
6. **Failures and recovery:** important errors, what caused them, whether partial outputs/state remain and how to recover. Include cancellation, offline/outage and retry behavior where relevant.
7. **Trust and sensitive data:** relevant authorization, credential handling, redaction and execution boundaries. Explain what a success or integrity result does and does not establish.
8. **Compatibility and lifecycle:** behavior changes, migrations, deprecated options, replacement commands and removal guidance. Update stale examples and links together.
9. **Limits and evidence:** unsupported cases, known gaps and links to relevant tests, scenarios or measured implementation status. State exactly which behavior and platforms were verified.

Use these as a coverage checklist, not mandatory boilerplate headings. A small option change may require a paragraph and example in an existing page. A new subsystem generally needs a dedicated guide. Internal APIs need responsibility, input/output invariants, permitted effects and failure behavior beside their definition, with a code-map update when their boundary changes.

## Workflow for every change

1. Before implementation, find the owning reference page and read its current promises. Identify which sections, examples and related entry points the change affects.
2. Implement and verify the behavior. Update the reference alongside the code, including negative paths and compatibility implications. For a new feature, create its page and link it from the reference index.
3. Update implementation status when measured capabilities, acceptance evidence or limitations change. Update design decisions and architecture/interface docs only where the change affects them; do not mark a draft accepted without maintainer review.
4. Search the repository for superseded commands, settings, defaults and limitations. Correct current instructions; preserve historical evidence with an explicit supersession note when appropriate.
5. Exercise affected runnable examples with provisioned dependencies when available. Report exactly what ran; structural validation cannot establish example behavior or platform support. Unavailable prerequisites remain an explicit verification limit.
6. Run `node tooling/check-docs.mjs` and `git diff --check`. Review the actual prose against the implementation: link/metadata checks cannot detect every missing feature or stale statement.
7. In the PR, link updated reference and status/interface pages, identify verification and any remaining documentation gaps. A functionality change is not complete until this documentation is current.

An internal-only refactor may have no user-reference impact. Explain the specific reason in the PR, and still update interface/code-map documentation if responsibilities changed. A bug fix that changes observable behavior requires checking and updating its documented behavior or limitation; "bug fix" is not a blanket exemption.

## Existing documentation gaps

This requirement applies immediately to new and changed functionality. It does not certify that every older feature already has a detailed guide. When touching an older feature, bring its relevant documentation up to this standard. Record unrelated discovered gaps in implementation status with the feature, owning subsystem and missing coverage. Do not replace missing operational documentation with a link to an unimplemented proposal or claim a complete documentation inventory without auditing it.
