# Expanded mise qualification

Status: qualification experiment completed with passing and falsified hypotheses;
production integration remains unqualified. This is a draft design record, not
an accepted design or a claim of shipping product support.
This extends the completed [Node experiment](experiment.md) following the
maintainer request to qualify broader backends, macOS and production broker/
network enforcement. Existing passing results remain evidence for their original
scope; they do not satisfy the expanded scope.

## Hypotheses and required evidence

1. The same Oyzu TOML/lock adapter can drive multiple real backend families.
   Exercise core Node, Go, Java and Python distributions, an aqua registry tool,
   a script/plugin backend and a native package-manager backend. Record exact
   backend/plugin/distribution identities, real version output, environment and
   entrypoints, frozen locking, dependency requirements and all outbound paths.
   A failure or unsupported path must remain visible; no replacement executable
   or locally reimplemented installer counts as backend reuse.
2. Native macOS behaves correctly under the same shell/exec/lock tests. Run on
   an actual macOS host, recording OS, architecture, compiler, source/binary
   hashes and artifacts. Linux Zsh, cross-compilation and workflow YAML alone
   are insufficient. Extend platform cases to argument edge cases, working paths
   with spaces, symlinks, process termination and installation concurrency.
3. The actual public `oyzu::broker` implementation can supply backend metadata,
   archives and verification material while upstream credentials stay on the
   host. Link the production crate; do not replace its decisions with a synthetic
   broker. Controlled origin servers may provide real archives and test-only
   credential canaries. Verify source-prefix restrictions, redirect
   reauthorization, error sanitization, request/byte limits, unavailable routes,
   denied cached selections and tampered content.
4. The production executor's network boundary contains backend and child-process
   acquisition. Run the real backend inside the enforced worker with a private
   broker channel. Attempt direct IP, DNS, proxy-environment, redirect, alternate
   client and spawned-process bypasses, and attempts to read host-only canaries.
   Unsupported native enforcement must fail closed, not become an online retry.
   Distinguish the host OS from the worker OS and report each tested combination.
5. Connect these layers end to end: authorized acquisition succeeds through the
   real broker, frozen execution succeeds without acquisition, and denial cannot
   silently switch source or lock identity. Preserve standalone operation,
   Oyzu-owned files and the prohibition on a separate mise executable.

## Execution boundaries

The public repository already contains a source-scoped broker and an offline
Docker executor. Their code and tests are the implementation under qualification.
Private platform authorization is a separate service boundary; this work must not
publish private platform details or substitute a local flag for server-side
authorization evidence. Any unavailable service or host remains an explicit gap.

Use an isolated GitHub Actions workflow for native macOS and retain its run and
artifact identities. Keep experimental drivers under `tooling/mise-experiment`.
Keep ecosystem-specific behavior outside the shared production engine. Necessary
production corrections require focused regression tests and the repository's
Rust/task/documentation checks. No design is marked accepted by a passing test.

## Completion record

The final audit below records an outcome and its limits for each hypothesis.
Completion means the representative qualification experiment was executed and
reported, including failures. It does not mean every backend works, all outbound
paths have been mediated, or the production integration is ready.

| Hypothesis | Measured outcome | Qualification limit |
| --- | --- | --- |
| Oyzu files drive multiple backend families | Core Node, Go, Java, Python, Aqua/jq and native npm/Prettier install and execute; unchanged asdf Go plugin executes but its acquisition is denied | Broad backend cases are Linux x64; scripted acquisition, general version discovery and arbitrary package graphs remain unqualified |
| Native shell/exec behavior is portable | Linux 36/36 and macOS ARM64 35/35; Windows hardlink mode 32/33, file mode 31/33 | Windows child termination fails; `.cmd` argument fidelity fails; no macOS x64 or MSVC claim |
| Real broker supplies acquisition safely | Approved metadata, archives and attestations succeed; denied sources, redirects, tampering, credential leakage and exact resource limits are exercised | Source URL authorization does not authorize cached tool selection; no deployed corporate service was available |
| Real executor contains acquisition | Windows-host/Linux-worker direct IP, DNS, proxy, child-client and host-file attempts are denied; unavailable image fails closed | No native Windows/macOS sandbox qualification; no universal sandbox-escape claim |
| Frozen execution preserves identity end to end | Frozen execution makes no acquisition and preserves Oyzu files; fresh checksum rejection passes | Cached installation accepts a changed lock digest: this hypothesis is falsified for the prototype |

The reuse recommendation is conditional: keep Oyzu's TOML and lock as the public
contract and use the linked mise library behind an adapter. Oyzu must own cached
installation identity and selection authorization, broker transport policy and
Windows process lifecycle. A small maintained upstream patch is feasible for the
tested paths; mise's backends are not uniformly compatible with HTTP URL routing.
No separate mise executable or mise project configuration is needed by the
passing cases. None of these findings authorizes production tool installation.

### Final evidence audit

The [final broker audit](../../../tooling/mise-experiment/results/production-broker-final-audit.json)
records 25 passing cases and one failed cached-identity case. Its nonzero exit is
intentional evidence of that failure, not a passing qualification run. The report
contains source/binary/image hashes, exact fixture identities, actual backend
version output, installation paths and runtime environment observations. Go,
Java, Python and Aqua observations come from the installed tools; Java's
`JAVA_HOME` is read by a compiled Java program. Node and Prettier execute from the
explicitly locked dependency toolset. The [final asdf run](../../../tooling/mise-experiment/results/production-broker-asdf-final.json)
independently repeats the scripted acquisition denial with the current frontend.

The [cached-identity failure](../../../tooling/mise-experiment/results/production-broker-cached-identity-initial.json)
also reproduces in the final audit: after installing real Node, changing the
distribution digest to 64 zeroes still executes `v22.14.0` with zero acquisition
requests. The prototype has no installation receipt binding that cached directory
to the selected distribution digest. Fresh-download checksum verification does
not establish cached integrity. The public broker authorizes acquisition URLs;
it provides no cached-version revocation service. The prototype's local denial
flag is not evidence of corporate authorization.

The [production Fetcher boundary tests](../../../tooling/mise-experiment/results/production-broker-limits.json)
allow 4,096 requests and reject the next; allow a 128 MiB response and reject one
byte more; allow 1 GiB cumulatively and reject the next byte; and stop after five
redirect attempts. They call the actual production `Fetcher`, with independent
origin request counts. The [unavailable-image test](../../../tooling/mise-experiment/results/production-executor-denial.json)
calls the actual executor and verifies rejection with an empty output directory,
without pulling an image or falling back to host execution.

The final worker audit attempts reads of the host canary path, guessed private
mount paths, traversal from the broker mount and `/proc/1/root`. All fail. Worker
and PID 1 environments are scanned on the host for the generated credential
canary; it is absent. This supplements the actual acquisition and network bypass
tests, rather than treating the process-local HTTP guard as a security boundary.

The [final native macOS artifact](../../../tooling/mise-experiment/results/macos-lifecycle.json)
passes all 35 cases on macOS 26.6.2 ARM64 using Rust 1.95.0
(`59807616e`, 2026-04-14), at commit
`aecf0442aad4235fdb32a6c550e7b50a02f0fe06`:
[CI run 36955882473](https://github.com/micahlmartin/oyzu/actions/runs/36955882473),
artifact `11206151767`. Normalized harness hashes match the executed commit.
This includes arguments, unusual paths, symlink selection, termination,
concurrent installation and Bash/Zsh activation lifecycle. The [final Windows file-mode run](../../../tooling/mise-experiment/results/windows-file-lifecycle.json)
reproduces both Windows failures with the same frontend source; the hardlink and
Linux lifecycle results below use that source too. Windows uses a junction for
the linked-project case; Unix hosts use a symlink.

Final verification also passed the production crate's 71 Rust tests,
`cargo clippy --locked --all-targets -- -D warnings`, `cargo fmt --all -- --check`,
and eight real CLI development-task scenarios. These do not qualify complete
builder scenarios or production installation. The experiment's independent Rust
driver passed Clippy/format checks; the Linux frontend passed its build and
checks with the already documented upstream lint exception. Production source
was not changed by this qualification.

### Remaining product work

Before integration, define and verify immutable installation receipts and cache
reuse rules, authoritative corporate selection/revocation policy, Windows child
cleanup and a supported shim layout. Decide how native clients and scripts reach
the broker without exposing credentials or bypassing lock integrity. The tested
asdf plugin cannot do so through mise's HTTP replacement setting alone.

General backend version discovery (including Go's Git subprocess), plugin identity
inside the lock, arbitrary npm dependency/lifecycle graphs, automatic multi-tool
lock creation, source-built tools, multi-workspace process-global state, shared
store capacity and cross-volume frontend upgrades remain unqualified. Broader
backend coverage on native macOS/Windows and native sandbox enforcement are also
outside the demonstrated results. These are explicit limitations, not inferred
successes from the representative matrix.

### Historical first production-broker evidence

The [Windows-host/Linux-worker run](../../../tooling/mise-experiment/results/production-broker-windows-host.json)
passes ten cases through the actual `oyzu::broker::Session` and
`oyzu::executor::execute_with_mounts` APIs. The independently compiled test driver
links the production crate; broker and executor source is not copied into a fake
implementation. A credential-free loopback HTTP bridge in the worker adapts mise
requests to the existing production spool transport.

Real Node metadata creates `oyzu.lock`, its archive installs through the broker,
and frozen Node execution makes no acquisition request. The origin observes the
host-only test credential on approved requests. Canary scanning finds it in no
worker-visible workspace, output or spool file. An upstream error body and
response cookie containing that canary are not exposed to the worker.

A default-network positive control reaches the same origin that the enforced
worker cannot reach by raw TCP, spawned curl or an explicit proxy. The broker
rejects an unapproved path and reauthorizes a redirect before contacting its
denied destination. Inside the worker, the root filesystem is read-only,
capabilities are empty, no-new-privileges is set, only loopback exists and the
Docker socket is absent. This proves this host/worker combination, not a native
Windows process sandbox or a deployed platform authorization service.

The macOS workflow ran at commit `7f39a651b516982ae8e90e7213ed6ef3794292af`
([run 36951600718](https://github.com/micahlmartin/oyzu/actions/runs/36951600718)).
Its runtime step passed; [the inspected artifact](../../../tooling/mise-experiment/results/macos-arm64.json)
records 30 passing cases on native Darwin ARM64. Normalized harness hashes match
the executed Git commit. Both real Node distributions, Bash/Zsh lifecycle with
duplicate PATH entries and user edits, lock ownership, direct exec, shims and
concurrent installation are covered. This does not prove macOS x64 or native
macOS network enforcement. At this checkpoint, additional platform edge cases,
the broader backend matrix, further attack cases and final regression remained
open; the final audit above records the subsequent results.

### Expanded core-backend and attack evidence

The [expanded Windows-host/Linux-worker run](../../../tooling/mise-experiment/results/production-broker-core-linux.json)
passes 20 cases using the actual production broker and executor. Beyond the Node
baseline, the real core Go backend installs Go 1.24.1 and reports the expected
version and `GOROOT`. The core Java backend installs Temurin `8.0.442+6` and runs
the real JVM. Java's additional metadata lookup passes through the same broker,
using upstream's existing URL-replacement setting before the experimental HTTP
route guard. Neither frozen execution changes `oyzu.lock` or acquires content.

These two additional cases use exact Oyzu locks populated from separately
provisioned upstream release metadata. They do **not** establish backend version
discovery, cross-platform Go/Java installs, or dependency resolution. Archive
digests, metadata digests, source identities, production source hashes, driver
identity and worker image identity are recorded with the run. Acquisition routes
remain host-supplied; no mise project configuration or lockfile is introduced.

Additional attacks cover external UDP DNS traffic, a socket attempt from the
installed Node runtime, encoded path traversal variants, a corrupted archive and
an unavailable source. The corrupt installation fails checksum verification;
neither failed installation becomes executable or changes the lock. Credentials
remain absent from worker-visible files and reports.

The [initial Go attempt](../../../tooling/mise-experiment/results/production-broker-go-initial.json)
ran out of space while retaining the preceding Node fixture in the executor's
512 MiB scratch filesystem. A [Windows-backed output-store retry](../../../tooling/mise-experiment/results/production-broker-go-output-timeout.json)
exceeded the harness's 240-second installation timeout.
The passing cases release the completed tool fixture before installing the next
one in scratch. This qualifies separate one-tool installations, not shared-store
capacity or output-store performance. The production resource limits were not
relaxed.

The expanded Rust frontend and HTTP patch built and passed the experiment's
Clippy/format checks on Linux (with the previously documented upstream
`collapsible_match` lint exception). The [Linux Node regression](../../../tooling/mise-experiment/results/linux-expanded-core-regression.json)
again passes all 31 cases. The [native Windows regression](../../../tooling/mise-experiment/results/windows-expanded-core-regression.json)
passes all 28 existing cases against the expanded frontend. The [expanded native macOS run](../../../tooling/mise-experiment/results/macos-expanded-core-regression.json)
passes all 30 existing cases at commit `1fa895982a9a184c5c01cb8f83c8a1b871ac38ae`
([CI run 36953659752](https://github.com/micahlmartin/oyzu/actions/runs/36953659752),
artifact `11205018271`). Normalized harness hashes match that executed commit.

### Python provenance evidence

The [Python expansion](../../../tooling/mise-experiment/results/production-broker-python-linux.json)
passes 22 cases, including real CPython 3.12.9 installation and frozen execution.
The backend verifies the independently locked archive digest and genuine GitHub
artifact attestations. Sigstore's signed trust metadata is fetched from its
public TUF repository by the **actual host broker**, through an approved source
route. The worker remains offline. Response identities and digests are included
in the evidence; metadata is validated against the verifier's embedded trust
root, not trusted merely because the broker delivered it.

A second installation receives the same valid archive and a bundle whose DSSE
signature has been modified. It fails with `DSSE signature verification failed:
no valid signatures found`. Neither the upstream attestation setting nor the
cryptographic verifier was disabled. Existing upstream URL replacements route
both GitHub API and TUF requests; the bridge now preserves query strings needed
by the attestation API while the production broker still authorizes each URL.

The [first Python attempt](../../../tooling/mise-experiment/results/production-broker-python-cache-initial.json)
failed because Sigstore attempted to create its cache under the worker's
read-only home. The passing run supplies an explicit scratch home/cache. This is
an integration requirement for verification helpers in a read-only worker, not a
reason to weaken verification or make the root filesystem writable.

### Aqua registry evidence

The [Aqua expansion](../../../tooling/mise-experiment/results/production-broker-aqua-linux.json)
passes 23 cases. Mise's real `aqua:jqlang/jq` backend installs the official jq
1.7.1 Linux binary through the broker, then executes its version command and a
JSON expression without further acquisition or lock changes. The backend uses
the registry snapshot baked into the pinned mise source; its metadata names
`aquaproj/aqua-registry` commit `de88b84179743a8f44ad9f279a9dc4522e25f371`.
This covers one Aqua package and platform, not every registry verification mode.
The [initial jq test](../../../tooling/mise-experiment/results/production-broker-aqua-initial.json)
installed successfully but failed on an invalid test expression. Parenthesizing
the arithmetic corrected the harness; no backend code was changed for that fix.

### Scripted acquisition boundary

The [real asdf plugin test](../../../tooling/mise-experiment/results/production-broker-asdf-denial.json)
fetches the unchanged Go plugin at commit
`a75b761963d8e6eda1a185c73476da8a75b8d300` through the production broker, verifies
its recorded archive digest and preserves its MIT license. Mise invokes its real
`bin/download` script. Curl exits with code 6 inside the offline worker; no Go
archive request crosses the bridge, the lock remains unchanged and subsequent
execution reports the tool is not installed.

This is a **negative containment result**, not successful scripted acquisition.
The plugin's hard-coded curl URL does not use mise's HTTP replacement setting.
The asdf backend also explicitly declares that lockfile URLs do not apply to its
scripted downloads. Successful corporate acquisition, binding downloaded bytes
to the Oyzu lock digest, and recording the plugin identity inside the Oyzu lock
remain open. The host fixture's separately pinned plugin does not prove those
requirements.

### Native npm backend and locked tool dependencies

The [native npm run](../../../tooling/mise-experiment/results/production-broker-npm-linux.json)
passes 24 cases, including Prettier 3.5.3 installed by mise's real npm backend and
the native npm CLI from Node 22.14.0. Both tools appear in Oyzu TOML and
`oyzu.lock`; the package distribution declares its dependency on `core:node`.
The experimental adapter orders selected tools by those locked dependencies and
passes the installed dependency toolset to upstream installation APIs.

Npm's backend uses its checksum option rather than `PlatformInfo`'s download URL.
The adapter supplies the digest from Oyzu's lock to that existing verifier. The
registry bridge preserves upstream version/dependency/integrity declarations
while adapting the tarball transport URL to its broker route. Evidence records
both original and delivered metadata digests. No npm credential enters the
worker and neither a mise configuration nor a mise lockfile is introduced.

The [negative-check expansion](../../../tooling/mise-experiment/results/production-broker-npm-negative-linux.json)
records npm 10.9.2, frozen Node/Prettier execution without acquisition, rejection
of missing and cyclic tool dependencies, and a real checksum mismatch rejected
before package installation. The [Linux dependency-adapter regression](../../../tooling/mise-experiment/results/linux-dependency-regression.json)
passes all 34 then-current cases. Prettier has no transitive npm package
dependencies: this does not qualify arbitrary npm graphs, lifecycle downloads,
scoped registries or other native package managers. Multi-tool locks in this case
come from independently provisioned metadata; automatic multi-tool lock creation
is not implemented by the spike.

### Platform edge cases

The [Linux edge run](../../../tooling/mise-experiment/results/linux-platform-edges.json)
passes 34 cases, adding empty/Unicode/trailing-backslash/metacharacter arguments
and project directories containing spaces and Unicode. Direct execution also
preserves a newline argument. The [Windows edge run](../../../tooling/mise-experiment/results/windows-platform-edges-initial.json)
passes 30 of 31 cases: direct execution and unusual project paths pass, but the
generated `.cmd` shim drops `^` and `!` from one argument. This failure is retained;
the earlier narrow shim result must not be read as general argument fidelity.
The [native hardlink-mode run](../../../tooling/mise-experiment/results/windows-hardlink-platform-edges.json)
passes all 31 Windows cases, including the argument that the `.cmd` shim changed.
The experiment selects upstream's existing hardlink mode; it does not introduce
a replacement parser or invoke a separate mise executable. That mode requires a
compatible same-volume layout and is not a qualified cross-volume or frontend
upgrade solution. The final macOS artifact above covers the additional edge cases.

The [Linux lifecycle run](../../../tooling/mise-experiment/results/linux-lifecycle.json)
passes 36 cases, including selection through a symlink and termination of the
executing tool when the frontend PID is terminated. The [Windows lifecycle run](../../../tooling/mise-experiment/results/windows-lifecycle-initial.json)
passes 32 of 33 cases in hardlink mode: junction-based selection passes, but
terminating the frontend leaves its Node child running. A continuing heartbeat
proves the child survived; the harness then terminates that fixture child.
The prototype's Windows `Command::status` path therefore does not yet provide
process-tree cleanup. This is separate from the production Docker executor's
worker isolation and must not be described as qualified native cancellation.

### Outbound-path inventory and unqualified paths

Pinned-source inspection already shows why additional backends need independent
qualification rather than inheriting the Node result:

| Backend family | Observed pinned behavior | Required case |
| --- | --- | --- |
| Core Go | Version listing invokes `git ls-remote`; installation constructs a mirror URL and fetches its `.sha256` companion | Mediate Git metadata and checksum acquisition, not only an archive override |
| Core Java | Even a locked archive URL is followed by a Java-metadata lookup for installation layout | Supply approved metadata as well as archive bytes |
| Core Python | Precompiled installation has lock-integrity and provenance-verification branches | Genuine and invalid attestations exercised through the broker; version discovery remains open |
| Aqua registry | Registry is baked into the pinned source from `aquaproj/aqua-registry` commit `de88b84179743a8f44ad9f279a9dc4522e25f371`; locked asset names are checked against that registry | Real jq acquisition and frozen execution pass; other verification modes remain open |
| Script/plugin backend | `asdf-community/asdf-golang` at `a75b761963d8e6eda1a185c73476da8a75b8d300` invokes curl against a hard-coded HTTPS archive/checksum URL; mise's asdf backend explicitly delegates downloads to scripts | Pin plugin code separately; test native-client containment and binding to the Oyzu distribution digest |
| Native package-manager backend | Explicit native npm mode uses Node/npm from the locked dependency toolset; checksum is a backend option, not a locked URL | Prettier install, execution and bad-digest rejection pass; arbitrary package graphs and lifecycle downloads remain unqualified |

### Python precompiled catalog input bounds

Fork revision `23598b3db1df9f3f1f8e11c35a69a9bff1c5d7a2` bounds both existing
precompiled-catalog fetch paths to 16 MiB of HTTP response data and 16 MiB of
application-level gzip output, with one decoded overflow-probe byte. Oversized,
invalid UTF-8, truncated and checksum-corrupt catalogs fail before selection.
The upstream platform/flavor/version ordering is unchanged. This is not a bound
on total process memory or an artifact extraction limit.

Linux passed the decoder regression, scoped strict library/example Clippy,
formatting and 22 existing embedding scenarios. Three-platform validation
[run 37016853259](https://github.com/oyzuai/mise/actions/runs/37016853259)
completed successfully on Windows, macOS and Linux at this fork revision. Read-only upstream catalog observations on 2026-10-02 measured
195,798 decoded bytes for Linux amd64 GNU, 181,436 for macOS arm64 and 158,412 for
Windows amd64 MSVC. They establish compatibility with this cap at that time;
they are not retained provenance or a replay through the Python backend.

Python embedding selection, explicit snapshot provenance, broker integration,
attestation validation and full archive/runtime qualification remain outstanding.
The root CLI still imports no mise dependency. The fork inventory is consistent,
but its separate sensitive-change review gate requires current-head approval
from @micahlmartin; owner assignment and passing tests do not supply that approval.

### Embedded Python locked-artifact retention

Fork revision `4595732a494af3afc442945d671844210c41e627` rejects a missing or
substituted locked filename in the explicit-target precompiled catalog path when
an embedding context is active. It reuses upstream selection and checks the
returned filename against the exact locked identity. Unlocked selection retains
upstream ordering; ordinary nonembedded refresh retains upstream fallback.
Attestation enforcement is unchanged. Direct locked-URL installation and the
future Oyzu worker path still require separate qualification.

Both Python catalog regressions pass locally on Linux, including exact older-build
retention, unlocked newer-build selection, missing/empty/wrong-version denial and
ordinary fallback compatibility. Scoped strict library/example Clippy, formatting,
22 existing embedding scenarios and compliance guard checks also pass. Native
validation is [run 37020409067](https://github.com/oyzuai/mise/actions/runs/37020409067),
completed successfully on Windows, macOS and Linux at this revision. These tests
do not establish a complete Python
resolver, publisher verification, production admission or distribution approval.

### Python version discovery boundary

Source review at fork `bb56ea99001b7b743738a564192b9ff9bebd1f4d` confirms
that `PythonPlugin::_list_remote_versions` combines PyPy discovery with a
host/settings-dependent precompiled CPython cache. Its source-build branch can
provision and execute python-build. The embedded exact-version catalog API avoids
those paths; it is not yet a general resolver.

`fuzzy_match_versions_pep440` specializes prerelease filtering around the existing
fuzzy matcher. It is not a complete PEP 440 specifier evaluator. The shared Node
embedding selector interprets ranges with npm semantics and therefore must not be
used to claim Python native constraint support. TM-03 still requires explicit-target
CPython candidate discovery, upstream ordering/prefix reuse, correct intersection
of Python native constraints and retained input provenance. Any added dependency
must pass the existing source/notice and shipping-graph gates. Until that resolver
exists, unsupported selectors must remain errors rather than silently dropping
native constraints or consulting ambient Python/PyPy state.

A concrete reuse candidate is the published
[pep440_rs 0.7.3 API](https://docs.rs/pep440_rs/0.7.3/pep440_rs/), which exposes
`VersionSpecifiers::contains` for conjunctions such as `>=3.10,!=3.11.*,<3.14`.
Evaluate this against [PEP 440](https://peps.python.org/pep-0440/) and native
builder inputs instead of extending npm range parsing. The
[published package source](https://docs.rs/crate/pep440_rs/0.7.3/source/) lists
Apache and BSD license files. This is candidate discovery only: no dependency
has been added, no license alternative selected, and no distribution approved.
Before adoption, retain the exact package checksum, original notices and feature/
dependency graph; qualify Python-specific operators, exclusions, malformed inputs
and the admitted stable-CPython policy. Package-version API documentation should
be used instead of assuming the repository's older README matches the release.

The [candidate package evidence](python-constraint-candidate.json) records the
0.7.3 archive checksum matched against crates.io, package-declared license
expression, source revision, direct dependency requirements and original notice
hashes. The archive and both unmodified license texts were retained outside Git.
No code was executed or adopted during this read-only package inspection. The
package declares `Apache-2.0 OR BSD-2-Clause`; neither alternative has been chosen.
Its required direct dependencies are once_cell, serde, unicode-width and unscanny;
optional dependencies are not an audited shipping graph. Compiler compatibility,
resolved dependency notices and semantic conformance remain unverified.

The isolated candidate probe now passes 15 matching cases and six malformed
specifier cases on Windows GNU with Rust 1.94.0, including compatible releases,
wildcard exclusions, epochs, local/post/prerelease behavior and rejection of npm
caret/OR syntax. A locked offline rerun and strict Clippy pass. The retained
[first-party probe](../../../tooling/mise-upstream/pep440_probe.rs) and experiment
manifest/lock text in the candidate evidence permit reproduction: create a new
external directory, write those exact manifest/lock texts as Cargo.toml/Cargo.lock,
copy the probe to src/main.rs, and run `cargo run --locked` there. The registry
cache must be provisioned before adding `--offline`.

This experiment resolves 11 dependency packages; it is not the fork's shipping
graph. No dependency was added to either product. Transitive notice review,
Linux/macOS verification, complete constraint conformance and resolver integration
remain outstanding. Stable CPython admission must still be applied separately
from specifier matching; passing prerelease matching cases does not admit them.

The same locked candidate experiment also passes on Linux x86-64 with Rust 1.95.0.
The candidate evidence now records 23 original top-level notice files from all
11 resolved packages, including path/size/hash and an externally retained notice
bundle digest. No package lacked a top-level notice candidate. This filename-based
collection does not audit additional source notices, license exceptions, optional
feature graphs or release obligations. No license alternative was selected and
macOS qualification was pending at that collection; the subsequent native result is recorded below.

The `Python constraint candidate` workflow reconstructs this retained experiment
under the runner's temporary directory on Windows, macOS and Linux, verifies the
probe source hash, and runs the pinned Rust 1.95.0 semantic probe, locked offline
Clippy and formatting checks. It does not edit either product's Cargo graph. Native
workflow [37027499477](https://github.com/micahlmartin/oyzu/actions/runs/37027499477)
passed on all three hosts at `87973b366e67cd72f116cc2af84404a14d9e623b`. This does not convert the candidate to
an approved dependency or extend the probe's 21-case coverage into a full audit.

The unmodified published 0.7.3 source archive also passes all 35 default-feature
library unit tests on Windows GNU (Rust 1.94.0) and Linux (Rust 1.95.0). Linux ran
locked and offline with networking disabled after explicit dependency provisioning;
the first offline attempt lacked an optional dependency's registry entry and did
not run tests. The generated test lockfile is retained separately in the candidate
evidence because it includes development/optional resolution beyond the consumer
probe graph. This does not extend the consumer notice audit to that larger graph.
The source and original licenses remain outside the product repositories. The
published suite has not run on macOS; the separate 21-case consumer probe has.
