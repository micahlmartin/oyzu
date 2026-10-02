## Purpose

Describe the problem and resulting behavior.

## Related work

Link the tracking issue and OEP, if applicable.

## Implementation approach

For code changes, name the owning subsystem and any interface or shared-behavior change. Briefly identify the AI tool used and how the generated change was directed and reviewed. Full prompts or chat logs are not required. See CONTRIBUTING.md for the AI-built workflow and current license status.

Review prompts (answer only where relevant): Does an existing subsystem already own this behavior? Does a new trait represent a real extension boundary? Can the change stay within the adapter and its registration, or does a shared contract need to evolve? Are unsupported capabilities explicit rather than successful no-ops? Check the [boundary review examples](../docs/code-organization.md#review-boundaries-in-practice); passing tests does not establish correct ownership.

For a new subsystem, link its module-level responsibility comment and code-map entry so reviewers can see its entry points and the responsibilities it delegates.

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
