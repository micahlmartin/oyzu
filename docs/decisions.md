# Decision register

Updated: 2026-10-01. This records product constraints agreed in discussion and technical choices proposed by these drafts. It does **not** mark an OEP accepted.

## Agreed product constraints

| ID | Constraint | Primary design |
| --- | --- | --- |
| DEC-001 | One CLI for humans and agents; deterministic operation, no LLM required | VIS-001 |
| DEC-002 | Conventional projects need no Oyzu build file; minimal explicit declaration is target + uses | OEP-0006 |
| DEC-003 | TOML for tools/env/tasks and local overrides; optional YAML for builds; no new programming language | OEP-0002 |
| DEC-004 | Builders infer native tasks and group them by target | OEP-0005 |
| DEC-005 | Same-name TOML tasks override implicit tasks; pre_/post_ hooks apply in direct runs and builds | OEP-0005 |
| DEC-006 | Tools resolve to an integrity-verifiable lock before build execution | OEP-0004 |
| DEC-007 | Capture inputs before constrained execution; no undeclared downloads or arbitrary host reads | OEP-0007 |
| DEC-008 | Every build emits a dist bundle and manifest when persistence is possible | OEP-0012 |
| DEC-009 | Versions/artifact identities and snapshot/release facts are resolved in planning | OEP-0013 |
| DEC-010 | Trust follows evidence and facts; no separate trusted/untrusted builder implementations | OEP-0013 |
| DEC-011 | Local-origin builds cannot become production artifacts under managed release rules | OEP-0013 |
| DEC-012 | OCI is the standalone remote-cache default; no custom cache server required | OEP-0011 |
| DEC-013 | Python, Go, Node, Rust, Docker, Helm, Java Maven/Gradle/Ant; single and multi-project builds | OEP-0014 |
| DEC-014 | Standalone CLI needs no login; the same CLI supports paid managed operation | OEP-0016 |
| DEC-015 | Protected machine management settings persist through logout; no public fallback | OEP-0002 |
| DEC-016 | No custom device-PKI enrollment prerequisite | OEP-0002 |
| DEC-017 | Integrate Artifactory/Nexus through connectors and policy | OEP-0010 |
| DEC-018 | Agent-to-upstream package traffic; no mandatory SaaS byte proxy or customer-hosted gateway | OEP-0009 |
| DEC-019 | Prefer scoped short-lived tokens; keep upstream credentials out of developer env/config | OEP-0010 |
| DEC-020 | Headless agent is a mode of the CLI binary; desktop optional | OEP-0015 |
| DEC-021 | Reuse mise in-process where appropriate; never invoke/bundle a separate mise executable | OEP-0003 |
| DEC-022 | Windows/macOS/Linux are host requirements; shell switching is a core capability | OEP-0008 |
| DEC-023 | No proactive universal mirror or OCI repackaging of all tools | OEP-0004 |
| DEC-024 | No developer-authored pipeline product; future CI derives work from builders and policy | VIS-006 |
| DEC-025 | Public client/contracts and separate private platform repository | OEP-0001 |
| DEC-026 | Consumer-owned materialize mappings select producer artifacts and place them in isolated input workspaces; dependency edges are inferred | OEP-0006 |
| DEC-027 | Consumer platform requirements propagate to runtime artifact producers; host, execution and target platforms remain distinct | OEP-0006 |
| DEC-028 | Platform matrix variants receive matching artifacts at identical logical paths in separate contexts; target test evidence remains explicit | OEP-0014 |
| DEC-029 | Every builder exposes an implicit test command, detects the existing framework or selects a documented standard default, and automatically collects JUnit and applicable code coverage in dist with manifest references; ordinary projects need no reporting configuration | OEP-0014 / OEP-0012 |

## Proposed choices needing review or experiments

| ID | Decision | Current proposal and next evidence |
| --- | --- | --- |
| OPEN-001 | Public license and contribution terms | Unselected; decide before upstream code import or substantive external contributions |
| OPEN-002 | Rust integration with mise | Audit a pinned source revision, choose reusable modules/fork boundary, preserve notices, benchmark switching |
| OPEN-003 | Configuration schemas and root/merge semantics | OEP-0002 proposes root YAML, nested TOML, whole-task replacement and `oyzu.lock`; prove with fixtures |
| OPEN-004 | Hook failure/argument rules | Success-only post, no recursive hooks, args to primary only; review TASK examples |
| OPEN-005 | Cross-platform hermetic executor | Validate Linux/macOS/Windows capabilities; do not silently fall back to host execution |
| OPEN-006 | Proxy endpoint authentication | Choose safe per-protocol local capabilities, project scoping, ports and TLS behavior |
| OPEN-007 | Supported enterprise backends | Validate all acquisition paths and upstream token capabilities; unsupported cases must fail clearly |
| OPEN-008 | Public protocol and manifest schemas | Semantic requirements drafted; versioned machine-readable schemas/conformance precede stability |
| OPEN-009 | OCI cache lookup/concurrency | Prototype registry interoperability, immutable candidates, conflict handling and retention |
| OPEN-010 | Version mappings and release evidence | Build ecosystem-valid fixtures and verified GitHub/GitLab identity/protection adapters |
| OPEN-011 | Stack and distribution | Rust core, React/TypeScript, optional Tauri proposed; packaging and license/dependency audit required |
| OPEN-012 | Offline management and revocation | Specify bounded validity windows, cached snapshot verification and fail-closed behavior |
| OPEN-013 | Initial implementation slice | Python/uv plus tool/env/task foundation proposed; retain all agreed ecosystems in roadmap |
| OPEN-014 | Remaining graph syntax | Materialize from/artifact/to and platform/platform-matrix contracts agreed; chart digest bindings and other matrix axes remain open |
| OPEN-017 | Default container target platform | Resolve omitted values visibly from defaults and managed policy; choose standalone default separately |
| OPEN-015 | Full hosted CI/CD and first-party registry | Future scope; event scheduler, runners, deploys and registry need separate proposals |
| OPEN-016 | Portable user commands | Define argv/shell portability and native hook ownership without a new language |

## Initial implementation resolutions for review

The [build implementation map](build-implementation.md) proposes concrete v1alpha1 resolutions without marking OEPs accepted. These replace open-ended implementation discretion while preserving the agreed constraints above.

| Open item | Proposed initial resolution |
| --- | --- |
| OPEN-003 | Root target-map YAML, scoped TOML origins, whole-task replacement, bounded strict parsing and no expressions; OEP-0002 implementation |
| OPEN-004 / OPEN-016 | Success-only nonrecursive hooks; argv for portable commands; explicit sh/PowerShell string semantics; extra CLI arguments require argv/native adapter |
| OPEN-005 | Linux capability-qualified executor first, Windows/macOS host transport qualification; native targets require separate gates, never host fallback |
| OPEN-006 / OPEN-012 | Job-scoped broker sessions and explicitly granted offline reuse; IPC/runtime implementation and cached grant verification are release gates |
| OPEN-008 | Draft v1alpha1 build schemas and positive/negative fixtures; no stability or signature-validity claim |
| OPEN-009 | Verified OCI candidate manifests and optional mutable lookup hints; bounded listing for observed conflicts, no global atomicity assumption |
| OPEN-010 | Native-source-v1 development versions and immutable publication receipts; real provider/signing interoperability remains gated |
| OPEN-014 | Finite native runtime axes and typed Helm image-reference bindings, specified in draft schema and candidate examples |
| OPEN-017 | linux/amd64 proposed standalone image default; explicit/managed target choice remains supported |

Private dependency builds must keep upstream credentials out of project/dependency execution and exported outputs across all supported managers (OEP-0017). New Dockerfile context bindings and convenience fields remain draft UX choices. Public license, mise import boundaries and backend qualification are not silently decided by this specification pass.

The [proposal index](proposals/README.md) resolves OEP IDs. Detailed open decisions inside each OEP remain part of its review; this register highlights choices spanning components.
