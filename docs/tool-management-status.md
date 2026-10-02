# Tool-management implementation status

OEP-0003 remains authorized and in progress. These records preserve measured
results and outstanding gates; they do not redefine the full acceptance matrix.
Builder progress remains in [implementation status](implementation-status.md).

## Separate mise reuse experiment

The [implementation-ready OEP-0003 draft](proposals/OEP-0003-mise-integration/README.md)
now defines the proposed production architecture and work packages from these
findings. It remains a draft. The maintainer subsequently authorized staged
implementation on 2026-10-02; the production checkpoint below distinguishes
delivered contracts from remaining integration and release gates.

The authorized [mise experiment](proposals/OEP-0003-mise-integration/experiment.md)
investigates tool management separately from the builder objective below.
Its harness is outside the production Rust crate. The unmodified pinned mise
library compiles on Linux, but the compile-only external configuration probe
fails because its discovery-free constructor is private. The completed patched
experiment passes 31 Linux and 28 native Windows cases with real Node tools,
Oyzu-owned TOML/locks, shell lifecycle, exec, shims and a synthetic proxy route.
The [expanded qualification](proposals/OEP-0003-mise-integration/qualification.md)
adds native macOS, representative core/Aqua/npm backends and the actual public
broker and Docker executor. Linux passes 36 cases and macOS 35; Windows retains
child-cleanup and file-shim argument failures. Cached lock-digest binding fails,
and scripted acquisition is contained but not successfully brokered. The report
recommends conditional library reuse through an Oyzu-owned adapter, not production
readiness. No installation, shell or managed backend capability is claimed for
the shipped Oyzu CLI.

## Mise production implementation: first contracts (2026-10-02)

The maintainer authorized staged OEP-0003 implementation and designated
@micahlmartin as technical and licensing decision owner. Draft PR
[4](https://github.com/micahlmartin/oyzu/pull/4) contains the proposal, historical
qualification and the first production lock-identity component. Ownership does
not record a completed license audit or approve a particular shipped graph.

`oyzu tools inspect-lock` now performs bounded format-2 structure, record identity
and platform dependency-graph validation and computes complete selection digests.
It rejects cycles, dangling or ambiguous closures, unknown fields, malformed
digests, duplicate records and verification-subject mismatches. Inspection does
not authorize execution, prove installed content, resolve current configuration
or validate backend-specific options. See the [reference](reference/tool-lock-inspection.md).
TM-01 remains partial: production schemas, remaining contracts and integration
are still required. No production mise dependency, installer, activation or exec
path is enabled by this change.

Local Rust 1.94 Windows GNU verification passed: 152 tests, two preexisting
ignored tests, strict all-target Clippy, and nine real CLI task scenarios. Ten
new unit tests and one compiled-CLI test cover lock failures and a separately
computed canonical identity vector. MSVC could not run locally because its
linker is absent; existing CI covers native Windows/macOS/Linux and its results
must be inspected separately. These checks do not qualify the mise runtime.

The [upstream observer](reference/mise-maintenance.md) passed three unit tests and
a real public GitHub observation. The observed latest stable release was
`v2026.10.0` at `bc11f90c74eba23bf0d7350efb540e62fb7d9ffd`; the public fork head
was `b1b8d3e4aed6a0a610fdd1d845df11da470afd08`. The report correctly marked the
production pin absent. The weekly workflow is authored but not scheduled live
until merged into main. Patch provenance, advisory triage, owner approval gates,
branch protection, first source import approval and full qualification remain
outstanding. Do not interpret an observation job's green status as release approval.

## OEP-0003 payload identity and no-follow observation (2026-10-02)

TM-04 now has native handle-based payload observation under `src/tools/store`.
Unix reads use directory-relative no-follow opens; Windows retains ancestor
handles without delete sharing and rejects reparse-point file/directory opens.
The scanner includes directory entries, streams content digests, records internal
symlinks and resolves their chains without following them on disk. Outside
hardlinks, escaping/dangling/cyclic links, special files, portable-name violations
and case collisions fail. This is not yet the extractor, receipt validator,
transaction manager or lease/prune system. The full goal remains active.

Lock inspection now computes recursive installation keys in addition to selection
keys. Tests show a dependency artifact change invalidates both the dependency and
its parent, while adding another platform leaves existing platform keys unchanged.
The public `tools inspect-tree` command exercises the production observation path;
[its reference](reference/tool-lock-inspection.md#payload-tree-observation) states
bounds, errors, no-mutation behavior and the same-user concurrency limitation.

Windows GNU Rust 1.94 passed the full suite (159 tests, two preexisting ignored),
strict all-target Clippy and formatting. In a Linux Docker runner with Rust 1.94,
19 tool unit tests, the compiled tree-inspection CLI test and strict all-target
Clippy passed. Linux tests include actual relative symlinks, special files and a
root-directory replacement proving held reads do not redirect. Windows tests
prove replacement is denied while an ancestor handle is held. Native macOS/MSVC
qualification of this new change is pending the PR jobs. The already-locked libc
package is now a direct Unix dependency for native descriptor operations; its
local crate declaration is MIT OR Apache-2.0. No package version was upgraded and
no first-party license or distribution approval was selected by this change.

## Checkpoint 48: verified archive staging and fork embedding boundary

OEP-0003 TM-04 now includes tar/gzip materialization into empty caller-owned
staging. Locked archive size/SHA-256 are verified into an unnamed private file
before extraction. Writes use the native no-follow handle layer, new files cannot
overwrite existing entries, links are created after regular writes, and the
complete tree is inspected before returning. Entry, file, expansion, depth,
extension and aggregate path/target limits fail closed. Raw tar iteration bounds
GNU extension allocation. Publication, receipts, leases and acquisition remain
unimplemented; this is not a working `oyzu install` command. ZIP/PAX, archive
hardlinks and Windows symlink layouts are unsupported. See the
[materialization contract](reference/tool-lock-inspection.md#archive-materialization-foundation).

On Windows GNU Rust 1.94, the full suite passed (164 tests; two existing worker
fixtures ignored), along with strict all-target Clippy, formatting and all nine
real CLI task scenarios. Four archive integration tests cover byte verification,
empty staging, traversal/aliases, collisions, special files, GNU bounds, gzip
corruption and expansion bombs. Linux Docker Rust 1.94 passed six archive
integration tests (including internal/escaping/cyclic symlinks and nonblocking
rejection of FIFO/symlink archive sources), a native writer
test that replaces a held root and attempts an existing-link write, and strict
all-target Clippy. These are store algorithm tests, not backend installation
acceptance. No new dependency or copied third-party implementation was added.

[Fork draft PR 2](https://github.com/oyzuai/mise/pull/2) now contains a single-use
embedding context with private roots, no config discovery, mandatory supplied
HTTP transport, denied direct-client access, immutable settings reloads,
frontend-owned shims and verbatim PATH composition. On Linux Rust 1.95 its five
fresh-process scenarios and library/example Clippy passed. The fifth scenario
uses mise's actual Node parser/resolver with fixture catalog metadata, resolving
`22` to `22.15.0`; it is not a real download/install test. Fourteen compliance
regressions passed and the existing 1,109 lock records remain unreviewed. The
native three-host workflow is running on candidate `c404d20b2`; native results
must be inspected before claiming qualification. The compliance CI correctly
requires @micahlmartin's current-head review; owner assignment does not satisfy
that review. Oyzu still has no production mise Cargo dependency.

The prior Oyzu [CI run 36964551092](https://github.com/micahlmartin/oyzu/actions/runs/36964551092)
passed all three CLI builds and macOS/Linux task jobs, but Windows failed during
native npm workspace acquisition with an invalid root dependency on
`@oyzu-example/shared`. That failure remains unresolved; local archive checks
do not establish that the whole draft PR is green. Full OEP-0003 implementation,
licensing approval and end-to-end qualification remain outstanding.

## Checkpoint 49: locked selection receipt/content matching

TM-04 now validates closed format-1 receipts against every installation in a
selected format-2 closure and freshly hashes all payloads through native held
directory handles. It compares the complete verification record, source artifact
size/digest, layout/backend identities, direct dependency installation keys and
caller-trusted installer release identity. No installed-version shortcut or
mtime cache is accepted. Typed entrypoints, interpreter paths, prefix arguments
and environment paths must resolve inside the selected self/direct-dependency
payloads; command/environment case collisions and literal PATH replacement fail.
The library returns a content-bound selection digest, not an execution grant.
The [receipt contract](reference/tool-lock-inspection.md#receiptcontent-verification-foundation)
documents limits, initial resolved record syntax and the missing admission,
schema, publication and lease layers. This is partial TM-01/04 and MISE-07 evidence.

The full Windows GNU Rust 1.94 suite passed before the final additional receipt
regressions; all six final receipt integration tests then passed on Windows and
Linux Docker Rust 1.94, with strict all-target Clippy and Windows formatting.
Linux also passed all 20 tool unit tests. Cases include every bound receipt
identity, duplicate/unknown fields, absent receipts, changed archive identity
with unchanged version, typed interpreter/argument containment, current payload
tampering and dependency-only tampering. Fixtures contain ordinary synthetic
files and execute nothing; they do not qualify a real backend installation.
All nine compiled CLI task scenarios and documentation structure checks passed.

The fork's candidate `c404d20b2` passed its native Linux library/conformance job
in [run 36966809734](https://github.com/oyzuai/mise/actions/runs/36966809734).
Windows and macOS had passed the library Clippy step and were running the
conformance step when inspected. Those pending results are not inferred passes.
The compliance review gate still awaits the owner's recorded decision.

## Checkpoint 50: atomic installation publication and cooperative leases

TM-04 now publishes verified candidate installations through native no-replace
directory moves, after validating the entire selected closure. Permanent,
lexically ordered per-installation OS mutation locks serialize publishers;
shared kernel leases are acquired before releasing those locks. Existing
committed content is rehashed and never overwritten, including corrupt entries.
Receipts must be single-link regular files and installation roots contain only
the payload and receipt. Publication is atomic per installation, not across an
entire closure. The [store contract](reference/tool-lock-inspection.md#publication-and-os-lease-foundation)
states platform durability limits and caller responsibilities.

Windows GNU and Linux Docker Rust 1.94 passed all 11 receipt/publication tests
(plus one ignored child fixture invoked by the process tests), including two
separate publishers, shared lease contention, forced owner termination, invalid
staging and rejection of linked receipts. The native no-replace unit test passed
on both hosts. The full Windows suite, strict all-target Clippy and all nine
compiled CLI task scenarios passed during this increment; the final additional
receipt cases were then rerun on both hosts with strict Clippy. Formatting and
documentation structure checks passed. macOS publication is not yet qualified.

This is a library foundation, not a working install or execution flow. Operation
journals, recovery, workspace/session references, pruning, backend layout
admission, receipt authoring and supervised child-tree lifetime integration are
still outstanding. A lease establishes cooperative liveness, not authorization
or protection from another process running as the same user.

Fork [run 36967923522](https://github.com/oyzuai/mise/actions/runs/36967923522)
passed Linux library/conformance checks. macOS failed because the cleared child
environment acquired `__CF_USER_TEXT_ENCODING`; the diagnostic prints variable
names only. A macOS-specific exception is being validated. Windows remained
pending when inspected. These results do not establish native qualification or
licensing approval; @micahlmartin owns both technical and licensing decisions.

## Checkpoint 51: verified blob cache and direct extraction handoff

TM-04 now streams exact locked bytes into a private snapshot, atomically publishes
content-addressed blobs without replacement, and fully rehashes cache hits. A
hit never consumes the acquisition reader; corruption fails without fallback.
Per-blob OS locks serialize publishers. The returned read-only snapshot does not
alias the cache, and the extractor consumes it without reopening the cache path.
The existing archive entrypoint shares the same bounded verifier. Extraction and
final tree observation now retain the same held native directory root.
See the [blob contract](reference/tool-lock-inspection.md#verified-blob-cache-and-extraction-handoff).

Windows GNU Rust 1.94 passed the full Rust suite, strict all-target Clippy,
formatting and all nine compiled CLI task scenarios. The final eight Windows
blob tests passed after adding process-termination coverage. Linux Docker Rust
1.94 passed nine blob tests (including redirected roots/symlinks), all six archive
tests and strict all-target Clippy. One ignored child fixture is explicitly run
by the process tests. Checks cover exact/truncated/oversized/wrong bytes, short
and interrupted reads, stream errors, cache tampering, external hardlinks,
cross-process contention, termination mid-copy, publication collision cleanup
and extraction after mutation of the original cache file. Documentation structure
checks passed. These are real store operations on synthetic content, not backend
installation or complete MISE-05/13 qualification.

The cache bounds bytes and memory, but transport timeouts/cancellation belong to
the broker. Interrupted processes can leave unselected staging; journals,
recovery/quarantine/prune and full power-loss testing are unfinished. Publisher
verification, admitted layout finalization and production worker wiring remain
separate gates. No dependency or third-party code was added.

The prior fork run 36967923522 completed with Windows and Linux passing and the
documented macOS failure. Candidate `e896fe0f9` contains the narrow macOS fix;
[run 36969203910](https://github.com/oyzuai/mise/actions/runs/36969203910)
has passed Linux while Windows/macOS remain running. The source-integration and
release gates remain open; the full OEP is not implemented.

## Checkpoint 52: data-only layout finalization and candidate receipts

TM-01/04/05 now have an initial finite archive-layout record, its draft format-1
JSON Schema and a data-only staging finalizer. The finalizer binds the complete
canonical plan to a caller-trusted admission digest and the selected lock's
layout/backend/platform/blob identities. It strips an optional archive prefix,
applies tighter extraction limits, checks required paths, resolves typed
self/direct-dependency references and authors a canonical candidate receipt.
The complete selected closure is still reverified before atomic publication.
Neither operation edits the lock or executes payload code. See the
[finalizer contract](reference/tool-lock-inspection.md#data-only-candidate-finalization).

The end-to-end store test now streams a synthetic archive into the blob cache,
finalizes it into a receipt-bearing candidate, publishes under OS locks/leases,
and verifies the committed selection against unchanged lock bytes. Windows GNU
Rust 1.94 passed the full Rust suite, strict all-target Clippy, formatting and all
nine compiled CLI task scenarios. The final seven layout tests passed on Windows
and eight on Linux Docker Rust 1.94, with strict Clippy on both. Linux additionally
checks contained and escaping symlinks after prefix stripping. Earlier archive
and blob regressions also passed after the extractor refactor.

The schema checker passed with pinned `jsonschema==4.25.1`: one valid layout and
17 invalid shape mutations shared with the Rust finalizer tests. A dedicated CI
job now runs this shape check; native Rust checks exercise runtime semantics.
Schema validity alone proves no content integrity, admission or execution result.
Documentation checks passed. No production dependency or upstream code was added.

Supported finalizer transforms remain tar/tar.gz with optional strip-prefix,
root payload and no executable overrides. ZIP/xz/raw, subtree projection,
executable overrides, compiled descriptors, backend-generated plans, publisher
verification, worker/broker wiring and backend parity remain unfinished. These
synthetic archives are never represented as real installed Node versions and
are never executed. The full OEP acceptance matrix remains open.

Fork candidate `e896fe0f9d75f1a4f544b6ee92ffbdbc4f33f496` passed library/example
Clippy and all five fresh-process boundary checks on native Windows, macOS and
Linux in [run 36969203910](https://github.com/oyzuai/mise/actions/runs/36969203910).
This resolves the macOS runtime-variable failure. It qualifies only that current
library boundary, including the fixture-backed real Node catalog parser; it does
not qualify native installations, distribution licensing or a release.

## Checkpoint 53: upstream-owned Node archive facts and branch integration

Fork candidate `3552b5d03111f6d3143233dbebb94a8368110a9e` adds
`Session::node_archive_facts` for exact stable versions and the initial three
target tuples. The Node backend owns the fact type and shares artifact/mirror
selection with upstream lock metadata, plus native Node/npm path helpers with
its existing launch behavior. The embedding session enforces its own immutable
tool admission. Returned locations and paths perform no acquisition or target
execution and do not prove availability, size, checksums, signatures or admission.
The [reference](reference/mise-maintenance.md#candidate-node-archive-facts)
describes the boundary and corporate-route responsibilities.

Linux Rust 1.95 strict library/example Clippy and all six fresh-process scenarios
passed for the final source. The expanded backend scenario checks two versions,
all three target facts, parity with upstream artifact URL selection, invalid
versions/targets and zero additional transport calls. A separate process denies
Node planning when it is not admitted. Fourteen compliance regressions passed;
37 inputs and 1,109 unreviewed lock records remain inventory-consistent. No
dependency, license or notice file changed. Native requalification is running in
[run 36971369051](https://github.com/oyzuai/mise/actions/runs/36971369051); the prior
five-scenario native success does not prove this new increment.

The Oyzu branch merged builder work through `7746c1e`, resolving only a status
history conflict while retaining both histories. This dedicated tool journal is
linked from the overall status page so independently progressing builder/tool
histories no longer compete for the same appended checkpoint block. The merged
Windows GNU Rust 1.94 full suite, strict all-target Clippy, formatting and all
nine compiled CLI task scenarios passed. Documentation (149 Markdown files),
all 58 example structures and the pinned layout-schema checker passed; these
structural checks do not qualify runtime behavior. The full PR CI matrix must
still be checked after pushing the resolved branch.

TM-02/05/06 remain partial. Actual metadata and publisher verification, conversion
of the facts into a reviewed admitted layout, native archive parity, production
source import and worker wiring are not implemented. No separate mise executable
was built or invoked. The full OEP goal and licensing/release gates remain open.

## Checkpoint 54: native boundary qualification and dependency evidence

Fork revision `3552b5d03111f6d3143233dbebb94a8368110a9e` passed strict
library/example Clippy and all six embedding scenarios on native Windows, macOS
and Linux in [run 36971369051](https://github.com/oyzuai/mise/actions/runs/36971369051).
This supersedes checkpoint 53's pending native result, without qualifying actual
tool acquisition, installation or a production dependency.

The fork now includes a bounded offline Cargo graph/notice evidence collector.
The first Linux target collection observed 955 packages, including normal/build
closures and preserving all declared license expressions. Its source worktree
was reported as modified; this is exploratory review input, not a release SBOM.
All rows remain unreviewed. Windows ran 19 compliance tests with one unavailable
symlink case skipped; Linux passed all five new graph tests, including that case.
The existing inventory remains consistent at 37 inputs and 1,109 unreviewed lock
records. No dependency or upstream notice changed.

Oyzu CI [run 36971417416](https://github.com/micahlmartin/oyzu/actions/runs/36971417416)
passed its Windows build job but failed macOS archive-test setup and Linux Cargo
doctest setup. The archive fixture now resolves its temporary parent before
opening the deliberately no-follow store boundary. The doctest harness preserves
the Cargo proxy filename rather than resolving it to `rustup`. Neither fix relaxes
production checks. Linux passed the anchored-writer regression and real Cargo
doctest conformance after these corrections; native macOS requalification remains
required. The full OEP implementation and licensing review remain unfinished.

The corrected Oyzu source passed the full Windows GNU Rust 1.94 test suite,
strict all-target Clippy, formatting and all nine real CLI task scenarios.
Documentation structure passed for 149 Markdown files; it is not runtime proof.

## Checkpoint 55: receipt schema and shared validation fixtures

TM-01 now defines the draft format-1 receipt shape in
`docs/contracts/tools-v1/receipt.schema.json`. The existing receipt verifier
remains responsible for locked identity, fresh payload hashes, dependency
ownership, portable paths, case collisions, byte limits, file types and executable
permissions. The schema grants no admission or execution authority.

The schema checker accepts the synthetic native receipt and typed interpreter/
environment variants, and rejects 23 shared malformed-record cases. Rust uses
that same receipt shape with real fixture payload hashes and rejects those same
mutations. The initial 12 receipt integration tests passed on Windows GNU and
Linux, including concurrent publication and termination cases; the positive
shared-shape variants add a further runtime test. This is contract/store evidence,
not a successful native backend installation. Worker, backend descriptor and
selection-grant schemas and the full TM-01 exit gate remain incomplete.

The branch merged main at `1b71473`, retaining the newer Rust packaging work and
the shared Cargo-proxy correction. The only manual merge conflict was the comment
above that same correction. Current reference documentation explains the schema
command and the distinction between shape and content verification. The schema
checker, all 58 example structures and documentation structure passed.

The final 13 receipt integration tests passed on Windows GNU and Linux (the
separately invoked child fixture is marked ignored in the ordinary test list).
The merged Windows GNU Rust 1.94 full suite, strict all-target Clippy, formatting
and all nine real CLI task scenarios passed. Native macOS and full refreshed CI
still require confirmation against the pushed head.

## Checkpoint 56: durable process-lease observations

TM-04 now publishes a flushed, no-replace `leases/<id>.json` record before
returning a verified selection lease. It records the selection digest, sorted
installation keys, owner PID, timestamp and diagnostic ID. Normal destruction
removes only that record before releasing kernel leases. Forced termination
leaves stale evidence while the OS releases its locks. Neither PID nor journal
presence is a liveness or authorization check. Failed journal publication denies
selection without overwriting committed content. No new dependency is introduced.

Windows GNU passed 14 receipt/publication tests; Linux passed 15, including a
redirected journal-directory rejection. Tests inspect exact record contents,
overlapping independent lease IDs, ordinary cleanup, failed-record lock release
and a real killed child with a retained record. Linux strict all-target Clippy
also passed. Recovery/reaping, staging owner/start-time records, workspace and
shell-session references, prune and supervised child-tree integration remain
unfinished; this is not the complete MISE-05 fault-injection gate.

Native macOS job `110730105897` in run `36972783742` exposed more fixture parents
under the `/var` alias in archive integration tests. The four store integration
suites now share a physical temporary-root fixture, resolving only the trusted
parent before creating test content. Production no-follow checks and explicit
hostile links are unchanged. Final Linux archive/blob/layout/receipt suites pass
(6/9/8/15 tests); final Windows counterparts pass (4/8/7/14). Native macOS must
requalify this correction against the next pushed head.

The full Windows GNU Rust 1.94 suite, strict all-target Clippy, formatting and
all nine compiled CLI task scenarios passed. Documentation checks passed;
these results do not establish backend installation or release approval.

## Checkpoint 57: conservative stale process-lease recovery

TM-04 adds explicit `tools::recover_tool_leases(store, dry_run)`. New leases hold
a separate permanent journal guard lock for their whole lifetime, including
records with no installation keys. Recovery requires existing journal/mutation/
installation lock evidence, uses nonblocking exclusive acquisitions, validates
closed bounded records and removes only verified stale final journal files.
Unknown, malformed, pending, linked and legacy unguarded records remain; no
installation, lock file or directory is deleted. There is no automatic sweep or
CLI prune command. PID observations never decide liveness. The reference records
limits, aggregate outcomes and partial I/O/durability failure semantics.

Tests exercise a genuinely killed owner, dry-run, removal after all locks release,
busy overlapping leases, malformed and legacy records, hardlinks and an empty
record's independent guard. Full store recovery still needs staging operation
ownership/start-time evidence, quarantine and workspace/session reference handling.
This does not complete TM-04 or MISE-05.

Native macOS job `110731250217` in run `36973162287` passed all six archive tests
after the physical-parent fixture correction. Its concurrent blob publisher test
then failed with an unqualified ENOENT. The cache now adds operation context to
errors, and the test reaps both publishers before reporting a failure so fixture
cleanup cannot obscure the other child's result. The macOS cause remains
unresolved until the instrumented native run provides evidence; no pass is inferred.

Windows GNU Rust 1.94 passed the full suite, strict all-target Clippy, formatting
and nine real CLI task scenarios. Linux passed the final 16 receipt/recovery and
nine blob integration tests, the empty-record guard unit test and strict
all-target Clippy. Documentation structure passed. Native macOS recovery and
the outstanding concurrency failure still require qualification.

## Checkpoint 58: explicit Unix lock creation and reuse

Run `36973690703` passed native Windows and Linux build jobs but macOS job
`110732850664` isolated the blob concurrency failure to `open blob mutation lock`
with ENOENT. Unix lock initialization now uses exclusive no-follow creation,
then existing-only no-follow open on an already-existing entry. It never replaces,
unlinks or truncates the winning inode, and recovery remains existing-only.
The new native-access regression checks inode reuse, content preservation,
missing existing-only records and rejected symlinks. This is a candidate fix;
the macOS run must confirm the observed failure is resolved.

Linux passed that regression, nine blob and 16 receipt/recovery tests, and strict
all-target Clippy. Windows GNU Rust 1.94 passed its full suite, strict all-target
Clippy, formatting and nine real CLI task scenarios. The earlier CI's downstream
task/build jobs were skipped because of macOS failure, not counted as passes.

## Checkpoint 59: Go archive facts and normal lease release

Fork `8240083a4a5de152ce275609eff420cc9c7f0f8e` adds backend-owned Go
archive facts for exact stable versions on the three initial targets. Shared
upstream URL/layout helpers avoid an independent mirror/platform mapping. Linux
Rust 1.95 strict library/example Clippy and all seven fresh-process conformance
scenarios passed, including two Go versions, three targets, rejected inputs,
backend URL parity and no transport calls. Native Go conformance is pending.
No dependencies or notices changed. These facts do not prove catalog membership,
archive availability, publisher verification, acquisition or installation parity.
Oyzu still has no production mise dependency or distribution approval.

Root run `36974290636`, macOS job `110734650915`, passes all six archive,
nine blob and eight layout checks, including concurrent blob publication after
checkpoint 58. Receipt testing then fails when an exclusive lock remains busy
after both same-process shared leases are dropped. Normal destruction now
explicitly unlocks owned leases after journal cleanup, so duplicated/inherited
descriptors cannot prolong a completed supervisor lifetime. A Unix regression
retains a duplicate descriptor across owner destruction and requires immediate
exclusive acquisition. This addresses that mechanism; the native macOS rerun
must confirm the observed failure is resolved. Forced-owner termination remains
a separate kernel-release test.

Final Windows GNU Rust 1.94 full tests, strict all-target Clippy, formatting and
all nine compiled CLI task scenarios passed. Linux passed the duplicated-handle
regression, all 16 receipt/recovery cases and strict all-target Clippy. Documentation
structure passed. Fork run `36974808859` passes notice inventory and guard tests
but fails its human-review gate; no approval is inferred from owner assignment.

## Checkpoint 60: declared Unix executable layout paths

TM-04 now applies the existing layout contract's `executable_paths` on Unix.
It validates a bounded sorted unique portable path list before extraction, opens
each ordinary file through no-follow directory handles, requires one hardlink,
sets private mode 0700 and flushes the file. The receipt binds a freshly observed
tree after these changes. Windows rejects nonempty lists before staging writes;
no Unix permission operation is silently treated as a Windows launcher rule.
The regression materializes a nonexecutable archive member, marks it executable,
and publishes it through full receipt verification; invalid, duplicate, unsorted,
missing and directory paths cannot produce receipts. No fixture payload runs.
This is layout behavior, not backend installation qualification.

Windows GNU Rust 1.94 full tests, strict all-target Clippy, formatting and nine
real CLI task scenarios passed. Linux passed all nine layout tests and strict
all-target Clippy. Documentation structure passed. Native macOS confirmation
is pending; this does not complete TM-04 or any backend acceptance row.

## Checkpoint 61: native Go conformance and retained review evidence

Fork `8240083a4` passes library/example Clippy and all seven fresh-process
conformance scenarios on Windows, macOS and Linux in run `36974808824`. This
qualifies the data-only Go facts boundary, not artifact verification/installation.
Root `b63f9f7` passes all three native CLI build jobs in run `36975072550`,
including the macOS lease-release regression; downstream scenarios are still
running. Neither result covers the newer executable-layout revision.

Fork `f7d5bb906` adds per-target offline graph/notice collection after explicit
locked metadata provisioning, with 30-day CI artifacts and collector-change
triggers. The workflow now checks formatting of the Go backend file as well.
All 19 compliance regressions ran locally (one Windows symlink skip), the notice
inventory remains consistent, and workflow YAML parses with three explicit
targets. The first report-producing CI run remains pending. Local Linux evidence
contains 955 unreviewed packages; the cross-target collection stopped at missing
macOS cache metadata. It does not establish three-target evidence or approval.

## Checkpoint 62: incomplete notice scans fail collection

Fork `bb3ba5b25` makes directory enumeration errors fatal to candidate notice
collection. Python's default walk behavior could otherwise silently omit an
unreadable subtree. The regression injects an enumeration permission failure
and requires rejection instead of a partial inventory. All 20 compliance tests
ran on Windows (one symlink skip); the dependency/notice inventory is unchanged
and consistent. Native evidence collection remains pending, and no license
disposition or release approval has changed.

## Checkpoint 63: three-target candidate evidence and cache provisioning

Local offline collection at fork `bb3ba5b25` reports Linux 955, Darwin ARM64
952 and Windows MSVC 975 candidate packages. The Linux-mounted Windows checkout
is reported modified; these reports are exploratory, not clean-release evidence.
A separately downloaded Linux CI artifact from run `36975506849` binds clean
merge revision `b7914105eb84460a64bee20ca13784fed603e10d`, contains 955
packages, and has SHA-256
`3277f4125d6d0e05b8929cdc4132088bba2b2301d68af9f8c675f69dbbbdcf1f`.
It reports 98 packages without collector-recognized notice files; source-level
review remains necessary. No license expression has been approved or selected.

Target-specific `cargo fetch` missed GNU support packages needed by Windows
metadata collection. Fork `76192a7eb` provisions the exact metadata query and
feature set before offline collection instead. Local Windows collection then
succeeded. Compliance tests pass (19 passed, one Windows symlink skip), notices
remain consistent, and the updated CI matrix is pending.

## Checkpoint 64: clean native candidate reports retained for review

Run `36975506849` successfully collected and uploaded reports on all three
native runners. Downloaded reports bind clean merge revision `b7914105eb84460a64bee20ca13784fed603e10d`.
The [evidence index](proposals/OEP-0003-mise-integration/candidate-license-evidence.json)
records artifact names, report SHA-256 hashes, declaration counts and identities
without observed notices. Counts are 952/975/955 packages for Darwin ARM64,
Windows MSVC and Linux GNU, with 98/96/98 lacking collector-recognized notices.
These clean reports supersede the local modified-checkout reports for review
evidence, not for approval. Collection/upload steps passed while job cache cleanup
was still running; no overall workflow completion is inferred. The report source
predates the subsequent unreadable-directory and exact-provisioning fixes.

Executable-layout revision `1aae272` passes all three native CLI build jobs in
run `36975364359`; Linux CLI task and isolated BuildKit checks also pass. Other
scenario jobs were still running when recorded. Full OEP acceptance remains open.

## Checkpoint 65: backend descriptor shape contract

TM-01 adds the closed backend descriptor JSON Schema using the identity fields
already specified by the OEP. Required nullable identities, exact source pins,
digest shapes, positive safe revision integers and unique string features are
checked. The existing offline contract runner and CI include four valid and
thirteen invalid descriptor fixtures. All descriptor/layout/receipt shape checks
pass, as do documentation structure checks. Runtime descriptor parsing, sorted
feature semantics, canonical identity vectors and compiled backend admission
remain outstanding; this is not production source integration or TM-01 completion.
