# Implemented reference

This is the entry point for current functionality documentation. Maintain feature pages in place as behavior evolves, following the [documentation maintenance standard](../documentation.md); the [implementation status](../implementation-status.md) records measured support and remaining work. This index is not a claim that every implemented subsystem already has a complete guide.

| Reference | Coverage |
| --- | --- |
| [Configuration](configuration.md) | Runnable walkthrough, commands, settings/defaults, cascades/profiles, file locations, editing, administrative enforcement, recovery and deferred authentication |
| [npm workspaces](npm-workspaces.md) | Native member builds, composed quality tasks, snapshot packages, report ownership, prerequisites and remaining limits |
| [Node quality](node-quality.md) | ESLint/Prettier/Biome selection, provisioning, read-only checks, explicit formatting, scopes and failure behavior |
| [Python testing](python-testing.md) | Native pytest collection, root/configured suites, manager commands, report preparation, no-tests outcomes and evidence limits |
| [Python applications](python-applications.md) | Native-wheel console archives, runtime dependencies, snapshot artifacts, archive-source tests and format limits |
| [Helm charts](helm.md) | Local chart dependencies, snapshot packaging, native unittest discovery, independent JUnit reports, baseline integrity and remaining limits |
| [Go builds](go.md) | Native workspace tasks, applications, module snapshot zip/mod/info artifacts, projected local dependencies, reports and remaining limits |
| [Rust builds](rust.md) | Captured crates.io inputs, Cargo workspaces, native feature-gated binary selection, compiler-message artifacts, snapshot crates and JUnit/coverage |
| [Java quality](java-quality.md) | Shared native lint and read-only formatting defaults, explicit formatting, overrides and provisioning for Maven/Gradle/Ant |
| [Maven builds](maven.md) | Native reactors, captured repositories, snapshot JAR/POM/WAR artifacts, effective Surefire/Failsafe report directories and coverage |
| [Dockerfile quality](docker-quality.md) | Native lint/format defaults, read-only build gates, explicit formatting, ignored configuration preservation and toolchain prerequisites |
| CLI overview below | Discovery, development tasks and initial build/bundle usage; detailed subsystem coverage remains to be expanded as those features are touched |

The experimental CLI supports `oyzu discover`, `oyzu run list`, `oyzu run <task>`, `oyzu build`, `oyzu build --plan` and `oyzu inspect <bundle>`. Global `-C <directory>` selects the project and `--json` emits structured discovery or task outcomes. The implementation is not a stable API release.

Static discovery covers the initial native builder families. Task execution uses already installed native tools in the development environment. TOML overrides and success-only pre_/post_ hooks are executed; native npm lifecycle hooks remain owned by npm. Missing configured tools fail with an error. Unavailable lint/format integrations are listed with a reason rather than silently advertised as successful checks.

Local Rust discovery tests and black-box Node task checks are recorded in [implementation status](../implementation-status.md). Complete manager compatibility, managed policy enforcement and the full hermeticity contract remain required work.

Proposed behavior is documented in the [OEP index](../proposals/README.md). Add reference documentation here only when the implementation and its acceptance evidence exist. Include supported versions/platforms and compatibility limits; do not copy draft examples here as though they are shipped.

The build executor requires Docker and explicitly provisioned toolchain images. The npm default is `oyzu-toolchain/node:npm11.11.0-node22`, built from [the pinned toolchain definition](../../tooling/images/node-npm/Dockerfile); the Go default is `oyzu-toolchain/go:1.24-mod0.25.0`, described in [Go builds](go.md). Override these with `--image npm=<image>` or `--image go=<image>`; image identity is resolved before planning. No image is automatically downloaded. npm preflight checks an exact `packageManager` declaration against the provisioned npm version and uses native strict engine validation. This also applies to projects without dependencies; selecting an image is not a version-check bypass. Compatibility limits and measured acceptance for all native builder profiles are tracked in [implementation status](../implementation-status.md).

Provision the common Node quality image before the npm, pnpm or Yarn image: `docker build -t oyzu-toolchain/node:quality tooling/images/node-quality`. It supplies locked ESLint/Prettier defaults and Biome for native Biome configurations without installed project tools. Builds use read-only lint/format checks; `oyzu run format` is explicitly mutating. For development tasks, install native project dependencies or point `OYZU_NODE_QUALITY_HOME` at an explicitly provisioned directory containing the quality toolchain's package.json and node_modules. This is toolchain provisioning, not an Oyzu project configuration requirement. See [Node quality](node-quality.md) for versions, file scope and native configuration behavior.

Build output is `dist/manifest.json`, `plan.json`, `envelope.json`, native logs and target artifacts/reports. Snapshot names include the captured source identity. Native npm test scripts equal to `node --test` receive JUnit and LCOV reporters; Go test events and coverage profiles are normalized. Available lint/format-check tasks run in the captured worktree. Mutating formatter tasks do not run as build checks. Previous bundles are preserved under `.oyzu/history`. `inspect` validates content integrity only; it does not certify production trust.

Configuration commands, profile selection, administrative constraints, editing and their current limits are described in the [experimental configuration reference](configuration.md).
