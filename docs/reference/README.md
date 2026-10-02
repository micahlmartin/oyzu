# Implemented reference

This is the entry point for current functionality documentation. Maintain feature pages in place as behavior evolves, following the [documentation maintenance standard](../documentation.md); the [implementation status](../implementation-status.md) records measured support and remaining work. This index is not a claim that every implemented subsystem already has a complete guide.

| Reference | Coverage |
| --- | --- |
| [Tool identity inspection](tool-lock-inspection.md) | Format-2 selection/installation identities and no-follow payload tree observation; no installation or execution authority |
| [Mise maintenance](mise-maintenance.md) | Read-only upstream observation, exact Cargo pin checks and scheduled-workflow activation limits |
| [Configuration](configuration.md) | Runnable walkthrough, commands, settings/defaults, cascades/profiles, file locations, editing, administrative enforcement, recovery and deferred authentication |
| [npm workspaces](npm-workspaces.md) | Native member builds, composed quality tasks, snapshot packages, report ownership, prerequisites and remaining limits |
| CLI overview below | Discovery, development tasks and initial build/bundle usage; detailed subsystem coverage remains to be expanded as those features are touched |

The experimental CLI supports `oyzu discover`, `oyzu run list`, `oyzu run <task>`, `oyzu build`, `oyzu build --plan` and `oyzu inspect <bundle>`. Global `-C <directory>` selects the project and `--json` emits structured discovery or task outcomes. The implementation is not a stable API release.

Static discovery covers the initial native builder families. Task execution uses already installed native tools in the development environment. TOML overrides and success-only pre_/post_ hooks are executed; native npm lifecycle hooks remain owned by npm. Missing configured tools fail with an error. Unavailable lint/format integrations are listed with a reason rather than silently advertised as successful checks.

Local Rust discovery tests and black-box Node task checks are recorded in [implementation status](../implementation-status.md). Complete manager compatibility, managed policy enforcement and the full hermeticity contract remain required work.

Proposed behavior is documented in the [OEP index](../proposals/README.md). Add reference documentation here only when the implementation and its acceptance evidence exist. Include supported versions/platforms and compatibility limits; do not copy draft examples here as though they are shipped.

The build executor requires Docker and explicitly provisioned toolchain images. The npm default is `oyzu-toolchain/node:npm11.11.0-node22`, built from [the pinned toolchain definition](../../tooling/images/node-npm/Dockerfile); the Go default is `golang:1.24-bookworm`. Override these with `--image npm=<image>` or `--image go=<image>`; image identity is resolved before planning. No image is automatically downloaded. npm preflight checks an exact `packageManager` declaration against the provisioned npm version and uses native strict engine validation. This also applies to projects without dependencies; selecting an image is not a version-check bypass. Compatibility limits and measured acceptance for all native builder profiles are tracked in [implementation status](../implementation-status.md).

Provision the common Node quality image before the npm, pnpm or Yarn image: `docker build -t oyzu-toolchain/node:quality tooling/images/node-quality`. It supplies locked ESLint/Prettier defaults for projects without quality scripts or installed project tools. Builds use read-only lint/format checks; `oyzu run format` is explicitly mutating. For development tasks, install native project dependencies or point `OYZU_NODE_QUALITY_HOME` at an explicitly provisioned directory containing the quality toolchain's package.json and node_modules. This is toolchain provisioning, not an Oyzu project configuration requirement.

Build output is `dist/manifest.json`, `plan.json`, `envelope.json`, native logs and target artifacts/reports. Snapshot names include the captured source identity. Native npm test scripts equal to `node --test` receive JUnit and LCOV reporters; Go test events and coverage profiles are normalized. Available lint/format-check tasks run in the captured worktree. Mutating formatter tasks do not run as build checks. Previous bundles are preserved under `.oyzu/history`. `inspect` validates content integrity only; it does not certify production trust.

Configuration commands, profile selection, administrative constraints, editing and their current limits are described in the [experimental configuration reference](configuration.md).
