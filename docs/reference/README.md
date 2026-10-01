# Implemented reference

The experimental CLI supports `oyzu discover`, `oyzu run list`, `oyzu run <task>`, `oyzu build`, `oyzu build --plan` and `oyzu inspect <bundle>`. Global `-C <directory>` selects the project and `--json` emits structured discovery or task outcomes. The implementation is not a stable API release.

Static discovery covers the initial native builder families. Task execution uses already installed native tools in the development environment. TOML overrides and success-only pre_/post_ hooks are executed; native npm lifecycle hooks remain owned by npm. Missing configured tools fail with an error. Unavailable lint/format integrations are listed with a reason rather than silently advertised as successful checks.

Local Rust discovery tests and black-box Node task checks are recorded in [implementation status](../implementation-status.md). Complete manager compatibility, managed policy enforcement and the full hermeticity contract remain required work.

Proposed behavior is documented in the [OEP index](../proposals/README.md). Add reference documentation here only when the implementation and its acceptance evidence exist. Include supported versions/platforms and compatibility limits; do not copy draft examples here as though they are shipped.

The initial build executor requires Docker with a pre-provisioned `node:22-bookworm-slim` or `golang:1.24-bookworm` image. Override these with `--image npm=<image>` or `--image go=<image>`; image identity is resolved before planning. No image is automatically downloaded. Currently supported build inputs are dependency-free npm packages and simple Go applications with dependencies available in the toolchain. Other manager, workspace and platform features fail explicitly.

Build output is `dist/manifest.json`, `plan.json`, `envelope.json`, native logs and target artifacts/reports. Snapshot names include the captured source identity. Native npm test scripts equal to `node --test` receive JUnit and LCOV reporters; Go test events and coverage profiles are normalized. Available lint/format-check tasks run in the captured worktree. Mutating formatter tasks do not run as build checks. Previous bundles are preserved under `.oyzu/history`. `inspect` validates content integrity only; it does not certify production trust.
