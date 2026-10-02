# Proposal template

Copy this into `OEP-draft-short-name/README.md`. A maintainer allocates the permanent number before merge. Replace every placeholder; include only configuration needed to solve the problem.

```yaml
---
id: OEP-draft
title: Short descriptive title
status: draft
implementation: not-started
updated: YYYY-MM-DD
authors: []
reviewers: []
requires: []
tracking-issue: null
---
```

## Problem and intended outcome

Who needs this and what observable behavior changes? State goals and exclusions.

## User experience

Show the zero-config path first, then the smallest necessary exception. Explain discovery and errors. Do not introduce a pipeline DSL or repeat native project metadata.

## First end-to-end proof and delivery order

Name the representative project/input, actual user command, subsystems traversed and usable final result that will demonstrate the proposal. Identify the smallest connected implementation needed to run it and the observable pass/fail check. Use real product behavior and native tools; a mocked result or isolated component test is not this proof.

Separate the first demonstration, remaining required end-to-end scenarios and later hardening. Give each milestone finite acceptance criteria. Record additional edge cases, compatibility expansion, performance and resilience refinements as follow-ups; do not make exhaustive hardening a prerequisite to proving the solution. Preserve existing safety boundaries and explicit requirements. Link measured evidence when available and state limitations; the first demonstration does not imply stable support.

State the functional implementation completion criteria separately from production graduation. Completion of those functional checks ends the implementation goal; manual testing and maintainer evaluation precede selection of any later hardening scope.

## Contracts and design

Define inputs, outputs, identities, state transitions, dependencies, compatibility, and ownership. Mark provisional syntax. Explain how overrides and hooks interact with engine-owned guarantees.

## Trust and platform behavior

Describe standalone/managed and local/CI behavior, identity and credential boundaries, offline operation, and Windows/macOS/Linux differences. State which capabilities are measured, assumed, or still need a prototype.

## Failure handling and performance

Describe cancellation, partial results, retries, concurrent operations, invalid data, cache behavior, and relevant resource bounds.

## Verification and examples

Assign stable acceptance IDs and link example catalog entries. Specify observable pass/fail outcomes and negative cases. Do not claim a test passed until it exists and ran.

## Rollout and alternatives

Explain dependencies, migration, support boundaries, alternatives considered, and graduation criteria. A small feature can combine sections if it still answers these questions.

## Open decisions

List decisions needing maintainer input or experiments. Separate agreement on product direction from acceptance of implementation details.
