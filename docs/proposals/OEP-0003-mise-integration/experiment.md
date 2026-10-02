# Mise reuse experiment

Status: experiment completed; recommendation remains a draft, not an accepted
design or shipped capability.
Expanded qualification of broader backends, macOS and production broker/network
enforcement is now [authorized and in progress](qualification.md).
Started: 2026-10-01. Related drafts: [OEP-0003](README.md),
[tool acquisition](../OEP-0004-tool-acquisition-and-locking/implementation.md),
and [shell lifecycle](../OEP-0008-environment-and-shells/README.md).

## Hypothesis recorded before implementation

Oyzu can reuse mise's tool selection, version resolution, installation layout,
environment differences, shell integration and executable dispatch in-process,
with a small maintained integration boundary rather than reimplementing those
behaviors. Oyzu TOML and `oyzu.lock` remain authoritative: no generated mise
project configuration, mise lockfile, ambient mise override, or separate mise
executable may determine the selected toolchain.

For managed acquisition, reusable backend knowledge can coexist with Oyzu-owned
authorization and proxied installation. All discovery metadata, archives,
verification material, redirects and subprocess acquisition must be accounted
for. URL replacement alone is not enforcement. Unsupported paths must fail
without public fallback. Standalone use must not require a platform account.

The experiment may falsify these hypotheses. A failure is evidence to report,
not a reason to replace upstream behavior with a fake implementation.

## Pinned input and licensing

Inspect and build upstream commit `da0db43e9398b46bafa95232014708a51120e731`
(package version 2026.9.18). Its root license is MIT, copyright 2025 Jeff Dickey;
retain the complete upstream LICENSE with the external source checkout and any
redistributed source. Inspect dependency and registry notices before including
them. No Oyzu public license is selected by this experiment. Upstream source
stays outside the product source tree; retain reproducible patches separately.

## Experiment and acceptance evidence

1. Map source dependencies and attempt an unmodified pinned library integration.
   Compare the necessary fork patches with extracting internal crates. Record
   compiler, dependency, initialization and API limitations.
2. Read Oyzu TOML and its draft tool-lock schema directly. Supply requests and
   frozen exact selections to upstream logic in memory. Test conflicting ambient
   mise files and overrides; assert no mise project files are generated and the
   Oyzu lock is unchanged during frozen execution.
3. Use two real Node versions in separate project directories. Exercise enter,
   switch, leave, nested selection, independent shells, environment restoration,
   headless exec, argument/exit propagation and shim dispatch. No separate mise
   binary may be built, invoked or bundled. Map to MISE-01 through MISE-03.
4. Reuse the real Node backend through a synthetic corporate route, installing
   a real distribution. Test corrupted content, disallowed redirects, unavailable
   proxy with no public fallback, policy denial of cached tools and a subprocess
   download attempt. Distinguish routing from OS enforcement. Map to MISE-04 and
   TOOL-01 through TOOL-04.
5. Exercise concurrent installation and active-version independence (MISE-05).
   Measure cold and warm hook latency and process behavior (MISE-06), comparing
   equivalent in-process upstream paths without invoking a separate mise binary.
6. Run available Windows/PowerShell, Linux/Bash and macOS/Zsh hosts. Record actual
   host coverage; a Linux Zsh run cannot establish macOS support. Missing host
   evidence remains a limitation, never a passing cross-platform claim.

## Deliverables and decision

Keep a runnable isolated harness, pinned dependency/license inventory, minimal
patches, test results and measured timings. Record negative results as well as
successes. Recommend pinned library, maintained fork, extraction or rejection
based on measured evidence, including acquisition gaps and upgrade surface.
Production integration, broad plugin qualification and source-built tool support
are outside this experiment. Existing build-engine behavior remains separate.

## Results

The experiment completed with [31 passing Linux cases](../../../tooling/mise-experiment/results/linux-x64.json)
and [28 passing Windows cases](../../../tooling/mise-experiment/results/windows-x64.json),
using the real Node backend and real Node 22.14.0 and 24.0.0 distributions. Linux
ran with external networking disabled. The result supports reuse through a
small maintained fork and an Oyzu adapter, subject to the limits below. These
are harness results, not production Oyzu capabilities.

| Criterion | Evidence and scope |
| --- | --- |
| 1. Reuse boundary | Source map below; actual E0624 API probe; reproducible four-file patch; pinned license/dependency inventory |
| 2. Oyzu ownership | Runtime cases for backend metadata to `oyzu.lock`, hostile ambient mise input, exact locked execution, frozen files and no generated mise project files |
| 3. Lifecycle and dispatch | Two real Node versions, independent Bash/Zsh shells, directory transitions, user edits, direct exec and generated shims including quotes and exit codes |
| 4. Managed acquisition boundary | Real Node backend with a synthetic repository; digest, redirect, missing-content, unavailable-route and cached-policy rejection; Linux subprocess egress denial |
| 5. Concurrency and cost | One archive fetch for two concurrent installs; running Node process remains on its selected version; fresh-process and paired in-process timings plus syscall traces |
| 6. Available hosts | Linux verified; native Windows/PowerShell verified; no macOS host available |

The [verification record](../../../tooling/mise-experiment/results/verification.json)
retains exact compiler/check commands, outcomes and log hashes. Machine-readable
runtime results include binary and harness hashes. Negative findings and
unqualified production behavior are part of the outcome, not passing claims.

### Source map and initial evidence

| Responsibility | Pinned source | Integration implication |
| --- | --- | --- |
| Discovery and configuration context | `src/config/mod.rs`, `src/config/settings.rs` | Existing discovery-free constructor is crate-private; settings have a replaceable loader and process-global cache |
| Requests, resolved toolchain, paths and environment | `src/toolset/`, `src/args/backend_arg.rs` | Public request/version constructors and toolset operations permit an in-memory adapter; exact backend identity must be supplied |
| Backend resolution and lifecycle | `src/backend/mod.rs`, `src/plugins/core/node.rs` | Preserve native version semantics and Node layout/verification; installation includes backend subprocesses |
| Environment reversal | `crates/mise-util/src/env.rs`, `env_diff.rs` | Reusable PATH ownership and preservation of user edits; process initialization and internal state names matter |
| Shell generation and hook output | `src/shell/`, `src/assets/`, `src/hook_env.rs` | Activation accepts an executable path, while command names and state names remain mise-specific |
| Shims and Windows argument recovery | `src/shims.rs`, `crates/mise-util/src/env.rs` | Ambient `mise` lookup and Windows `mise x` are explicit patch points; argument recovery already exists upstream |
| HTTP | `crates/mise-util/src/http.rs` | Shared HTTP/metadata clients offer a useful guard point, but cannot mediate arbitrary backend subprocesses |
| Lower crates | `mise-util`, `mise-settings`, `mise-cache-core`, `aqua-registry`, `vfox` | Extraction of these alone does not retain the top-level toolset, Node backend and shell integration |

The exact upstream Cargo lock has SHA-256
`bb1791fd10a90210669acafc468177898c2dcf8af3c6076235022f3a8865d72c`.
The root LICENSE has SHA-256
`4d22d6f9222adef0a45ef895bfdd6f71d6394b1784b1001a4bebb4298b8ddc69`.
These are canonical Git-blob hashes. The initial Windows checkout converted
line endings, yielding different byte hashes; `prepare.py` now reports both
canonical and checkout hashes. Historical runtime harness hashes describe the
exact bytes used for those runs, not necessarily a later checkout's line endings.
The source requires Rust 1.95; the production Oyzu crate remains on 1.94.

On Linux, `cargo check --locked --example oyzu-api-probe
--no-default-features --features rustls,vfox/vendored-lua` compiles the library
but rejects external use of `Config::load_from_config_files` with E0624.
This is a concrete missing initialization API, not proof that every alternative
unmodified integration is impossible. Loading discovered mise configuration and
then clearing its file map would be too late to establish Oyzu ownership.

The Linux-filtered workspace metadata includes 974 packages, including
development dependencies. The [declaration inventory](../../../tooling/mise-experiment/dependency-licenses.json)
records their licenses. Six packages declare MPL-2.0; other expressions include
permissive alternatives to GPL/LGPL. Retain upstream/dependency source notices
and review distribution obligations before shipping. This inventory is not a
complete audit of bundled registry data, native code or runtime plugins.

### Linux behavioral evidence

The [isolated harness](../../../tooling/mise-experiment/README.md) uses the
[integration patch](../../../tooling/mise-experiment/embedding.patch). The patch
currently touches four upstream files: an empty-config constructor, shim
dispatch, a transport route guard, and verbatim PATH joining for a live shell. Initialization also requires the existing
upstream argument state and `backend::load_tools` entrypoint. No separate mise
executable was built or invoked.

Verified cases cover backend version resolution and metadata-to-`oyzu.lock`
creation; real installation of both versions; exact execution, arguments and
exit status; conflicting mise files/environment overrides; parent-directory and
nested-project selection; concurrent Bash and Zsh shells switching in opposite
directions; restoration of PATH and variables while preserving user edits;
shims without activation; frozen lock ownership; a policy denial of installed
content; checksum mismatch, redirect, missing-artifact and unavailable-route
failures; two simultaneous installs making one archive request; and an active
Node 22 process retaining its version while another invocation selects Node 24.

The route guard rejects unapproved initial HTTP destinations and redirects. A
separate curl subprocess cannot contact an external IP under Docker's
`--network none`. This establishes the synthetic Node route and the tested Linux
network boundary. It does not qualify arbitrary plugins, corporate credentials,
Artifactory/Nexus protocol behavior, or native Windows/macOS enforcement.
The policy case uses a local deny-version flag before cached-tool lookup; it
tests the enforcement point, not a production authorization service or cache
attestation scheme.

Debug-build measurements on Linux: a fresh hook with an empty mise cache takes
31.7 ms; subsequent fresh processes measure p50 29.2 ms / p95 34.0 ms. Fifty paired
in-process samples measure upstream environment/diff calls at p50 1.062 ms /
p95 1.539 ms, and those same calls plus the Oyzu TOML/lock adapter at p50 1.349 ms /
p95 2.026 ms. These are measurements of this
unoptimized harness on this host, not release-performance promises or an upstream
CLI benchmark. The cold sample starts with an empty application cache; the OS
filesystem cache was not evicted. No release-build benchmark was performed. Host load
was not controlled; other compilation was in progress during measurement.

The [Linux syscall trace summary](../../../tooling/mise-experiment/results/linux-trace.json)
records one executable for the hook (the linked experimental frontend), and two
for execution (that frontend followed by the installed Node binary). Neither
path makes a network connection. The trace also records filesystem syscall
counts and hashes of the full logs retained with the external fixture run.

Negative findings retained from earlier attempts: the unmodified configuration
constructor is private; omitting upstream argument/installation-state
initialization panics; a mirror path without a trailing slash changes URL join
semantics; and CRLF-converted embedded Bash assets break Linux activation.
The adapter and preparation script now handle these boundaries explicitly.

The [first Windows run](../../../tooling/mise-experiment/results/windows-initial.json)
found an adapter error: `env_with_path` computes a child-process PATH and removes
duplicates. A live shell must preserve user-owned duplicates. Resolution,
installation, direct exec and Windows command-file shims passed, but exact shell
restoration failed. The correction uses upstream split-path metadata and
`PathEnv` ordering with a small `join_verbatim` exposure. Both shell fixtures now
seed duplicate entries explicitly; the final runs pass that case on all three tested shells.

The production crate's Linux verification separately passed 71 Rust tests,
`cargo clippy --locked --all-targets -- -D warnings`, `cargo fmt --all -- --check`,
and the real compiled-CLI development task scenarios. Documentation checks pass
but do not establish product behavior.

The experiment's strict Clippy run first failed on an existing upstream
`collapsible_match` warning in `src/system/templating.rs`. The final harness
passes Clippy with `-D warnings -A clippy::collapsible_match`, plus formatting
checks for the frontend and four patched files. This is a scoped upstream lint
exception, not a claim that the unmodified upstream workspace passes strict
Clippy. Harness lint findings were fixed: environment enumeration uses upstream
`vars_safe`, and process exit occurs only at the frontend's outermost boundary.

### Windows and platform coverage

The [native Windows results](../../../tooling/mise-experiment/results/windows-x64.json)
record 28 passing cases on x64, including two independent PowerShell sessions,
exact PATH restoration with duplicate entries, user edits, both real Node
installations and `.cmd` shim argument/exit propagation. The Windows GNU build
succeeded with a consistent GCC/MSVCRT toolchain after earlier mixed-toolchain
build-script crashes. Those setup failures do not establish a mise Windows
runtime incompatibility.

Windows hook measurements: 71.3 ms with an empty mise cache, then p50 85.5 ms /
p95 94.7 ms over repeated fresh processes. OS filesystem caches were not evicted.
Fifty paired in-process samples put upstream environment/diff calls at p50
10.032 ms and the same calls with the Oyzu adapter at p50 11.355 ms. The Windows debug profile and
host differ from Linux; these are not a cross-platform performance comparison.

The Windows run does not establish an OS network-denial boundary. Only the Linux
container tested independent subprocess egress denial. No macOS host was
available; running Zsh on Linux is not macOS evidence. Production portability and
managed acquisition remain unqualified beyond these explicit tests.

### Recommended integration boundary

The evidence supports a **small maintained fork of the coherent mise tool
subsystem**, exposed through an Oyzu-owned adapter. It does not establish a
stable public mise embedding API. Extracting the lower utility crates alone
would lose the backend, toolset and shell behavior this experiment set out to
reuse. Copying individual internal modules would still make Oyzu maintain their
initialization and dependency relationships. This recommendation remains a draft
for maintainer review; it is not authorization to ship the fork.

The prototype retains upstream version resolution, Node installation and
verification, installed-tool paths, environment differences, shell syntax and
shim generation. Oyzu owns project discovery, TOML parsing, lock serialization,
backend identity, policy checks and route selection. Its independently authored
frontend is about 470 lines, with four small upstream patch points. Project
selection and lock translation remain real integration work even though the
tool-manager behavior is reused.

A small patch does not imply a small dependency or build footprint. This spike
links the broad top-level workspace; trimming unrelated capabilities and
measuring release artifact size remain production packaging work.

Keep the two acquisition decisions separate: the backend determines the
distribution and installation behavior; Oyzu determines which approved source
may supply it. The experiment translates an exact locked distribution into
upstream `PlatformInfo` in memory. The project lock stores identity, platform,
digest, size and a logical source ID, without transient proxy URLs or credentials.
Production managed acquisition still needs Oyzu's broker/connector integration
and OS enforcement for backend subprocesses. The synthetic HTTP guard is not a
replacement for that boundary.

The fixture adapter constructs a Node-specific archive path from the locked
version and host platform. It is not a generic backend-to-broker mapping. Backend
metadata supplies version and checksum evidence during the separate lock-creation
case; a production broker must translate the locked distribution identity to an
approved artifact without teaching the shared engine each ecosystem's URL rules.

Before production integration, qualify additional backends and their subprocess
behavior, complete native platform coverage, benchmark a release build, review
distribution notices and define an upstream upgrade test. Process-global mise
settings/configuration are suitable for the tested per-invocation CLI model;
concurrent projects inside one long-lived agent process have not been qualified.
The narrow TOML and tool-lock schema in this harness is proposed experiment
syntax, not a replacement for the full Oyzu configuration contract.
