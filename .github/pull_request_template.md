## Purpose

Describe the problem and resulting behavior.

## Related work

Link the tracking issue and OEP, if applicable.

## Implementation approach

For code changes, name the owning subsystem and any interface or shared-behavior change. Briefly identify the AI tool used and how the generated change was directed and reviewed. Full prompts or chat logs are not required. See CONTRIBUTING.md for the AI-built workflow and current license status.

Keep this proportional to the change. Address these questions where relevant:

- **Ownership:** Which subsystem owns the rule, and what existing behavior is reused? For a new subsystem, link its responsibility comment and code-map entry.
- **Boundary:** Which contract or dependency between subsystems changes, and which implementations and consumers are affected? Explain why any new abstraction or dependency direction is needed; shared extractions should remove superseded copies.
- **Evidence:** Which observable behavior and failure cases verify the change? Report the actual checks below, including native integration and platform coverage where applicable.

Use the [engineering guide](../docs/code-organization.md#review-boundaries-in-practice) for detailed review examples. Routine fixes do not require a new design document or an answer to every architectural question.

## Validation

Describe checks performed and relevant results.

## Compatibility and risks

Describe affected contracts, platform differences, and unresolved limitations.

## Documentation

Link the updated feature reference and applicable status/interface pages. For an internal-only change with no documentation impact, explain the specific reason. Observable behavior changes require corresponding documentation in this PR; see `docs/documentation.md`.

- [ ] Current usage, defaults, outputs, errors/recovery and compatibility limits match the implementation.
- [ ] New reference pages are indexed; affected examples and superseded instructions are updated.
- [ ] Measured capabilities and remaining gaps are recorded without treating draft designs as delivered behavior.
- [ ] Documentation checks passed, and affected examples were exercised or their verification limits recorded.
