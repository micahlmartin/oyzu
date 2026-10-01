# Delivery and review plan

Status: proposed sequencing. No implementation work or GitHub issues have been created by this documentation change.

## Review before coding

First review the agreed constraints and open choices in the [decision register](decisions.md). Accept OEPs individually after blocking questions are resolved. Early prototypes can answer questions while an OEP remains draft; they must not silently freeze a public contract.

Immediate decisions are the public license, mise source integration boundary, and executable cross-platform isolation approach. Public schema stability and enterprise compatibility claims depend on evidence from those prototypes.

## Complete slices

| Slice | User outcome | Designs | Initial example families |
| --- | --- | --- | --- |
| 1. Tools and environment | Install/lock tools; switch projects; run headlessly on three hosts | OEP-0002/0003/0004/0008 | EX-001–005, EX-046–047 |
| 2. Tasks | Discover native/implicit tasks, groups, overrides and hooks | OEP-0005/0006 | EX-006–010 |
| 3. First complete build | Plan, prepare, isolate, test and bundle a conventional Python project | OEP-0006/0007/0012/0014 | EX-011–012, EX-036, EX-042 |
| 4. Ecosystem breadth | Go/Node/Rust/Java and all agreed Python manager variants | OEP-0014 and shared contracts | EX-013–025 |
| 5. Composition | Containers, Helm, multi-project graphs, generation and variants | OEP-0006/0007/0014 | EX-026–033, EX-044 |
| 6. Reuse and publication | OCI caching, versioning, evidence and retryable publishing | OEP-0011/0012/0013 | EX-034–038, EX-048 |
| 7. Managed workstation | Centrally selected tools/registries with local credential broker | OEP-0009/0010/0016 | EX-039–043 |
| 8. Optional desktop | Inspect/manage the same agent without affecting headless operation | OEP-0015 | EX-045 |

This order is illustrative, not a commitment to finish every ecosystem before a managed prototype. Shared risks such as sandboxing and credential routing should be prototyped early. Slice sizes must shrink into reviewable issues; none should become a months-long unreviewed branch.

## Issue ownership

Public client, builder, examples, desktop, and protocol issues belong in `micahlmartin/oyzu`. Private administration, identity, policy service, and hosting issues belong in the private platform repository. Cross-cutting work has linked issues with a public-facing contract described entirely in public.

A delivery issue names the accepted OEP section, observable outcome, included/excluded work, acceptance IDs, example fixtures, supported host matrix, and verification evidence. Track unresolved decisions as design work, not as implied developer discretion. IDs of actual issues are added to OEP metadata only after creation.

## LLM implementation contract

Each implementation task receives repository instructions, the relevant accepted OEPs, a narrow outcome, examples, and checks. Generated code must be reviewed against observable requirements. It cannot self-certify a design, choose a license, or mark a feature stable.

Small PRs explain the resulting behavior and relevant validation. New configuration needs a demonstrated example that cannot be handled by discovery or an existing simple override. Add current reference documentation when behavior is implemented; keep draft alternatives out of the user reference.

## Graduation gates

A slice is complete when meaningful acceptance tests pass, failure cases are covered, relevant OS behavior is demonstrated, secrets/provenance claims are accurate, and user reference matches implementation. Documentation checks alone never satisfy product tests.

A release declares exact supported builders/managers/hosts and known limitations. Future capabilities—remote execution, plugin distribution, hosted CI/CD, and a first-party registry—remain separate deliverables until proposals and examples establish their contracts.
