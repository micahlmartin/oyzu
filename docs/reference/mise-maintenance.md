# Mise source and embedding maintenance

The repository includes a read-only upstream observation command:

```sh
python tooling/mise-upstream/check.py --output upstream-observation.json
```

Use Python 3.11 or later on Windows, macOS or Linux. Network access to GitHub's
public API is required. Optional `GH_TOKEN` supplies API authentication; the
script never prints it and refuses redirects. Each request has a 30-second
timeout and a 4 MiB response limit. No package, tool or upstream code is executed.
The command writes the requested JSON report; it does not change source pins,
create PRs, publish messages or approve updates.

The report records observation time, the latest upstream stable release's tag
and exact commit, the public `oyzuai/mise` default-branch commit, the experiment
base, and Oyzu's production dependency pin when configured. It verifies that
the named fork is public and has `jdx/mise` as its parent. If a direct mise
dependency exists, it must use that fork, an exact full `rev`, disabled default
features and a matching source in Oyzu's root Cargo.lock. A missing dependency
produces `production-integration-not-configured`, never an inferred production
pin from the fork head. Configured dependencies produce `manual-triage-required`.

A successful observation exits 0; failures write `observation-failed` with the
exception type and exit 1. Errors deliberately omit API bodies and credential
details. Diagnose API availability/authentication and local Cargo inputs before
rerunning. Failed observations must not be reported as current or up to date.
The command cannot run offline and does not overwrite a pin as recovery.

The `Mise upstream observation` workflow runs these tests and observations on
relevant PR changes, manual dispatch, and Mondays at 14:00 UTC. **Scheduled runs
start only after the workflow is merged into the default branch.** It has
read-only repository permissions and retains each JSON report as an Actions
artifact for 90 days. The workflow does not create issues or send external
messages. Owners must inspect reports and failed runs; escalation, durable
release evidence retention and an overdue-check dashboard are not implemented.

This is observation, not complete maintenance enforcement. It does not audit
security advisories, compute exposure, validate patch provenance, review license
obligations, qualify platforms, enforce owner approval or perform the monthly
promotion. `release_ready` always remains false. Those gates and @micahlmartin's
ownership are specified in the [maintenance procedure](../proposals/OEP-0003-mise-integration/upstream-maintenance.md).
Unit tests cover exact pin/lock agreement, moving/wrong sources and absence of
production integration. Current live observation evidence is recorded in
[tool-management status](../tool-management-status.md).

The candidate embedding boundary is tracked in
[fork draft PR 2](https://github.com/oyzuai/mise/pull/2), stacked on the compliance
foundation. Its library-only conformance example checks private initialization,
transport restrictions, settings resets, duplicate PATH entries and the actual
Node backend parser/resolver with fixture metadata. It is not Oyzu's production
Cargo dependency. Its native CI matrix and compliance review gate must be checked
against the exact candidate head; old experiment results do not qualify it.

## Candidate Node archive facts

The fork's experimental `Session::node_archive_facts(version, target)` reuses
the Node backend's artifact/mirror selection and native path helpers. It requires
Node admission in the immutable embedding session and an exact stable SemVer
without a `v` prefix, range, prerelease or build suffix. Initial target keys are
`linux/amd64/gnu`, `darwin/arm64/native` and `windows/amd64/msvc`; other tuples fail.
Version inputs are limited to 128 bytes and targets to 64 bytes. No filesystem
publication, process execution or network request occurs during this operation.

For example, `session.node_archive_facts("22.15.0", "linux/amd64/gnu")` returns
the upstream `node-v22.15.0-linux-x64.tar.gz` archive location, its strip prefix,
SHASUMS/signature locations, `bin/node`, `bin/npm` and the `bin` PATH directory.
Windows facts use ZIP, `node.exe`, the upstream `npm.cmd` location and the payload
root. `npm.cmd` is observed upstream layout data, not an Oyzu shim or typed
interpreter launch descriptor. The backend owns these facts; Oyzu does not add a
parallel archive catalog or native version resolver.

These are facts to be checked, not a verified install plan. URL construction does
not prove that an artifact exists, its byte count, checksum or publisher signature.
The supervisor still needs exact acquired bytes, verification evidence, a compiled
admission descriptor and native layout parity before creating the
[owned finalizer plan](tool-lock-inspection.md#data-only-candidate-finalization).
In corporate mode, upstream locations must map to approved logical broker routes;
they do not authorize direct public downloads or bypass a configured proxy.

The library-only conformance example tests two versions and all three targets,
exact fact values, parity with upstream lock artifact URLs, denied session/tool
and target inputs, and absence of extra transport calls. It does not install the
artifacts or execute their launchers. Run in the public fork with Rust 1.95 and
its documented native build prerequisites:

```sh
cargo run --locked --example oyzu-embedding-check --no-default-features --features rustls,vfox/vendored-lua
```

This builds the linked conformance example, not a mise CLI. Current candidate
results and remaining native gates belong in implementation status. Oyzu still
has no production dependency on this API and no automatic installation command;
the source import, licensing and release gates remain separate.
