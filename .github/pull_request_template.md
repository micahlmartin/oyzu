## Purpose

Describe the problem and resulting behavior.

## Related work

Link the tracking issue and OEP, if applicable.

## Implementation approach

For code changes, name the owning subsystem and any interface or shared-behavior change. Briefly identify the AI tool used and how the generated change was directed and reviewed. Full prompts or chat logs are not required. See CONTRIBUTING.md for the AI-built workflow and current license status.

Review prompts (answer only where relevant): Does an existing subsystem already own this behavior? Does a new trait represent a real extension boundary? Can the change stay within the adapter and its registration, or does a shared contract need to evolve? Are unsupported capabilities explicit rather than successful no-ops? See the [engineering guide](../docs/code-organization.md).

## Validation

Describe checks performed and relevant results.

## Compatibility and risks

Describe affected contracts, platform differences, and unresolved limitations.
