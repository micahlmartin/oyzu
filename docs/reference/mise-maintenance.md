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

### Node target checksum metadata

After Node admission, the experimental async method
`session.node_archive_metadata("22.14.0", "windows/amd64/msvc")` retrieves the
version's `SHASUMS256.txt` through supplied transport or the operation-private
metadata cache. It returns existing target archive facts plus a canonical
`declared_sha256` only when the exact generated archive filename has a valid
manifest entry. It shares Node's existing archive naming and checksum-fetch path;
it neither downloads the archive nor substitutes a source build or another target.

Version/target restrictions are the same as `node_archive_facts`. The shared hash
module's checked parser bounds manifest text to 8 MiB and 4,096 entries. It accepts
ordinary/coreutils binary-marker lines, normalizes hexadecimal letter case and
rejects malformed hashes, duplicate filenames (even equal hashes), extra fields
and control characters in filenames. Blank lines are permitted. The general
legacy checksum parser retains its previous behavior. Transport must still bound
the response before text decoding; this parser limit is not a network byte cap.

An absent exact entry, invalid manifest, denied session, unavailable transport or
unsupported input fails the operation. Same-version target queries can reuse one
cached manifest. Results are publisher-declared metadata only: they do not verify
the publisher's signature, acquire/check artifact bytes, establish artifact size
or authorize installation. The supervisor must bind verified evidence and exact
bytes before writing a distributable lock/install record. An archive's presence
in a supplied manifest is not proof that its URL is currently downloadable.

The twelfth fresh-process conformance scenario checks Linux, Darwin ARM64 and
Windows facts/checksums against supplied metadata, cache reuse, missing targets,
invalid/duplicate entries and parser limits. Existing missing-transport and
admission scenarios also cover this method. This is library conformance, not
production publisher verification or installation qualification. Fork revision
`1f516e78ec8c15234d955b0ad24eee70117f5e3b` passed all twelve scenarios on Linux,
strict library/example and shared-utility Clippy, and formatting. Native Windows
and macOS confirmation for this increment remains pending.

## Candidate Go archive facts

### Constrained catalog resolution

The candidate fork's `session.resolve_go_version(request, native_constraints)`
uses mise's backend catalog and the same admitted selector as Node. It supports
exact stable versions, numeric prefixes, `latest` and upstream npm-compatible
range expressions; all supplied constraints must match. For example, fixture tags
`go1.24.13`, `go1.24.14` and `go1.25.0` select `1.24.13` for request `latest`
with constraints `>=1.24` and `<1.24.14`. These are test values, not live recommendations.

Embedded Go obtains complete tags using mise's existing GitHub parser over the
supplied HTTP transport, rather than spawning Git. It retains Go prefix filtering,
prerelease exclusion, deduplication and ordering. Pagination permits at most 1,000
pages and 100,000 tags, rejects repeated URLs and denies next-page links that
change the origin or introduce URL credentials before header creation/acquisition. A failed page fails the operation; a partial catalog is never returned.
No commit-date requests are made. Response-byte limits and deadlines remain the
transport's responsibility. Catalog results can use the session's private cache;
there is no installed-tool or public aggregation-service fallback.

Selector limits match Node: 256 constraints, 1,024 bytes per nonblank/control-free
selector and 100,000 catalog versions of at most 128 bytes. Exact pins require
catalog membership. Missing transport, conflicting constraints, unsupported request
modes and absent/noncanonical stable versions fail. Older Go tags without three
numeric version components are not admitted by this initial archive contract.
The API does not discover `go.mod`/`go.work` directives, establish target availability
or verify publishers. Production worker wiring remains outstanding.
Linux Rust 1.95 passed all nineteen conformance scenarios, including six Go
resolution cases covering successful constraints/cache and five denial modes.
Native Windows/macOS verification for this increment remains pending.

### Target metadata

The fork also exposes async `session.go_archive_metadata("1.24.13", target)`.
It requires Go session admission and an exact canonical stable version before
fetching the existing target-specific `.sha256` URL through the supplied transport.
Targets remain Linux amd64 GNU, Darwin arm64 and Windows amd64 MSVC. It returns
the same `GoArchiveFacts` plus `declared_sha256` with a `sha256:` prefix and
lowercase hexadecimal digits. No archive bytes are downloaded or executed.

The response must be at most 128 UTF-8 bytes and, after trimming surrounding
whitespace, exactly 64 ASCII hexadecimal characters. Empty, malformed, multiple
or oversized digests fail. A transport error propagates without public-network
fallback; offline use requires a supplied transport serving retained metadata.
The transport owns streaming bounds and timeouts before text decoding; this
post-decoding check is not a streaming resource limit. No metadata cache is added.
Declared hashes do not establish catalog membership, artifact size/content,
publisher authentication or installation authority. This experimental API remains
in the fork; the production Oyzu dependency and worker connection remain pending.
Linux Rust 1.95 library/example Clippy and all thirteen fresh-process conformance
scenarios passed for this increment; native Windows/macOS verification is pending.

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
authenticity or native layout parity. The OEP's production broker wiring, publisher verification, admitted plan conversion
and native install tests remain missing.
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

## Candidate Node version selection

The fork's experimental async method
`session.resolve_node_version(request, native_constraints)` selects a canonical
stable Node version from supplied or operation-private cached metadata. The
session must admit Node. It reuses the upstream Node catalog loader, aliases,
ordering, prefix matcher and npm-compatible range filter. For example, with a
catalog containing `22.14.0`, `22.15.0` and `24.1.0`, request `latest` with
constraints `>=22` and `<22.15` selects `22.14.0`; request `22` alone selects
`22.15.0`. These are conformance fixture values, not recommendations or live
catalog claims.

Inputs support exact versions, ordinary numeric prefixes, pinned upstream Node
aliases such as `lts/jod`, and npm semver ranges. Every native constraint is
intersected before selecting the last match in upstream catalog order; constraint
order does not change the answer. Exact pins must occur in the catalog. Empty
intersections and unknown/nonstable versions fail rather than returning the input
as an assumed existing version. Path, system, ref, subtraction and explicit
`prefix:` modes are not admitted. Node's ordinary `22` prefix syntax is supported.

Each selector is limited to 1,024 bytes without control characters; at most 256
native constraints are accepted. Invalid inputs fail before metadata access.
The returned catalog is limited to 100,000 entries of at most 128 bytes each;
the transport remains responsible for response-byte bounds and deadlines before
JSON parsing. Metadata may use the supplied broker callback and private cache.
Without cached metadata or supplied transport, even an exact pin fails. There is
no ambient installed-version fallback, project discovery, archive acquisition,
installation or execution. No separate mise executable is involved.

This operation resolves Node metadata only. It does not prove target archive
availability, publisher authenticity, release-age eligibility or permission to
install. Those remain supervisor/admission checks. Other backends, the production
worker connection, multi-platform closure resolution and lock orchestration
remain incomplete. The library conformance example adds fresh-process selection
and missing-transport cases to the existing admission checks. Fork revision
`6f8e6863794ac070bb0b80921245cf73a9981a20` passed strict library/example Clippy,
formatting and all eleven fresh-process scenarios on Linux. Native Windows and
macOS confirmation for this increment remains pending.

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

The [preserved native archive index](../proposals/OEP-0003-mise-integration/candidate-notice-evidence.json)
binds downloaded reports and archives from run `36985298309`, at clean tested
merge revision `0d0cfce94bfd775ef95ee70090b22f367ee37458`. Independent verification
checked embedded/external report equality and every notice's raw/LF-normalized
hash, size and package/path correspondence.

Reproduce this check with Python 3.11+ from the Oyzu checkout, after downloading
the named artifacts using `gh run download 36985298309 --repo oyzuai/mise --dir retained-notices`:

```text
python tooling/mise-upstream/verify_notices.py --artifacts retained-notices
```

The offline verifier uses the checked-in index by default; `--index PATH` selects
a different trusted index. It compares report/archive hashes, clean source/target
identity, every indexed notice and the missing-notice package list. It writes JSON
to stdout without extracting files. Missing/malformed input, mismatches or approval
flags fail with a nonzero exit. Retain failed inputs for investigation and reacquire
original artifacts; do not change trusted hashes merely to silence a failure.
Unrelated artifact directories are ignored. Archive input is bounded to 400 MiB;
metadata/notice limits match the collector. The index is a trust input, not a
signature, and success does not authenticate a replaced index or establish legal
completeness. Output retains `legal_approval: false` and `release_ready: false`.

Verification passed on Windows against all three retained native archives.
Synthetic regression tests reject altered external bytes, embedded-report mismatch,
hidden entries, inconsistent notice identities/counts and false approval flags.
The upstream-observation workflow's existing test discovery includes these tests;
it does not fetch the retained archives. Preserve those separately before CI expires.

| Target | Candidate packages | Observed notice candidates | Packages without observed notices |
| --- | ---: | ---: | ---: |
| Darwin ARM64 | 952 | 1,501 | 98 |
| Windows MSVC | 975 | 1,545 | 96 |
| Linux GNU | 955 | 1,510 | 98 |

The earlier dirty Linux development archive had 1,512 entries. Comparison with
clean CI found two generated Python bytecode files whose names matched the
conventional notice pattern. Those are not legal notice text; use the clean CI
archives for review. Filename matches elsewhere also need content review, rather
than automatic license conclusions. Neither clean status nor successful archive
verification establishes a release audit. This archive does not recover
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
