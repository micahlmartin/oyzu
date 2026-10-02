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

## Candidate Go archive facts

The fork's experimental `Session::go_archive_facts(version, target)` uses Go's
upstream artifact/mirror calculation and shared archive-root constant. Like Node,
it requires immutable tool admission, full stable SemVer and one of
`linux/amd64/gnu`, `darwin/arm64/native` or `windows/amd64/msvc`. Input limits are
128 version bytes and 64 target bytes. It returns archive and `.sha256` sidecar
locations, `tar.gz` or `zip`, the `go` strip prefix, `bin/go` or `bin/go.exe`,
`bin` and a fresh-layout GOROOT of `.`. Legacy nested layouts and mutable GOPATH
package installation are outside this interface. Older version spellings without
a patch component are rejected rather than guessed.

This operation performs no Git discovery, acquisition, installation or target
execution. It does not prove catalog membership, artifact availability, digest
authenticity or native layout parity. The OEP's brokered Go catalog adapter,
verification, admitted plan conversion and native install tests remain missing.
Corporate callers must map upstream locations to approved logical routes. The
library conformance command above includes an isolated Go scenario covering two
versions, all three targets, upstream URL parity and zero transport callbacks.

## Pinned registry alias projection

After initializing an experimental fork `Session` with admitted core tools,
`session.tool_aliases()?` returns a sorted alias-to-canonical-ID map. For example,
admitting `node` includes `node -> core:node` and `core:node -> core:node`.
It also includes aliases actually declared by this revision's baked registry.
Empty admission returns an empty map. The initial session allowlist remains
Node, Go, Java and Python; other backends are not implicitly exposed.

The method rejects ambiguous aliases, canonical backend rebinding and upstream
registry drift that removes an admitted core backend. It reads no files, performs
no networking and consults neither floating registry data nor ambient project
aliases. Embedded settings disable floating registry updates, including after a
settings reload. This is a candidate input to Oyzu's request projection, not
version resolution, platform availability or authorization to install.

Fork `f4d245e88` passed strict library/example Clippy and all nine isolated
conformance scenarios on Windows, macOS and Linux in run `36983527342`. These
include all four admitted tools, unadmitted-tool exclusion, empty admission and
reload stability. The production Oyzu crate still needs reviewed source admission
and worker wiring to consume this map.

## Candidate dependency review evidence

The fork provides an offline, target-filtered Cargo evidence collector in
`tooling/compliance/cargo_graph.py`. Its [usage and limitations](https://github.com/oyzuai/mise/blob/codex/oyzu-embedding/compliance/README.md#candidate-cargo-evidence)
describe required caches, target filters, dependency edges, notice hashes and
provenance. Use it to prepare @micahlmartin's review of the proposed dependency
graph. It preserves declared license alternatives and unknowns; it cannot approve
an import, select legal terms or clear a distribution. All rows remain unreviewed.
Cargo metadata may unify development features and does not prove which native or
generated code ships. The actual compiled graph, notice packaging and applicable
source-delivery obligations remain separate release evidence.

### Retained candidate graph evidence

Fork `f7d5bb906` adds target-specific collection to the native embedding workflow.
After library checks, it provisions the exact locked, target-filtered metadata query and feature set,
then runs the offline collector for the matching Linux GNU, Darwin ARM64 or
Windows MSVC target. Reports are retained for 30 days as
`cargo-evidence-<target>-<commit>` artifacts. Preserve reviewed evidence outside
expiring CI storage before release review. A missing report fails the job; an
upload does not approve licenses, establish a shipping graph or fulfill source
delivery obligations. Notice directory enumeration failures reject collection;
unreadable directories cannot silently disappear from a successful report. Linux run `36975506849` produced a clean candidate report with 955 packages;
98 have no collector-recognized notice file. Missing observed notices require
source review and do not establish missing legal rights. Reports on PRs bind the
tested merge revision, which may differ from the branch head.

The [candidate evidence index](../proposals/OEP-0003-mise-integration/candidate-license-evidence.json)
records all three downloaded native reports from run `36976065769`, including
report hashes and package identities without observed notice files. Each binds
clean merge revision `9e7556ae1b057830296402549c76bfbd38981acc`: Darwin
ARM64 has 952 packages, Windows MSVC 975 and Linux GNU 955. This is a review
index, not a notice bundle, validated license selection or shipping SBOM. The
original reports must still accompany any review; index hashes alone cannot
recover their contents after CI artifact expiration.

### Original notice archives

The candidate collector now accepts `--notice-bundle cargo-notices.zip`. In the
fork checkout, after separately provisioning its Cargo cache, run:

```sh
python tooling/compliance/cargo_graph.py --target x86_64-unknown-linux-gnu --output cargo-evidence.json --notice-bundle cargo-notices.zip
```

Both parents must exist. The report path is overwritten; the archive path must
not exist. Collection is offline and does not compile or execute mise. It embeds
the exact report, original notice bytes and an index binding package identities,
relative paths, raw/LF-normalized hashes and sizes. The index lists packages with
no observed notices. The workflow retains both files in the same target/revision
artifact. Download both before the 30-day retention expires.

Fixed ZIP metadata and stored entries make identical inputs reproducible; original
line endings and copyright text are preserved. Notice drift, redirects, duplicate
paths or exceeded limits fail collection and remove the operation's partial
archive. Existing archives are never replaced. Limits are 20,000 notices, 2 MiB
per file, 256 MiB of notice bytes and 64 MiB each for report/index metadata. The
collector works on trusted provisioned cache roots; it is not a concurrent
hostile-filesystem sandbox. Select another output path to rerun collection.

An offline Linux development run captured 1,512 notices totaling 7,822,597 bytes
for 955 packages; independent ZIP verification matched every indexed file to the
report. It still reports 98 packages without observed notices and a dirty
worktree. This demonstrates collection, not a clean release audit. Native CI
archives for the updated collector remain pending. This archive does not recover
missing texts, inspect all source headers/native code, include installed tools'
licenses, select legal terms or fulfill source-delivery requirements. The exact
shipping graph, release SBOM and maintainer approval remain required.

### Java metadata boundary experiment

The fork's eighth fresh-process conformance scenario exercises the existing Java
backend `resolve_lock_info` method with supplied synthetic Temurin metadata for
Linux, Darwin ARM64 and Windows. It preserves target URL/checksum facts, rejects
unavailable versions and unsupported installer formats, and verifies exactly one
transport callback per target while reusing the backend's metadata cache. The
session admits Java only and reads no ambient configuration. Linux library/example
Clippy and all eight scenarios pass on Linux, Windows and macOS in run
`36977793983` at fork `136d65573`.
This adds no Java layout API and performs no JDK acquisition, publisher verification
or execution. It is parser-reuse evidence, not production Java qualification.
