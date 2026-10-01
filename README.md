# Oyzu

An opinionated developer platform for tools, environments, tasks, and builds.

**Oyzu is 100% AI-built and community-directed.** AI generates our first-party code; people define requirements, guide implementation, review and validate results. Handcrafted code patches are not our contribution workflow. See the [contribution guide](CONTRIBUTING.md) for how to participate and the current license status.

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
- [Reference](docs/reference/README.md): reserved for verified implemented contracts.
- `.github/`: contribution templates and CLI compilation, task and initial build-scenario workflows.

All designs remain drafts for maintainer review. Run `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --all -- --check` for the current Rust implementation. `python tooling/test-task-scenarios.py --cli <compiled-oyzu-path>` tests the CLI against real native task scenarios. Run `node tooling/check-docs.mjs` separately for documentation structure.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License status

The intended open-source license has not yet been selected. No open-source license is granted by this scaffold; do not assume MIT or another license applies.
