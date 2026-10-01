# Oyzu

An opinionated developer platform for tools, environments, tasks, and builds.

The Rust CLI is under active implementation. The first checkpoint provides native builder/task discovery and development task execution with overrides and hooks. Full build orchestration, isolated execution, versioned artifact bundles and the complete scenario suite remain in progress; see the [implementation status](docs/implementation-status.md).

```text
cargo build --locked
cargo run -- -C examples/tasks/npm-scripts/project run list
cargo run -- -C examples/tasks/npm-scripts/project run foo
```

Native tools must already be installed. This implementation does not install toolchains. `oyzu run` executes development tasks; it does not claim hermetic build evidence.

## Repository layout

- [Design library](docs/README.md): reading paths, architecture, decisions, and delivery plan.
- [Visions](docs/vision/README.md): platform and component direction.
- [Proposals](docs/proposals/README.md): detailed Oyzu Enhancement Proposals (OEPs).
- [Examples](examples/README.md): 58 checked-in design-contract scenarios with source, configuration, expected behavior, and failures.
- [Example catalog](docs/examples.md): acceptance-criteria traceability.
- [Reference](docs/reference/README.md): reserved for verified implemented contracts.
- `.github/`: contribution templates and CLI compilation/task-scenario workflow.

All designs remain drafts for maintainer review. Run `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --all -- --check` for the current Rust implementation. `python tooling/test-task-scenarios.py --cli <compiled-oyzu-path>` tests the CLI against real native task scenarios. Run `node tooling/check-docs.mjs` separately for documentation structure.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License status

The intended open-source license has not yet been selected. No open-source license is granted by this scaffold; do not assume MIT or another license applies.
