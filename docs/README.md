# Oyzu design library

This library consolidates the product direction discussed through 2026-10-01. **Visions and OEPs remain drafts; implementation is in progress.** Measured capabilities are recorded in [implementation status](implementation-status.md). Agreed product constraints are distinguished from proposed implementation choices in the [decision register](decisions.md).

Start with the [platform vision](vision/VIS-001-platform.md), then the [architecture](architecture.md), [example catalog](examples.md), and [proposal index](proposals/README.md). Use the [glossary](glossary.md) for consistent terms.

## Reading paths

| Goal | Documents |
| --- | --- |
| Understand the product and component boundaries | [Vision index](vision/README.md), [architecture](architecture.md) |
| Design tools, environment, shell switching, and tasks | OEP-0002 through OEP-0005 and OEP-0008 in the [proposal index](proposals/README.md) |
| Implement mise-backed tool management | [OEP-0003](proposals/OEP-0003-mise-integration/README.md): runtime, lock/store, acquisition/authorization and ordered acceptance work packages |
| Design discovery, execution, caching, and artifacts | OEP-0006, OEP-0007, OEP-0011 through OEP-0014 |
| Design managed clients, credentials, and distribution | OEP-0009, OEP-0010, OEP-0015, OEP-0016 |
| Turn designs into bounded implementation work | [Delivery plan](delivery.md), [examples](examples.md), [proposal process](proposals/OEP-0001-proposal-process/README.md) |
| Implement the build engine from detailed contracts | [Build implementation map and work packages](build-implementation.md), [record schemas](contracts/README.md) |
| Check decisions requiring review | [Decision register](decisions.md) |
| Find implemented behavior | [Reference status](reference/README.md) and [implementation status](implementation-status.md) |
| Extend an ecosystem adapter | [Builder code organization](builder-code-organization.md) |

## Document authority

Visions explain intended outcomes. OEPs contain proposed technical contracts and acceptance scenarios. Accepted OEPs will guide implementation; reference documentation will describe verified shipped behavior. An illustrative config or API payload is not a stable schema.

Every OEP has a stable ID, independent design/delivery status, dependencies, and a tracking-issue field. Null means no issue has been created. No GitHub issues, implementation, or approvals are implied by this document set.

The public repository holds all public client behavior and protocol contracts. Private platform implementation lives in its separate private repository; public contributors do not need it.

## Validation

Run `node tooling/check-docs.mjs` from the repository root to check local Markdown links, document identifiers, metadata, and proposal dependencies. This validates documentation structure, not product implementation. Also run `python tooling/check-build-contracts.py` with tooling/design-requirements.txt for the draft build-record schema fixtures; this checks structure and selected invariants, not build execution or trusted evidence.
