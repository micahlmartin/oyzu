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
features and a matching source in Oyzu's root Cargo.lock. Inspection covers renamed
`package = "mise"` declarations, normal/build/development dependencies and target
tables; all declarations must agree on one revision. Workspace inheritance, mise
patch/replacement overrides and a lock entry without an inspected declaration
fail explicitly because their provenance is not supported by this root-manifest
observer. Unrelated dependencies and overrides are unaffected. A missing dependency
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
Unit tests cover exact pin/lock agreement, moving/wrong sources, renamed and
target-specific declarations, conflicting revisions, unsupported source overrides
and absence of production integration. Current live observation evidence is recorded in
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
reads the [official Go release JSON catalog](https://go.dev/dl/?mode=json&include=all)
through supplied HTTP transport and a process-private parsed catalog cache, as
required by the OEP.
It returns `GoVersionResolution { version, catalog_sha256 }`, binding selection to
an exact SHA-256 of the UTF-8 catalog text. This is metadata identity, not publisher
authentication. The earlier candidate GitHub-tag path was removed because it did
not match the specified initial source; no source fallback is attempted.

Selection uses the shared Node/Go selector and upstream Go version comparator.
Exact stable versions, numeric prefixes, `latest` and upstream npm-compatible ranges
are supported. All supplied constraints must match. For example, catalog releases
`go1.24.13`, `go1.24.14` and `go1.25.0` select `1.24.13` for `latest` constrained
by `>=1.24` and `<1.24.14`. These are conformance values, not live recommendations.

Catalog input must be valid UTF-8 and is limited to 16 MiB before JSON decoding, 100,000 release records and
128 bytes per version. The transport owns streaming byte limits and deadlines.
Missing or mistyped `version`/`stable` fields and duplicate release identities fail.
The adapter also decodes file records for target matching; other additional
official catalog fields are ignored. Missing file lists permit version selection
but cannot satisfy target metadata. Unstable
releases and versions outside the canonical three-component stable archive contract
are excluded; older spellings are never converted into guessed artifact versions.

Selector limits match Node: 256 constraints and 1,024 bytes per nonblank/control-free
selector. Invalid selectors fail before acquisition. Exact pins require catalog
membership. Missing metadata, conflicting constraints and unsupported request modes
fail without Git execution, installed-tool discovery or another catalog source.
Offline reuse within one worker can use its parsed snapshot. A new worker requires
a supplied transport serving retained metadata; this adapter has no disk cache. The returned digest always identifies the actual text
selected from, including on cache hits.

This version-selection API alone does not discover `go.mod`/`go.work` directives,
prove target availability or verify publishers. Target metadata performs the file
matching described below. Production worker wiring and real archive qualification remain
outstanding. All twenty Linux Rust 1.95 conformance scenarios passed, including
seven Go catalog cases. At fork revision `32db610a7`, all three native hosts passed
the library/example Clippy, utility-library Clippy and conformance steps in
[run 36990464407](https://github.com/oyzuai/mise/actions/runs/36990464407).
Windows/macOS jobs were still finishing evidence/cache steps when inspected.
This qualifies those checks for the catalog-source correction, not the later
catalog-file binding or the full installation/backend matrix.

### Real metadata replay qualification

From the Oyzu checkout, Python 3.11+ can provision public metadata fixtures:

```text
python tooling/mise-upstream/capture_go_metadata.py --version 1.24.13 --version 1.25.0 --output /outside/repository/go-metadata.json
```

Use a native absolute path outside the repository whose parent exists. Output must
not already exist. The helper fetches the official release catalog and each exact
target checksum sidecar over HTTPS, checks identity/hash agreement and retains
original UTF-8 response bodies, byte sizes, SHA-256 values and six expected fields
per version/target. It accepts one to sixteen unique canonical numeric versions;
missing, unstable, ambiguous or contradictory records fail without fallback.
Catalog responses are bounded to 16 MiB, sidecars to 128 bytes and release file
lists to 4,096 entries. Serialized fixture output is bounded to the replay reader's
32 MiB limit before file creation, including JSON escaping overhead. Network reads
use a 30-second socket timeout, not a global operation deadline. This host provisioning helper uses Python's normal HTTPS/proxy
configuration; it is not the isolated product broker. No archive is downloaded.
Acquisition failures create no output; a write failure removes the newly created
partial file. Rerun with another output path after resolving the failure.

In the fork checkout, set `OYZU_GO_METADATA_FIXTURE` to the captured file and run
the library conformance command described above. The example adds a fresh-process
`go-real-metadata` scenario, checks captured byte hashes and compares exact version,
target, URL, declared size/hash and catalog digest. Only captured URLs are served;
network fallback is absent. The parent bounds fixtures to 32 MiB and copies them
into private worker state without forwarding the environment variable. Without
the variable, the ordinary synthetic conformance suite remains unchanged.

Go 1.24.13 and 1.25.0 for all three initial targets passed Linux Rust 1.95 replay
with Docker `--network none`, alongside twenty-two existing scenarios. A changed
expected size produced a failing replay. Provisioning tests, Clippy, formatting and
compliance inventory checks passed. This qualifies real metadata interpretation,
not archive content, publisher signatures, installation, native Go execution or
legal approval. Native CI confirmation is recorded below. Retain the fixture
outside the checkout; a later catalog capture can have a different digest.

The fork's native CI now captures and replays these same two versions on each
initial host, after running the ordinary suite. It downloads the provisioner from
Oyzu commit `f35c01d5bc8348b4ee6efee8e52c5352e0003dfd` and checks its fixed
SHA-256 before execution. Updating that helper requires an explicit workflow
revision/hash change. Fixture files stay in runner temporary storage, outside the
fork's source and notice inventory. Target/revision artifacts retain the fixture
and capture report for 30 days, including after replay failure when capture succeeded.
Preserve needed evidence before expiry. Publisher outages, changed metadata and
missing cases fail the job; synthetic data is never substituted. This native CI
replay uses the supplied-response boundary; it does not claim the runner's network
is disabled. The separate Linux `--network none` result above proves that narrower
offline case. Workflow syntax, path filters and the exact pinned capture step were
verified locally on Windows. Run `36992169356` subsequently passed all three native
library jobs at tested merge `58fa4ec749d3af8638bd5aa582fb2928595f2739`
(fork head `b71e447bc10acfbce3627b5f4c8baeb4a0f2f6fb`), including the ordinary
suite and six real target cases per host. All three captured fixtures and reports
were retained outside the checkout and independently checked for exact fixture and
response hashes/sizes and catalog/sidecar agreement. The
[evidence index](../proposals/OEP-0003-mise-integration/go-metadata-replay-evidence.json)
records their identities. This resolves native metadata replay verification only;
it does not establish product worker integration or production backend admission.

### Target metadata API

The fork exposes async `session.go_archive_metadata("1.24.13", target)`.
It requires Go session admission, an exact canonical stable version and one of the
initial Linux amd64 GNU, Darwin arm64 or Windows amd64 MSVC targets before metadata
access. The official catalog snapshot must contain that stable release and the
exact archive filename computed by upstream Go artifact selection. Its file record
must match the target OS/architecture, `go`-prefixed version and `archive` kind,
with a positive byte size and a valid SHA-256. The method then fetches the existing
`.sha256` sidecar and requires case-insensitive equality with the catalog digest.

Output contains `GoArchiveFacts`, lowercase `declared_sha256` with a `sha256:`
prefix, `declared_size` and the same `catalog_sha256` used for version selection.
These are declared metadata, not proof of acquired bytes or publisher authenticity.
No archive is downloaded or executed. Missing releases/targets, contradictory
record fields, malformed hashes and zero sizes fail before sidecar acquisition;
sidecar disagreement also fails without fallback.

Each canonical stable release is limited to 4,096 file records with unique,
nonempty filenames of at most 512 bytes. The catalog snapshot is cached only for
this worker process. Sidecar responses remain uncached, limited to 128 UTF-8 bytes
and exactly 64 ASCII hexadecimal characters after surrounding whitespace is
trimmed. Empty, malformed, multiple or oversized digests fail. The transport owns
streaming bounds and timeouts before buffering/decoding; these checks are not a
streaming resource limit. Offline operation requires supplied retained catalog and
sidecar responses; no other source or cache is silently substituted.

This extends the experimental metadata result shape and replaces checksum-only
acceptance. Production Oyzu source admission, worker wiring, real archive/layout
qualification and publisher verification remain outstanding. At fork revision `0d58ad7c1`, all three native hosts passed strict library/example
Clippy and the twenty-two-scenario conformance step in
[run 36991065278](https://github.com/oyzuai/mise/actions/runs/36991065278).
Windows/macOS jobs were still finishing post-check work when inspected; these
results do not qualify the separate optional real-metadata replay on those hosts. Linux Rust 1.95 strict
library/example Clippy, formatting and all twenty-two conformance scenarios passed.

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
