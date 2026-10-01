# Repository instructions

## Scope
This is the public Oyzu repository. It must be buildable and usable without private repository access or a platform account.

## Working rules
- Preserve existing user changes and inspect the worktree before editing.
- Do not publish private platform details, credentials, or customer information.
- Keep the CLI and agent headless; the desktop interface is optional.
- Account for Windows, macOS, and Linux in applicable changes.
- Do not invoke or bundle a separate mise executable.
- Keep project configuration minimal; do not introduce a programming language.
- Link significant implementation work to its agreed design and acceptance criteria. Distinguish drafts from accepted requirements.
- Do not choose a license or copy upstream code before preserving applicable notices and checking licensing.
- Add verification appropriate to the change; never claim unperformed checks passed.

## Current state
Draft visions and OEPs are indexed in docs/README.md. Implementation is now authorized and in progress. The Rust CLI sources are in src/ with tests in tests/. Track measured capabilities and remaining work in docs/implementation-status.md; do not mark full build scenarios passing based only on discovery or development task execution.

Run `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --all -- --check` for Rust changes. Run `python tooling/test-task-scenarios.py --cli <compiled-oyzu-path>` for real CLI task behavior. Tool installation is excluded from the current implementation goal; use explicitly provisioned native tools.

Run `node tooling/check-docs.mjs` for documentation structure. Do not imply this validates product behavior. Preserve agreed constraints in docs/decisions.md, distinguish proposed syntax from stable contracts, and never mark a design accepted without recorded maintainer review.

The examples/ tree contains design-contract fixtures for review, not an implemented acceptance suite. Keep project configuration minimal and expectations outside project roots. Validate their structure with `python tooling/check-examples.py` (Python 3.11+ and tooling/examples-requirements.txt). Do not implement a fake Oyzu engine to make example outcomes pass.
