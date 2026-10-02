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

Make the finish line concrete before implementation:

- **First proof:** project/input, real command, expected artifact or usable result, and evidence location.
- **Functional completion:** the finite set of required user scenarios, including failures essential to correct behavior.
- **Deferred qualification:** additional hardening and its rationale, with no implied commitment to implement it in this milestone.

For every follow-up, identify which missing or failing functional scenario it closes. Otherwise place it in deferred qualification. Before the first proof, prioritize connecting the next missing part of the user flow over refining an already working component. The detailed design sections below are not an instruction to exhaust every edge case before demonstrating the proposal.

## Contracts and design

Define inputs, outputs, identities, state transitions, dependencies, compatibility, and ownership. Mark provisional syntax. Explain how overrides and hooks interact with engine-owned guarantees.

## Trust and platform behavior

Describe standalone/managed and local/CI behavior, identity and credential boundaries, offline operation, and Windows/macOS/Linux differences. State which capabilities are measured, assumed, or still need a prototype.

## End-to-end proof and implementation order

Define the first real user scenario from input through all essential integrations
to the resulting behavior. Specify the runnable command, actual dependencies,
controlled environment and observable pass/fail evidence. Identify the smallest
complete implementation that proves the proposed solution and the concrete
integration blockers. Record what actually ran and what remains unproven.

Make that proof the first implementation milestone. Separate safeguards required
to run it responsibly from later mandatory release hardening and optional
robustness improvements. Plan expansion to the full scope after the path works;
do not replace the intended integration with mocks or narrow the final acceptance
criteria. A list of tested components is not end-to-end evidence.

## Failure handling and performance

Describe cancellation, partial results, retries, concurrent operations, invalid data, cache behavior, and relevant resource bounds.

## Verification and examples

Assign stable acceptance IDs and link example catalog entries. Specify observable pass/fail outcomes and negative cases. Do not claim a test passed until it exists and ran.

## Rollout and alternatives

Explain dependencies, migration, support boundaries, alternatives considered, and graduation criteria. A small feature can combine sections if it still answers these questions.

## Open decisions

List decisions needing maintainer input or experiments. Separate agreement on product direction from acceptance of implementation details.
