# Oyzu examples: the design contract

These are the concrete projects and scenarios we will review to decide **what Oyzu should discover, what the user should write, and what should happen**. They are checked into this repository before the implementation.

Start with zero-configuration projects, then exceptions and composition. Configuration must earn its place: native project metadata should supply facts we already know. Expectations outside a project are review material, never required configuration for its user.

## Suggested review order

1. [Python library](builds/python-uv-library/README.md), [Python API](builds/python-api/README.md), [Go app](builds/go-app/README.md), [Node package](builds/node-package/README.md), [Rust app](builds/rust-app/README.md): does the zero/minimal-config experience match the product?
2. [Implicit tasks](tasks/go-implicit/README.md), [native scripts](tasks/npm-scripts/README.md), [groups](tasks/grouped-targets/README.md), [overrides and hooks](tasks/overrides-and-hooks/README.md): are discovery and replacement rules intuitive?
3. [Tool installation](tools/install-and-lock/README.md), [switching](environments/switch-projects/README.md), [overrides](environments/local-overrides/README.md), [headless use](environments/headless-exec/README.md): is environment ownership clear?
4. [Maven](builds/java-maven-reactor/README.md), [Gradle](builds/java-gradle-multi-project/README.md), [Ant](builds/java-ant/README.md), [workspaces](builds/node-workspace/README.md), [mixed repo](builds/mixed-monorepo/README.md): are native relationships preserved?
5. [Containers](builds/docker-offline/README.md), [Helm](builds/helm-chart/README.md), [image/chart composition](builds/image-and-chart/README.md), [matrix](builds/compatibility-matrix/README.md), [generation](builds/generated-source/README.md): which exceptions truly need syntax?
6. [Caching](cache/oci-standalone/README.md), [release eligibility](release/eligibility/README.md), [managed tools](managed/tool-catalog/README.md), [package proxy](managed/package-proxy/README.md): are trust and failure behavior explicit?

7. [Private dependency acquisition](DEPENDENCIES.md): do all package managers preserve the same credential boundary, including container packaging and custom Dockerfiles?

## All scenarios

| ID | Example | Area |
| --- | --- | --- |
| EX-001 | [Install a tool and preserve its identity](tools/install-and-lock/README.md) | tools |
| EX-002 | [Switch between independent tool environments](environments/switch-projects/README.md) | environments |
| EX-003 | [Checked-in defaults and local overrides](environments/local-overrides/README.md) | environments |
| EX-004 | [Execute without shell hooks or a desktop](environments/headless-exec/README.md) | environments |
| EX-005 | [Untrusted repository tasks do not run on entry](environments/repository-trust/README.md) | environments |
| EX-006 | [Implicit Go tasks without Oyzu configuration](tasks/go-implicit/README.md) | tasks |
| EX-007 | [Native npm scripts and their lifecycle hooks](tasks/npm-scripts/README.md) | tasks |
| EX-008 | [Qualified tasks in a mixed target build](tasks/grouped-targets/README.md) | tasks |
| EX-009 | [Replace an implicit task and attach hooks](tasks/overrides-and-hooks/README.md) | tasks |
| EX-010 | [Hooks around cached computation](tasks/cached-hooks/README.md) | tasks |
| EX-011 | [A Python library with uv and automatic reports](builds/python-uv-library/README.md) | builds |
| EX-012 | [A Python HTTP API with minimal builder intent](builds/python-api/README.md) | builds |
| EX-013 | [Requirements-based application](builds/python-pip/README.md) | builds |
| EX-014 | [Poetry-managed package](builds/python-poetry/README.md) | builds |
| EX-015 | [Legacy setup.py package with a C extension](builds/python-legacy-native/README.md) | builds |
| EX-016 | [A zero-configuration Go application](builds/go-app/README.md) | builds |
| EX-017 | [Go workspace with an explicit native compiler dependency](builds/go-workspace-cgo/README.md) | builds |
| EX-018 | [A Node package with native build and test scripts](builds/node-package/README.md) | builds |
| EX-019 | [Equivalent npm pnpm and Yarn projects](builds/node-managers/README.md) | builds |
| EX-020 | [Native npm workspace dependency graph](builds/node-workspace/README.md) | builds |
| EX-021 | [A Cargo application with no Oyzu configuration](builds/rust-app/README.md) | builds |
| EX-022 | [Cargo workspace features code generation and proc macros](builds/rust-workspace/README.md) | builds |
| EX-023 | [Maven multi-module reactor](builds/java-maven-reactor/README.md) | builds |
| EX-024 | [Gradle subprojects and an included build](builds/java-gradle-multi-project/README.md) | builds |
| EX-025 | [Conventional and custom Ant task names](builds/java-ant/README.md) | builds |
| EX-026 | [Multi-stage image requiring no external base](builds/docker-offline/README.md) | builds |
| EX-027 | [Separate host execution and container target platforms](builds/container-variants/README.md) | builds |
| EX-028 | [Helm packaging with a local library dependency](builds/helm-chart/README.md) | builds |
| EX-029 | [Application image and chart artifact relationship](builds/image-and-chart/README.md) | builds |
| EX-030 | [Python API Node frontend Go command image and chart](builds/mixed-monorepo/README.md) | builds |
| EX-031 | [A shared deterministic generator with two consumers](builds/generated-source/README.md) | builds |
| EX-032 | [One project tested across runtime versions](builds/compatibility-matrix/README.md) | builds |
| EX-033 | [Shared input and transitive affected targets](builds/affected-graph/README.md) | builds |
| EX-034 | [OCI cache configuration without pipeline cache scripts](cache/oci-standalone/README.md) | cache |
| EX-035 | [A local cache entry cannot confer release authority](cache/producer-evidence/README.md) | cache |
| EX-036 | [Failed tests and tampered artifacts remain visible](artifacts/failed-bundle/README.md) | artifacts |
| EX-037 | [Version text and branch labels do not authorize production](release/eligibility/README.md) | release |
| EX-038 | [Idempotent publication with separate receipts](release/publication-retries/README.md) | release |
| EX-039 | [Approved tool discovery and direct upstream acquisition](managed/tool-catalog/README.md) | managed |
| EX-040 | [Local package routes and expiring upstream leases](managed/package-proxy/README.md) | managed |
| EX-041 | [Management persists through logout and outages](managed/logout-and-outage/README.md) | managed |
| EX-042 | [Explicit probes for host and network access](security/sandbox-escape/README.md) | security |
| EX-043 | [Change mandatory scanning without editing a repository](managed/policy-checks/README.md) | managed |
| EX-044 | [Service-dependent test evidence is explicit](builds/service-test/README.md) | builds |
| EX-045 | [Closing the desktop does not own the build process](lifecycle/desktop-independent/README.md) | lifecycle |
| EX-046 | [Cross-platform profile and activation lifecycle](environments/shell-lifecycle/README.md) | environments |
| EX-047 | [All backend acquisition paths must be mediated](tools/backend-routing/README.md) | tools |
| EX-048 | [Provider facts retain unknown states](release/source-control-facts/README.md) | release |
| EX-049 | [Select multiple artifacts from one producer](builds/materialize-selected-artifacts/README.md) | builds |
| EX-050 | [Materialize a directory and share compatible content](builds/materialize-directory/README.md) | builds |
| EX-051 | [Private Python dependencies across uv pip Poetry and legacy projects](dependencies/python-private/README.md) | dependencies |
| EX-052 | [Private npm pnpm and Yarn dependencies with lifecycle scripts](dependencies/node-private/README.md) | dependencies |
| EX-053 | [Private Maven Gradle and Ant Ivy dependency preparation](dependencies/java-private/README.md) | dependencies |
| EX-054 | [Private Go modules without direct VCS fallback](dependencies/go-private/README.md) | dependencies |
| EX-055 | [Private Cargo registries and credential-free build scripts](dependencies/rust-private/README.md) | dependencies |
| EX-056 | [Private Helm dependencies and OCI registry authentication](dependencies/helm-private/README.md) | dependencies |
| EX-057 | [Prepared apt and apk inputs for container images](dependencies/container-os-packages/README.md) | dependencies |
| EX-058 | [Custom Dockerfiles consume prepared dependencies without credentials](dependencies/docker-credential-boundary/README.md) | dependencies |

## Directory convention

Each scenario contains a README, one or more actual project roots, a scenario.json expectation record, and focused variant/negative input files as needed. Project roots contain real native source and metadata. No shared ancestor Oyzu configuration is needed.

The README describes commands, defaults, outcomes, and failures. The JSON file links acceptance criteria and records intended checks; it is review metadata, not a shipped schema or extra configuration developers must maintain.

Examples describe intended behavior, not implementation. Zero-config roots intentionally omit build.yaml. The smallest app hint stays `api: { uses: python/app }`. Tests, reports, and output sections are inferred. Consumer-owned materialization and platform propagation now follow the [agreed example contract](MATERIALIZATION.md); other matrix axes and chart value binding remain under review.

See the [design catalog](../docs/examples.md), [review guide](REVIEW.md), and [validation scope](VERIFICATION.md). No CLI, policy service, or fake engine has been implemented to make these examples appear to work.
