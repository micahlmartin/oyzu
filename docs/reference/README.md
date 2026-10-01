# Implemented reference

The experimental CLI supports `oyzu discover`, `oyzu run list`, and `oyzu run <task>`. Global `-C <directory>` selects the project and `--json` emits structured discovery or task outcomes. The implementation is not a stable API release.

Static discovery covers the initial native builder families. Task execution uses already installed native tools in the development environment. TOML overrides and success-only pre_/post_ hooks are executed; native npm lifecycle hooks remain owned by npm. Missing configured tools fail with an error. Unavailable lint/format integrations are listed with a reason rather than silently advertised as successful checks.

Local Rust discovery tests and black-box Node task checks are recorded in [implementation status](../implementation-status.md). This does not establish hermetic builds, complete manager compatibility, policy enforcement or snapshot artifact support. Those remain required work.

Proposed behavior is documented in the [OEP index](../proposals/README.md). Add reference documentation here only when the implementation and its acceptance evidence exist. Include supported versions/platforms and compatibility limits; do not copy draft examples here as though they are shipped.
