# Expanded mise qualification

Status: authorized work in progress; no new backend or enforcement claim yet.
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

Pending: completion of the backend matrix and source inventory; additional native
platform edge cases; the remaining enforced acquisition/attack matrix; final
cross-platform regression and evidence audit. Initial native macOS and real
broker evidence are recorded below. Completion requires actual outcomes for every item above,
not merely adding test scripts or CI configuration.

### First production-broker evidence

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
macOS network enforcement; additional platform edge cases remain open.
The broader backend matrix, further attack cases and final regression remain open.

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
passes all 28 existing cases against the expanded frontend. The macOS rerun is
still in progress; its earlier result applies to the earlier recorded source.

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

### Outbound-path inventory started

Pinned-source inspection already shows why additional backends need independent
qualification rather than inheriting the Node result:

| Backend family | Observed pinned behavior | Required case |
| --- | --- | --- |
| Core Go | Version listing invokes `git ls-remote`; installation constructs a mirror URL and fetches its `.sha256` companion | Mediate Git metadata and checksum acquisition, not only an archive override |
| Core Java | Even a locked archive URL is followed by a Java-metadata lookup for installation layout | Supply approved metadata as well as archive bytes |
| Core Python | Precompiled installation has lock-integrity and provenance-verification branches | Genuine and invalid attestations exercised through the broker; version discovery remains open |
| Aqua registry | Registry is baked into the pinned source from `aquaproj/aqua-registry` commit `de88b84179743a8f44ad9f279a9dc4522e25f371`; locked asset names are checked against that registry | Real jq acquisition and frozen execution pass; other verification modes remain open |
| Script/plugin backend | `asdf-community/asdf-golang` at `a75b761963d8e6eda1a185c73476da8a75b8d300` invokes curl against a hard-coded HTTPS archive/checksum URL; mise's asdf backend explicitly delegates downloads to scripts | Pin plugin code separately; test native-client containment and binding to the Oyzu distribution digest |
| Native package-manager backend | Pending inventory | Capture dependency resolution and lifecycle downloads through approved routes |
