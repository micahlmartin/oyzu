<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/assets/oyzu-full-logo-reverse.svg">
    <img src="docs/brand/assets/oyzu-full-logo-color.svg" alt="Oyzu" width="320">
  </picture>
</p>

# Oyzu

**An opinionated, batteries-included developer platform.**

From a project checkout to a build you can inspect, Oyzu aims to bring tools, environments, tasks, builds, and evidence into one consistent workflow. It understands your ecosystem's conventions, chooses useful defaults, and keeps configuration focused on intent and exceptions.

[Platform vision](docs/vision/VIS-001-platform.md) · [Current capabilities](docs/implementation-status.md) · [Design library](docs/README.md) · [Contribute](CONTRIBUTING.md)

**100% AI-built, community-directed.** People define requirements, guide agents, review and verify results; AI generates our first-party implementation code, tests and scripts. See the [contribution policy](CONTRIBUTING.md#ai-built-community-directed) and [engineering rules](AGENTS.md#architecture-rules).

## The vision

- **Batteries included.** Tool management, environment activation, native tasks, testing, reports, artifacts, and caching belong in a coordinated developer experience.
- **Opinionated defaults.** Conventional supported projects should build without an Oyzu build file. Builders own the routine wiring; you declare the exceptions.
- **Evidence built in.** Plans should be explainable, execution constrained, and outputs accompanied by manifests and test evidence.
- **A foundation you can use independently.** The standalone CLI needs no platform account. Humans, CI, and AI agents share the same deterministic commands; a desktop interface is optional.

The intended host platforms are Windows, macOS, and Linux. Optional managed capabilities extend the same foundation with organizational identity, configuration, policy, and visibility. The [platform vision](docs/vision/VIS-001-platform.md) describes the full direction; delivery is incremental.

## Try the current CLI

**Early implementation.** The vision above is broader than today's capabilities.

The Rust CLI is under active implementation. Native builder/task discovery and development tasks support overrides and hooks. An initial container build command produces snapshot bundles for simple Node/npm and Go projects. The complete builder catalog and scenario suite remain in progress; see the [implementation status](docs/implementation-status.md).

```text
cargo build --locked
cargo run -- --root examples/tasks/npm-scripts/project run list
cargo run -- --root examples/tasks/npm-scripts/project run foo
```

Native tools must already be installed. This implementation does not install toolchains. `oyzu run` executes development tasks; it does not claim hermetic build evidence.

## Repository layout

- [Design library](docs/README.md): reading paths, architecture, decisions, and delivery plan.
- [Code map and engineering guide](docs/code-organization.md): subsystem ownership, interfaces, reuse and extension workflow.
- [Visions](docs/vision/README.md): platform and component direction.
- [Proposals](docs/proposals/README.md): detailed Oyzu Enhancement Proposals (OEPs).
- [Examples](examples/README.md): 58 checked-in design-contract scenarios with source, configuration, expected behavior, and failures.
- [Example catalog](docs/examples.md): acceptance-criteria traceability.
- [Reference](docs/reference/README.md): current functionality and verified limits, maintained with each change under the [documentation standard](docs/documentation.md).
- `.github/`: contribution templates and CLI compilation, task and initial build-scenario workflows.

All designs remain drafts for maintainer review. Run `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --all -- --check` for the current Rust implementation. `python tooling/test-task-scenarios.py --cli <compiled-oyzu-path>` tests the CLI against real native task scenarios. Run `node tooling/check-docs.mjs` separately for documentation structure.

## Contributing

**Oyzu is 100% AI-built and community-directed.** First-party implementation code, tests and scripts are AI-generated; handcrafted code patches are not our contribution workflow. People define requirements, direct agents, review results and verify behavior. Ideas, bug reports, design feedback, examples, testing and reviewed AI-generated changes are welcome under the [contribution policy](CONTRIBUTING.md), including its current license restrictions. Third-party code retains its actual authorship and licenses.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

Logos, color tokens, and usage guidance are in the [brand guide](docs/brand/README.md).

## License status

The intended open-source license has not yet been selected. No open-source license is granted by this scaffold; do not assume MIT or another license applies.
