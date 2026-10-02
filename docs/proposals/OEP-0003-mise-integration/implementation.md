# Implementation sequence and production acceptance

Normative draft companion to [OEP-0003](README.md). This is a work breakdown for
the proposed implementation, not a report that these work packages are done.
The experiment is complete. Staged implementation was authorized on 2026-10-02;
TM-01 now has an initial lock structure/identity inspector with recursive
installation keys; TM-04 has a no-follow payload tree observer and bounded
verified tar/gzip staging materializer and full-selection receipt/content matching.
Streamed verified blob caching, atomic per-install publication and cooperative OS leases now have an initial
implementation. Durable process-lease observations now record selected identities
and clean up on ordinary release. Explicit conservative reaping of guarded final
lease records is implemented; staging recovery/prune, consumer lifecycle and admitted
layout admission remain absent. Initial data-only tar/gzip and bounded ZIP32 layout finalization
now binds a caller-admitted descriptor, strips an archive prefix, applies smaller
extraction limits, applies bounded declared Unix executable permissions and authors a candidate receipt; it does not provide compiled
backend descriptors or prove publisher verification. These do not
complete either package or qualify a production backend. Other packages remain
unimplemented unless existing components are explicitly identified as foundations.
TM-01 now has draft versioned layout and receipt JSON schemas with shared
invalid-shape fixtures checked by the schema validator and Rust runtime. A draft
backend descriptor schema and shape fixtures now encode the specified identity
fields; bounded Rust parsing, shared shape tests and canonical identity inspection
are implemented. Compiled descriptor admission remains outstanding. Worker
and selection-grant schemas remain outstanding.

TM-03 now has a first frozen selection boundary: given an already resolved
workspace, working directory, profile, request digest and platform, it chooses
the nearest physical locked scope and rejects stale requests or an unavailable
target without parent fallback. Effective request projection, alias/version
resolution and transactional lock editing remain unimplemented. No worker or
execution path is enabled by this boundary alone.

## Dependency order and deliverables

Each package is a reviewable change with an executable acceptance result. Do not
enable an advertised capability merely because its interfaces or fixtures exist.

| Package | Depends on | Concrete implementation and exit gate |
| --- | --- | --- |
| TM-01: contracts | Maintainer review of this draft | Owned facade/diagnostic types; format-2 parser/serializer and canonical hashing; versioned JSON schemas for receipt, layout, worker, backend descriptor and selection grant; valid/invalid golden fixtures; schema/semantic agreement tests |
| TM-02: source integration | TM-01 and recorded license approval | Public fork Git revision pin, validated provenance and patch register; private embedding module; library-only packaging; notices/SBOM; hostile ambient mise fixture and no project discovery; MISE-09/15 |
| TM-03: exact resolver | TM-01/02 | Alias canonicalization, scope/profile/native constraints, all-platform dependency DAG validation, explicit update/migration/CAS lock writing; no install during graph validation; MISE-01/08 |
| TM-04: installation store | TM-01 | Content-addressed blobs, secure extraction, receipts/tree manifests, per-key process locks, leases, recovery/quarantine/prune; changed-lock cached regression fails safely; MISE-05/07/13 |
| TM-05: worker and broker | TM-02/04 | Same-binary private-channel worker; streaming host broker, artifact handles and target layout plans; actual executor containment, limits/cancellation/credential tests; MISE-04/09/13 |
| TM-06: core prebuilt admission | TM-03/04/05 | Node then Go, Temurin Java, PBS Python and jq, each exact/range and each admitted platform; verification and foreign-target layout parity; MISE-12 |
| TM-07: exec and shims | TM-03/04/06 | Native launch descriptors, Windows Job Objects, Unix supervisor, versioned executable shims, typed interpreter entrypoints, direct exec and lease retention; MISE-03/10 |
| TM-08: shell lifecycle | TM-07 | Upstream-generated Bash/Zsh/PowerShell hooks, independent reversible state, local invalidation, missing-selection behavior, profile editing; MISE-02/06/16 |
| TM-09: managed selection | TM-01/05/07 | Public authorizer client/conformance server, authenticated agent IPC, signed grants, expiry/logout/revocation, policy capability negotiation and mediated-launch mode; MISE-11 |
| TM-10: npm tools | TM-03/05/07 | Exact Node/npm, captured native transitive closure, registry rewrite evidence, offline installation, denied lifecycle downloads and Windows script descriptors; MISE-12 plus OEP-0017 manager checks |
| TM-11: builder handoff | TM-04/06/09 | Builder-provided typed requirements; immutable tool materialization and plan/bundle identities; no ambient shell/default compiler; MISE-14 |
| TM-12: release and update | All advertised packages | Complete target/shell/backend matrix, benchmark comparison, fault-injection audit, package inspection, real managed service/connector gate, [upstream update procedure](upstream-maintenance.md), monitoring and protected review gates, documented unsupported cases and two-repository update/rollback rehearsal; MISE-15 |

TM-04 can proceed independently of the upstream import using owned test records
and real/synthetic archives. Synthetic fault fixtures verify a store algorithm;
they cannot count as successful backend installation. TM-09's public conformance
server needs no private code, but managed release additionally needs actual
service interoperability. Native npm and builder handoff may land after core
development tooling behind explicit capability gates. Such an intermediate
release must say which capabilities are absent; it does not complete this OEP.

Each implementation PR names its package, acceptance IDs, affected contracts,
measured host/worker/target and remaining failures. Update
[tool-management status](../../tool-management-status.md) with evidence, not a list
of functions added. Tests replacing genuine backend behavior with a fabricated
successful executable do not satisfy any backend gate.

## Fork changes to implement

Do not merge the spike wholesale. Convert its four experimental seams and add
only the additional seams required by production contracts:

1. A single private embedding constructor accepts fully resolved configuration,
   frontend identity, filesystem roots and explicit disabled-feature flags. It
   bypasses mise discovery and owns once-only settings initialization.
2. HTTP callbacks carry typed broker requests; no environment URL map or policy
   flag. Review every upstream path that bypasses that client and disable it or
   route it through an admitted native adapter.
3. Separate target-aware artifact/layout planning from executable post-install
   behavior for admitted archive backends. Return the finite layout record;
   compare it against existing upstream install behavior on matching targets.
4. Environment/path output preserves owned occurrence information and duplicate
   entries; expose structured data before upstream shell rendering.
5. Frontend/shim identity is an explicit Oyzu executable reference. Windows
   generic `.cmd` wrappers are never the product shim implementation.
6. Installed-version discovery consumes only facade-provided verified roots.
   Disable upstream fast paths that bypass receipt validation or choose another
   cache directory. Installed status is an observation, not authority.

Keep patches separated by responsibility, with upstream source references and
tests. Do not promise the experiment's four-file patch remains the final patch
size. Reject an upstream update that broadens unmediated behavior until its
descriptor and tests are updated.

## Acceptance matrix

Acceptance IDs are defined once in [the proposal](README.md). These rows specify
the required observable outcomes rather than redefining those IDs.

| IDs | Fixture/action | Required observation |
| --- | --- | --- |
| MISE-01/08 | Root Node 22, nested Node 24, sibling Node 22; profiles; hostile ambient mise files/env; exact and range requests | Correct scoped version, stable unchanged lock, no mise config reads/writes, no separate mise process, no sibling contamination |
| MISE-08 | Missing/stale/format-1 lock; platform expansion; conflicting alias/duplicate IDs; two versions in one dependency closure; cycle; concurrent external lock edit | Stable specific error before affected install; explicit migration/update only; original lock bytes retained on failure |
| MISE-07 | Install real Node, change only archive digest; then separately change backend/layout/dependency and mutate installed executable | Cached exec/activation/shim/build deny; no reusing version directory; no automatic public download/relock; dependent snapshots invalidated |
| MISE-05 | Two processes install same/different keys; terminate at each download/extract/receipt/rename step; power-loss simulation; concurrent prune and exec | At most one valid published key, no partial readers, old version usable, live leases retained, abandoned staging safely reclaimed |
| MISE-13 | Absolute paths, traversal, symlink-chain escape, hardlink escape, zip duplicates, case collisions, ADS, devices, bombs and malformed plan | Contained failure, no outside file change, no receipt publication or post-install process |
| MISE-04 | Actual broker/executor plus reachable external positive control; HTTP/Git/curl/npm, raw IP, DNS, proxy env, redirect and installed child attempts | Only approved broker routes succeed; no upstream credential in child env/files/logs/spool; no network fallback |
| MISE-04 | Exact request/response/session/redirect limits, truncated streaming, cancelled download, lease rotation, missing worker image | Defined cap enforced, bounded memory, no committed partial blob, sanitized diagnostic, fail closed |
| MISE-09 | Concurrent workers for different roots/profiles; worker panic, protocol oversize/mismatch, inherited env/descriptor attack | No shared settings or leaked handles; rejected malformed request; unaffected other operation |
| MISE-12 | Real core and registry artifacts with publisher metadata, genuine and tampered Python attestation | Actual versions/env/entrypoints match; verification strength retained; cryptographic failure cannot degrade to checksum-only |
| MISE-12 | Npm tool with transitive packages and typed Node launcher; second frozen run; package needing undeclared lifecycle download | Complete captured identities, zero replay acquisition; undeclared lifecycle fails; no ambient npm/Node used |
| MISE-02 | Enter/switch/leave/nested scopes; duplicate PATH; absent/empty vars; user PATH/scalar edits; two terminals; broken lock | Correct reversible owned delta, no unrelated edit loss, independent state, previous project not left selected on error |
| MISE-03/10 | Empty/Unicode/newline/quote/backslash/metacharacter args; spaces/Unicode/UNC paths; command collisions; exit codes; Ctrl-C/termination | Exact argv and cwd, native extension behavior, explicit collisions; Windows entire child tree stops when supervisor dies |
| MISE-03/05 | Interactive Unix child group; signal forwarding; frontend upgrade and shim hardlink/copy fallback; active old version during update | Correct terminal control/lease lifetime; no accidental new-version dispatch for running command; no separate mise image |
| MISE-11 | Valid/expired/wrong-tenant/subject/audience/key/epoch/digest grant; offline bound; clock rollback; restart/logout/revocation | Only exact current permitted selection starts; no unsigned/toy-flag acceptance; no managed-to-standalone transition |
| MISE-14 | Same hermetic build from differing interactive PATH/env; changed tool receipt; missing target distribution | Ambient differences irrelevant; tool identity affects prepared input; missing frozen platform fails without source edit |
| MISE-16 | Repeated profile install/remove, user-edited block, symlink target, concurrent edit, malicious literal env and stderr | Unrelated bytes preserved, conflicts explicit, valid shell quoting, secrets redacted, inspection JSON schema stable |
| MISE-15 | Upgrade source pin and rollback; inspect release archive/process trace with mise absent | Reviewed backend compatibility only, complete notices and identities, no packaged/invoked mise CLI |

## Required host, execution and target coverage

Initial native target release tuples are Linux amd64/gnu, Darwin arm64/native and
Windows amd64/msvc. Qualify Linux arm64/gnu, Darwin amd64/native, Linux musl and
Windows arm64 separately before advertising them. All three OSes are requirements;
one missing OS blocks the initial cross-platform release, not merely a footnote.
Pin native CI runner image, compiler, shell and fixture versions in test metadata.
Minimum supported host baselines proposed here are Ubuntu 22.04, macOS 14 and
Windows 11 24H2; run a minimum-baseline job plus a current-image job for release.

Each initial core backend must pass native version/entrypoint/environment tests
on each admitted target. If Python or another backend has no suitable artifact,
that exact tuple is marked unsupported and cannot be selected; it is not tested
using a different implementation under the same identity. Node's two-version
shell/exec suite runs on every host; no Linux Zsh substitution for native macOS.
Native PowerShell/Bash/Zsh versions follow [the runtime contract](runtime.md).

Broker/executor tests record host OS, worker OS/architecture and installed target
separately. Run the Linux acquisition worker through Docker from each supported
host, then native finalization and execution on that host. A Windows-host/Linux
worker test alone does not establish native Windows finalization or process
control. Foreign-target plans must be generated without executing that target's
binaries and validated against native upstream layout results.

Native management of untrusted arbitrary installers remains disabled. The
data-only finalizer and qualified Linux worker are the specified v1 solution;
no implied AppContainer/Seatbelt implementation fills that gap. For managed tools
with unsupported layout/scripts, diagnose the exact missing capability before
any host code runs.

## Performance and resource acceptance

Reproduce paired upstream library versus Oyzu facade measurements in the same
process against installed identical tools, without running a mise executable.
Also measure actual hook and exec overhead including same-binary worker spawn,
receipt verification/cache invalidation and agent IPC. Report cold process/cache
and warm-cache measurements separately; do not claim filesystem cache eviction
unless it was performed. Record build profile, host load, source/binary hashes,
CPU/memory and at least 100 samples after ten warmups for warm metrics.

Proposed release gates on the pinned performance runners: warm unchanged hook
p95 at most 150 ms and paired facade overhead at most the larger of 20 ms or 25%
of upstream p95; no more than 10% p95 regression against the previous released
Oyzu on the same runner. Cold activation and first tree verification are reported
separately with bytes/files scanned, not hidden inside warm numbers. A warm hook
must make zero network requests and zero install/project-task calls. These are
engineering acceptance budgets to validate, not current performance claims.
If unmet, optimize or seek an explicit reviewed budget amendment; never remove
receipt/policy checks or relabel cold samples to pass.

Bound process count to one local tool worker per operation and configured
acquisition concurrency (default two). Do not create an unbounded thread pool
per prompt. Broker memory remains bounded independent of artifact size. Package
size, linked-library dependency count, cold RSS and worker spawn cost are release
measurements; substantial growth requires source/dependency review, not a
speculative second implementation.

## Build integration and rollout

Builder discovery reports static native constraints without executing project
code. Preflight resolves a frozen tool snapshot and obtains verified receipt
handles before native preparation. The planner records selection digest,
installation/tree/backend identities, execution platform and verification
evidence, and copies/materializes immutable tool inputs into the action worker.
It never inherits a developer activation, machine path or writable mise cache.
Requirements for cross-compilation distinguish host tools, execution tools and
produced target artifacts rather than using one platform string for all three.

Retain existing explicitly provisioned toolchain images during rollout. A builder
opts into the new provider through a typed interface only after its own real
build/task suite passes. No automatic reinterpretation of `--image` as an install
request and no removal of existing working provisioned-tool flows. OEP-0004's
zero-config ephemeral snapshot remains recorded and policy-constrained; it does
not permit floating versions inside an already frozen action.

Release stages are explicit capabilities: internal contracts/store tests;
standalone core native tooling; managed prebuilt tooling; admitted npm tools and
builder integration. Until the complete matrix passes, label the capability
experimental and list unavailable commands/backends. Do not silently turn a
managed machine into standalone when a later stage is unavailable.

Rollback selects a retained signed/verified Oyzu release image and its supported
backend descriptors. Keep referenced old images/payloads until leases end. If the
old release cannot read format 2 or a descriptor, report incompatibility and
require an explicit restored compatible lock; do not rewrite the new lock or
delete installed content to simulate rollback. Policy is rechecked independently
of binary rollback. Rehearse with a running old tool, concurrent new install and
old/new shims on every host.

## Required verification and definition of done

Rust implementation changes run `cargo test --locked`,
`cargo clippy --locked --all-targets -- -D warnings`, and
`cargo fmt --all -- --check`. Run the real CLI task scenarios with the compiled
CLI and backend-specific build scenarios when touching builder integration.
Run `node tooling/check-docs.mjs`; schema/fixture checks from TM-01 are an
additional structural check, never a substitute for runtime evidence.

For each acceptance row retain command line, source/release/backend identities,
host/worker/target, fixture provenance, exit/outcome, redacted logs and machine
readable result artifact. CI failures remain failures. A conformance server
proves the public protocol, not actual corporate service availability. Managed
release requires a real supported service and at least one genuine configured
corporate connector, with credential canaries and no-fallback denial tests.

This OEP is implemented only when all MISE criteria pass for the advertised
matrix, source/license review is recorded, public-only builds succeed, user docs
describe actual behavior, and no known experiment failure is represented as a
passing product result. Draft acceptance requires separate maintainer review;
implementation completion cannot self-approve the design.
