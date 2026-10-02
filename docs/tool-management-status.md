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

## Checkpoint 66: runtime backend descriptor identity inspection

TM-01 adds `oyzu tools inspect-backend PATH`, backed by a private descriptor
module and the existing strict JSON/canonical record encoders. It reads at most
2 MiB plus an overflow probe, requires explicit nullable identities and exact
source/digest shapes, bounds revision integers and requires sorted unique features.
Output reports identity only; it cannot authorize source imports or installations.
The runtime consumes all shared descriptor fixtures, rejects duplicate keys,
unsorted features and oversize input, and binds every field against a canonical
golden digest. A real CLI test verifies read-only behavior, failure exit status
and independence from malformed Oyzu/mise configuration. Compiled admission,
compatibility and reviewed source integration remain outstanding.

Windows GNU Rust 1.94 full tests, strict all-target Clippy, formatting and all
nine real CLI task scenarios passed. Linux descriptor unit tests, all three tool
inspection CLI regressions and strict all-target Clippy passed. Shared schema
fixtures and documentation checks passed; native macOS confirmation is pending.

## Checkpoint 67: corrected native evidence pipeline passes

Fork head `76192a7eb07cb87491db4f816b5da05df210f110` passes the entire
three-host embedding workflow `36976065769`, including exact metadata provisioning,
offline collection and uploads. Downloaded reports bind clean tested merge
revision `9e7556ae1b057830296402549c76bfbd38981acc`. The candidate evidence
index now references those artifacts and hashes, superseding the older index
that predated unreadable-directory and provisioning fixes. Package/notice-gap
counts are unchanged. This establishes candidate evidence collection, not human
license approval or a complete shipping graph.

Root run `36975364359` passes all three native CLI task scenario jobs in addition
to its builds and isolated BuildKit check. Captured-source Linux builds remain
in progress and are not counted as passed.

## Checkpoint 68: nonblocking rejection of Unix special record inputs

The shared tool-record reader now owns bounded regular-file reads for descriptor
inspection, lock inspection, candidate staging, selection verification and lease
publication. Unix opens with O_NONBLOCK before checking the opened file type, so
a FIFO cannot hang validation before the regular-file check. A real CLI regression
creates a FIFO without a writer and requires both inspectors to reject it within
a bounded deadline, killing/reaping the child if the regression returns. Ordinary
regular-file symlinks remain supported; store containment rules are unchanged.

Windows GNU Rust 1.94 full tests, strict all-target Clippy, formatting and nine
real CLI task scenarios passed. Linux FIFO/descriptor and lock CLI regressions,
nine layout tests, 16 receipt/recovery tests and strict all-target Clippy passed.
Documentation structure passed; native macOS verification remains pending.

## Checkpoint 69: existing Java metadata API conformance

The maintained fork adds an eighth isolated scenario using the existing Java
backend's target-aware lock metadata API. Supplied synthetic Temurin metadata
exercises all three initial targets, exact URL/checksum preservation, unsupported
installer filtering, missing-version rejection, Java-only settings admission and
one supplied transport request per target. No Java production code, dependency
or notice changes are required. Linux Rust 1.95 strict library/example Clippy and
all eight scenarios pass; 20 compliance tests ran with one Windows symlink skip,
and the notice inventory remains consistent. Native Windows/macOS verification
is pending. This does not establish archive layout, cryptographic verification,
installation or Java execution parity.

## Checkpoint 70: ZIP reuse boundary inspection

The maintained fork uses `zip` 8.6.0. Its cached original manifest declares MIT
and Rust 1.88, and its original MIT copyright/permission notice is present. No
source or dependency was imported into Oyzu. Source inspection found that
`read/zip_archive.rs` builds an IndexMap keyed by raw filename, collapsing duplicate
central-directory entries, and allocates the metadata vector before caller entry
limits can run. `read/stream.rs` stops central-directory visitation on a parsing
error. Neither high-level entry count nor successful streaming visitation alone
proves the OEP's duplicate, metadata-bound and malformed-directory obligations.

This changes the planned ZIP integration: add bounded metadata validation before
using the decoder, with duplicate/local-central consistency fixtures and real
Windows archive parity. ZIP remains unsupported; no library or licensing approval
is inferred from this inspection.

## Checkpoint 71: one archive path admission implementation

TAR extraction now delegates portable names, duplicate sets, strip-prefix
validation, case/type collisions, depth/expanded-entry bounds and anchored
parent creation to `store/archive/paths`. Decoder-specific parsing, content
limits and delayed links stay in the extractor. This is the path layer for the
planned ZIP decoder, not ZIP support. Raw names stored for duplicate detection
now consume the existing 32 MiB name budget, including skipped strip ancestors;
previous accounting covered only expanded paths and link targets. A regression
requires budget exhaustion to reject an otherwise-skipped ancestor before writes.

Windows GNU Rust 1.94 full tests, strict all-target Clippy, formatting and nine
real CLI task scenarios passed. Linux passed the name-budget regression, six
archive tests, nine layout tests and strict all-target Clippy. Native macOS
confirmation of the path refactor remains pending. Separately, fork `136d65573`
passes all eight scenarios and library/example Clippy on all three native hosts
in run `36977793983`, qualifying the Java metadata boundary only.

## Checkpoint 72: bounded ZIP candidate staging

The archive-layout finalizer now handles ZIP32 stored and DEFLATE files through
the fork's `zip` 8.6.0 decoder version, with default features disabled. The lock
adds only zip and typed-path; both packages' original license texts are preserved
under `third-party/` and match independently downloaded Cargo sources. This does
not approve the shipping graph or enable the production mise provider.

Before decoder indexing, Oyzu validates bounded metadata and entry counts,
duplicates, local/central agreement, physical record coverage, size/ratio budgets
and data descriptors. Extraction reuses anchored path admission and requires
decoded size/CRC checks before receipt creation. ZIP64, encryption, split
archives, stubs, links, special entries and alternate name encodings remain
explicitly unsupported. See the [reference](reference/tool-lock-inspection.md)
for the exact subset and recovery behavior. This advances TM-04; real upstream
archive parity and the full OEP acceptance matrix remain incomplete.

Stored and DEFLATE integration fixtures exercise candidate staging, publication
and subsequent locked-content verification. Negative fixtures cover corrupt
payloads, inconsistent headers, oversized metadata, duplicate names, traversal,
case collisions and unsupported extensions. Both ZIP32 descriptor encodings are
extracted and corrupted variants rejected. Windows GNU Rust 1.94 full tests,
strict all-target Clippy, formatting and nine real CLI task scenarios passed.
Linux archive/layout tests and strict all-target Clippy passed. Documentation
structure and shared contract fixtures passed. Native macOS/MSVC qualification
of this change remains pending.

Separately, previously running captured-source CI job `110739178439` in run
`36975364359` completed with failure during Helm lint: the parent chart reported
a missing child dependency. Earlier passing native build/task checks do not turn
that captured-source run into passing end-to-end evidence. The failure remains
to be resolved and reverified.

## Checkpoint 73: real Node ZIP store parity

An opt-in production-store integration fixture now uses the original Node
22.14.0 Windows x64 archive, pinned at 34,906,389 bytes and SHA-256
`55b639295920b219bb2acbcfa00f90393a2789095b7323f79475c9f34795f217`.
The digest was checked against the official release's HTTPS checksum list;
publisher signatures are not claimed. An offline Python preparation script
uses its independent ZIP implementation to inventory all 3016 stripped entries.
The Rust store must reproduce every entry kind, file size and digest before
publishing and verifying the selection. The original LICENSE is retained in
the payload and participates in content identity.

The Windows GNU run passed real Node execution (`v22.14.0`) from the committed
payload while holding its lease. Changing only the archive digest denied both
cached verification and lease acquisition; restoring the lock recovered the
selection. Linux passed the same foreign-target materialization/publication and
changed-lock checks in Docker with `--network none`, without attempting Windows
execution; strict all-target Clippy passed. The fixture is now explicitly wired
into all three native CI builds; those results remain pending.

Windows full locked tests, strict all-target Clippy, formatting and nine real
CLI task scenarios passed. Documentation structure and workflow YAML parsing
passed. A malformed fixture was rejected without creating a manifest.

This supplies real archive and store evidence for TM-04/MISE-07/13, not full
acceptance of those IDs. Backend and installer admission identities are synthetic
test records. Native metadata/layout parity against the maintained fork,
publisher verification, production resolution/authorization, supervised launch
and the remaining platforms/backends still require implementation and evidence.
See [the reference](reference/tool-lock-inspection.md) for exact provisioning and
execution commands. Normal tests report this externally provisioned case as
ignored, never passed without its inputs.

## Checkpoint 74: resolve the captured-build fixture failure

The prior Helm failure was reproduced with native Helm 3.22.0: the subchart-only
fixture omitted its child dependency declaration. The fixture now declares the
contained child using an empty repository. The compiled CLI with the provisioned
Linux Docker worker passed chart/rendered artifact production and both reports;
a changed child assertion failed and prevented artifacts. Source preservation
and bundle schemas/digests were checked in both cases. See builder
[integration verification](implementation-status.md#tool-integration-verification-declare-the-fixtures-native-helm-child-dependency).
The complete CI run remains unverified; this correction does not complete any
remaining tool-management package.

## Checkpoint 75: nearest frozen scope selection

`tools/selection` now selects an existing locked environment from an explicitly
resolved workspace and physical cwd, exact profile, effective request digest
and platform. It reuses bounded whole-lock validation and existing selection
identities. The nearest component-boundary ancestor wins; stale nested requests
and missing nested platform coverage cannot fall back to a convenient root.
Outside/non-directory cwd inputs fail. It reads no project or mise configuration
and performs no network, installation, execution or lock mutation.

Windows scope/profile/stale/platform regressions pass, including a root with
Windows coverage and a nearer Linux-only tool closure. Linux additionally tests
internal directory aliases and rejection of a symlink escaping the workspace.
The [reference](reference/tool-lock-inspection.md#frozen-environment-selection)
documents caller obligations and errors. This is a TM-03 foundation; effective
request projection, backend resolution, authorization, lock editing and worker
wiring remain incomplete.

The final Windows GNU full locked suite, strict all-target Clippy and formatting
passed; nine real CLI task scenarios passed during this change. Linux passed all
four selection tests and strict all-target Clippy, including locked-scope alias
ambiguity. Documentation structure passed. Native macOS/MSVC confirmation and
the remaining end-to-end tool-management flows are still pending.

## Checkpoint 76: effective request projection and frozen binding

`tools/requests` now projects captured effective configuration through an
explicit caller-trusted alias catalog. It preserves version expressions for
the backend, rejects unknown/duplicate/rebound aliases, normalizes bounded native
constraint and capability sets, and computes `oyzu.tool-requests.v2` over the
documented closed identity object. Environment values, credentials and policy
revisions do not enter that digest. The existing canonical tool-ID validator is
shared with lock parsing rather than copied.

`select_for_tool_requests` checks both the projected digest and canonical request
map against one captured lock, preserving nearest-scope and no-fallback rules.
Tests cover effective profile overrides, alias/direct-ID equivalence, preserved
range syntax, environment exclusion, reordered sets, changed constraints and
capabilities, duplicate/oversized inputs, a separately computed golden digest,
and stale configuration or tampered request-map rejection without lock edits.

This advances TM-03 but does not load a production alias catalog, collect native
builder constraints, resolve versions, edit locks, authorize execution or enable
the worker. The [reference](reference/tool-lock-inspection.md#effective-tool-request-identity)
documents the experimental Rust boundary and its caller obligations; the OEP
remains a draft with incomplete acceptance gates.

Windows GNU full locked tests, strict all-target Clippy, formatting and nine real
CLI task scenarios passed. Linux passed request/selection integration tests and
strict all-target Clippy. Documentation structure passed. Native macOS/MSVC
confirmation and the complete production tool-management flows remain pending.

## Checkpoint 77: main integration and native ZIP checks

The tool branch incorporates main through
`1dcb2a80b8fe7307659e281d76908d314f62df6b`, preserving its builder updates and
eight-suite captured-build matrix. Merge conflicts were limited to the status
journal and the independently corrected Helm dependency fixture. Both branches'
verification records remain; the fixture retains an explicit empty repository
for its contained child. The real Node ZIP CI step remains in the merged workflow.
Documentation, example structure, acceptance inventory and workflow parsing
checks passed; those checks do not establish native builder acceptance.

Earlier ZIP implementation commit `8d6c7ff` now passes native CLI build/test jobs
and CLI task scenarios on Windows, macOS and Linux in run `36979879906`, as well
as the isolated BuildKit worker check. Its captured-source job remains live and
is not counted as passed. That revision predates the real Node ZIP fixture and
request projection, so these results do not qualify those later additions.

The merged tree passed Windows GNU full locked tests, strict all-target Clippy,
formatting and nine real CLI task scenarios. Linux request/selection tests and
strict all-target Clippy passed again after integration. Complete native CI and
the eight captured suites at this merged revision remain pending.

## Checkpoint 78: admitted names from the fork registry

The candidate fork exposes `Session::tool_aliases`, using its revision-baked
registry to map short names, registered aliases and canonical core IDs onto only
the session's admitted backends. Canonical rebinding, ambiguous names and removal
of an admitted core backend fail closed. Embedded settings disable floating
registry updates and preserve that setting after reload. No dependency, lockfile,
registry data or upstream notice changed.

The ninth fresh-process conformance scenario covers Node, Go, Java and Python,
exclusion of unadmitted tools and reload stability; empty admission is checked
in the existing denied-tool scenario. This advances the TM-02/03 candidate
boundary, not production catalog loading. Oyzu still needs reviewed source
admission and worker wiring before using this map for actual resolution.

Fork revision `f4d245e888c31414037ca8fe15d1ea634eb6320a` passed Linux Rust 1.95
strict library/example Clippy and all nine isolated conformance scenarios.
Formatting passed. Compliance regression checks passed (20 tests, one Windows
symlink skip), and inventory consistency passed with all 1,109 lock records still
unreviewed. Native macOS and Windows confirmation for this revision is pending.
No separate mise executable was built or invoked.

## Checkpoint 79: explicit format-2 lock publication

`tools::ToolLockEdit` captures a bounded existing source or a missing destination
and prepares a complete candidate through the existing whole-graph validator.
It preserves unchanged TOML records/comments, retains exact bytes for a semantic
no-op, returns added/changed/removed record identities and checks that lossless
editing did not change candidate semantics. Publication uses a permanent sibling
writer lock, exact captured-byte comparisons, flushed same-directory staging,
atomic replacement or no-replace creation, and Unix directory sync. Windows
replacement denials have a two-second retry cap; read-only sources fail before
staging. No resolver, migration, command, networking or implicit installation is
enabled by this API.

Tests exercise real files, concurrent external edits and writer locks, invalid
graphs, empty-graph transitions, inline arrays and comments. Windows tests cover
sharing denial and recovery plus read-only cleanup; Unix covers redirected files
and nonblocking FIFO rejection. This is cooperative optimistic publication, not
an atomic filesystem compare-and-swap against a malicious noncooperating writer.
The [reference](reference/tool-lock-inspection.md#explicit-lock-edit-transaction)
documents caller obligations, failure recovery and remaining TM-03 orchestration.

Windows GNU passed all seven edit tests, the full locked Rust suite, strict
all-target Clippy, formatting and nine compiled CLI task scenarios. Linux passed
all six applicable edit tests and strict all-target Clippy. Documentation
structure passed. Native macOS/MSVC CI for this new editor is pending.

Separately, run `36982735259` at the earlier merged revision `2e5ecb4` now confirms
the explicit real Node ZIP store parity/changed-lock test on Windows, macOS and
Linux, together with each native CLI build/test job. This qualifies that fixture
at that revision, not this later lock editor or the full production backend.

## Checkpoint 80: preserved candidate notice evidence

The maintained fork's offline Cargo collector now optionally creates a
deterministic archive of original observed notice bytes, embeds its exact report
and supplies a raw-hash/size index with explicit missing-notice package identities.
It rejects drift, redirects, duplicate/unsafe paths and exceeded bounds; ordinary
failure removes only its own partial output. Native embedding CI now retains the
report and archive together. No dependency, original notice or approval policy
changed, and all dispositions remain unreviewed.

The Linux offline development collection covered 955 packages and 1,512 notices
(7,822,597 raw bytes). Independent archive verification matched all files, report
bytes, raw/LF hashes and package/path pairs. The report correctly identifies a
dirty worktree and 98 packages without observed notices; it does not establish a
clean shipping graph, source-delivery compliance or legal approval. Windows
compliance checks pass with two symlink tests skipped; Linux runs all 24 tests.
Native CI evidence for this collector increment remains pending. Operational
instructions and limitations are in the [maintenance reference](reference/mise-maintenance.md#original-notice-archives).

This collector is committed at fork `94f0f75aadda0f20145de9dd0ea1df1b6c2403dc`.
The local development archive's SHA-256 is
`b318a04a06ec013568cf4623d64bf2adb036c08454162d7c2123ebd42b4e2789`;
its embedded report identifies the precommit working state, not an approved head.
Separately, fork run `36983527342` now passes on all three hosts for `f4d245e88`,
confirming the nine-scenario alias-projection increment from checkpoint 78. That
run predates notice bundling and does not qualify the new collector artifacts.

## Checkpoint 81: constrained Node metadata selection

Fork `6f8e6863794ac070bb0b80921245cf73a9981a20` exposes
`Session::resolve_node_version` for session-admitted Node. The Node-owned adapter
composes upstream catalog loading, aliases, ordering, prefix matching and npm
range filtering. It intersects every supplied native constraint before choosing
a canonical stable catalog version; even exact pins require metadata membership.
Path/system/ref/subtraction selectors, malformed ranges and oversized inputs fail
before metadata access. Missing transport without cached metadata fails closed.
No installation, target execution or ambient configuration discovery occurs.

Strict Linux library/example Clippy, formatting and all eleven fresh-process
scenarios passed. The new selection fixture covers exact/prefix/range/channel
requests, constraint ordering/intersection/conflicts, uncataloged pins, denied
admission, pre-acquisition input rejection and reuse of one supplied catalog.
Compliance checks remain inventory-consistent and unapproved; 24 regression tests
pass on Windows with two symlink skips. Native Windows/macOS conformance for this
revision is pending. This advances the candidate TM-03 seam but does not complete
production source admission, worker wiring, other backend resolvers, target
availability, publisher verification or lock/update orchestration. The
[reference](reference/mise-maintenance.md#candidate-node-version-selection)
documents inputs, bounds, effects and caller responsibilities.

Separately, root run `36984574863` passes native CLI build/test jobs on Windows,
macOS and Linux at `1ac6667`, covering the preceding lock-editor increment. Its
remaining task/captured-build jobs were still active or queued when checked and
are not counted as complete here.

## Checkpoint 82: target-bound Node checksum metadata

Fork `1f516e78ec8c15234d955b0ad24eee70117f5e3b` exposes Node target facts with a
required publisher-declared SHA-256 entry for the exact archive filename. It
shares the backend's checksum acquisition path and uses a checked variant in the
existing hash owner. Field splitting remains shared with the legacy parser;
duplicate names, malformed hashes, extra fields and exceeded limits fail the new
path. Legacy tolerance remains unchanged. Missing targets never trigger source
compilation or another target fallback.

All twelve fresh-process scenarios, strict library/example Clippy, shared
`mise-util` library Clippy and formatting passed on Linux. The new scenario covers
all three initial targets, canonical checksums, cache reuse and malformed/missing
metadata; existing denial cases cover absent transport and tool admission.
Windows compliance checks pass with two symlink skips and no inventory change.
Native Windows/macOS confirmation remains pending. This is declared metadata,
not publisher signature verification, acquired bytes/size or install authority;
the [reference](reference/mise-maintenance.md#node-target-checksum-metadata)
documents the distinction and bounds.

## Checkpoint 83: clean native notice archives retained

Run `36985298309` passes on all three hosts for the notice collector at fork
`94f0f75aa`. All three downloaded archives were independently checked against
their embedded/external reports and every indexed notice's hash, size and package
path. The [evidence index](proposals/OEP-0003-mise-integration/candidate-notice-evidence.json)
records exact artifact/report hashes at clean merge revision
`0d0cfce94bfd775ef95ee70090b22f367ee37458`. Darwin has 952 packages/1,501 notice
candidates, Windows 975/1,545 and Linux 955/1,510. Missing-notice package counts
remain 98, 96 and 98 respectively; no legal disposition changed.

Correction to checkpoint 80: the earlier dirty local archive's 1,512 entries
included two generated Python bytecode files matched by the filename heuristic.
They are not legal notice text. The clean native archive has no such entries and
supersedes that exploratory archive for review. This illustrates why candidate
filename observations cannot substitute for actual content/obligation review.
The native run predates Node resolution/checksum metadata; it does not qualify
those later library increments or the eventual shipping graph.

## Mise integration checkpoint 84: repeatable retained-notice verification

The first-party `tooling/mise-upstream/verify_notices.py` now reproduces candidate
notice evidence checks against the checked-in target/hash index. It verifies all
three retained native archives from run 36985298309 on Windows, including exact
embedded report bytes, raw and normalized notice hashes, package/path correspondence,
clean source identity, missing-notice identities and totals. It reads without
extracting or acquiring anything and never changes approval status.

Two regression tests with ten negative subcases pass, covering external tampering
and internally inconsistent archives even when the archive hash is updated. The
existing upstream workflow discovers the new tests. This is evidence integrity,
not a source import approval, release audit or completed TM-02/TM-12. The maintenance
reference documents invocation, bounds, trust inputs and failure recovery.
## Mise integration checkpoint 85: candidate Go checksum metadata

The candidate fork exposes `Session::go_archive_metadata` using its existing Go
archive facts and target-specific checksum URL. Backend/version/target admission
precedes transport access. It accepts only a bounded single SHA-256 digest and
returns declared metadata without artifact acquisition or execution. The owning
maintenance reference documents response limits, offline requirements and the
remaining catalog, publisher-verification and worker-integration gaps.

A thirteenth fresh-process conformance scenario exercises all three initial target
URLs, case normalization, malformed/multiple/oversized checksums and invalid inputs.
Existing scenarios now check explicit transport denial and session admission for
this method. Windows compliance tests passed (22 plus two symlink skips), and the
factual dependency/notice inventory remained unchanged. Linux library/example
Clippy and all thirteen executable conformance scenarios passed. Native
Windows/macOS execution of this increment remains pending. This does not complete
TM-03/TM-06 or authorize a production source import.
## Mise integration checkpoint 86: candidate Go constrained catalog resolution

The fork now exposes admitted Go version selection using upstream tag parsing,
Go version filtering/ordering and the shared Node/Go constrained selector. Embedded
Go uses supplied HTTP pagination instead of its ordinary Git subprocess path.
It requires exact-pin catalog membership and rejects partial/absent metadata,
conflicting constraints, repeated/foreign-origin pagination and oversized catalogs.
The bounded tag API is separate from ordinary upstream callers, whose pagination
behavior is retained. No dependency or preserved notice changed.

Six new fresh-process scenarios cover successful selection/cache and missing
transport, cycles, foreign origins, 1,000-page and 100,000-tag bounds. Strict Linux
library/example and utility-library Clippy passed after correcting an owned-handle
compile error. All nineteen executable Linux conformance scenarios passed. Windows compliance tests
passed with two platform symlink skips; inventory remains consistent. Native
Windows/macOS results for this increment remain pending. Native constraint discovery,
publisher verification, production worker wiring and full OEP acceptance remain
outstanding. The maintenance reference documents inputs, limits, offline behavior
and unsupported older Go version spellings.
## Mise integration checkpoint 87: Go catalog source correction

Review against the acquisition contract found that checkpoint 86's GitHub-tag
adapter did not implement the specified initial Go release JSON source. The
candidate now uses the official `go.dev/dl/?mode=json&include=all` catalog through
supplied transport, retains the shared selector/upstream comparator and returns
`GoVersionResolution` with the selected version and exact catalog-byte SHA-256.
The added GitHub pagination API was removed; ordinary upstream behavior is restored.
This corrects implementation alignment without changing the OEP requirement.

The adapter bounds response bytes before JSON parsing, requires valid UTF-8,
checks record counts/version lengths and rejects duplicate release identities.
A process-private cache stores only successfully parsed snapshots. New workers need
supplied metadata; no disk cache, Git or alternate catalog fallback is added.
Seven catalog scenarios replace the earlier tag scenarios, including exact digest,
constraint/cache behavior, duplicate/malformed/invalid-UTF-8 records and byte/count
limits. All twenty Linux conformance scenarios and final strict library/example
and utility-library Clippy checks passed; compliance inventory/tests pass
on Windows with the two existing symlink skips. Artifact-file parity, publisher
verification, native directives and production worker integration remain missing.
The earlier Go checksum increment passed all three native hosts in run 36988631019;
that evidence does not qualify this catalog correction.
## Mise integration checkpoint 88: Go target records bound to catalog evidence

The candidate fork's Go target metadata now requires the official stable release
and exact upstream-derived archive filename, matching file OS/architecture/version,
archive kind, positive declared size and valid hash. It then requires the checksum
sidecar to agree with that catalog hash. Results include size and catalog digest;
missing or contradictory catalog records fail before sidecar access. This replaces
the prior checksum-only metadata acceptance without enabling archive acquisition.

The Go embedding adapter owns decoding and matching; upstream URL/layout rules stay
in the parent backend. Canonical stable releases reject duplicate/invalid filenames
and more than 4,096 file records. Conformance now exercises all target fields, absent
release/target, hash disagreement, malformed sidecars and duplicate/excessive file
lists. Linux Rust 1.95 strict library/example Clippy, formatting and all twenty-two
fresh-process conformance scenarios passed. Windows compliance tests pass with two existing
symlink skips and unchanged inventory; documentation checks pass. Native execution,
real archive qualification, source approval and production integration remain
outstanding, so TM-03/TM-06 and the overall OEP remain incomplete.
## Mise integration checkpoint 89: native official-catalog correction checks

Inspection of run 36990464407 at fork head
`32db610a7cc3e460bf678dbd9e2761c5fedb3dad` confirms successful library/example
Clippy, shared utility-library Clippy and fresh-process conformance steps on
Windows, macOS and Linux. These steps cover the twenty scenarios at that revision,
including official Go catalog selection and its byte-identity/error cases.
Windows/macOS jobs were still finishing post-check work when inspected; this
records specific successful steps rather than claiming the whole workflow passed.

This supersedes checkpoint 87's pending native runtime checks. It does not apply
to checkpoint 88's later target-file binding at `0d58ad7c1`; its run 36991065278
was independently confirmed live and still preparing native jobs. Production
installation, source approval and the remaining OEP gates stay outstanding.
## Mise integration checkpoint 90: real Go metadata captured and replayed offline

The first-party fixture provisioner captured official metadata for Go 1.24.13 and
1.25.0 on Linux amd64 GNU, Darwin arm64 and Windows amd64 MSVC: six expected archive
records and seven exact response bodies. The fork's optional fresh-worker replay
passed all six cases and the twenty-two existing scenarios on Linux Rust 1.95 with
Docker networking disabled. A changed expected size failed the actual library-result
comparison. No Go archive was downloaded or executed.

The capture's 2,527,192-byte fixture is retained outside the repository; its hash,
source, case identities and limitations are recorded in
[the replay evidence index](proposals/OEP-0003-mise-integration/go-metadata-replay-evidence.json).
Eight upstream-tooling tests, strict library/example Clippy, formatting, compliance
inventory/tests and documentation checks passed. The maintenance reference and code
map describe the independent fixture oracle and optional fork replay. Native replay,
archive/layout qualification, licensing approval and production wiring remain
outstanding. Fixture capture/replay does not complete TM-06 or the OEP.
## Mise integration checkpoint 91: native catalog-file checks and fixture limits

Run 36991065278 at fork revision `0d58ad7c135d5be63d96b356dbf4ca94708566b9`
passed the strict library/example Clippy and twenty-two-scenario conformance steps
on Windows, macOS and Linux. Windows/macOS jobs were still finishing post-check
work at inspection. This supersedes checkpoint 88's pending native checks, but does
not claim native optional real-metadata replay or full installation qualification.

The capture writer now bounds serialized fixture output to the replay reader's
32 MiB limit before creating a file. This accounts for JSON escaping expansion,
which can exceed the limit even with an input body below 16 MiB. Regression tests
confirm the written digest, refusal to replace an existing fixture, preservation
of its bytes and no output for oversized serialization. All nine upstream-tooling
tests and documentation checks passed. Product Rust sources were unchanged.
## Mise integration checkpoint 92: native real-metadata replay CI

The fork workflow now captures and replays real Go metadata for two versions and
three targets on each native host, in addition to baseline conformance. The
first-party capture helper is pinned by full Git revision and verified SHA-256
before execution. Fixtures remain outside the fork checkout and are retained with
capture reports in target/revision artifacts for 30 days. Live source failures
fail qualification; no synthetic fallback is introduced.

A malformed combined example-path filter was corrected into separate entrypoint
and directory patterns, restoring triggers for future example-only edits. Workflow
syntax/path assertions and the exact pinned capture step passed on Windows,
producing all six cases and seven responses. Compliance inventory/tests passed
with the two existing Windows symlink skips. New native workflow execution is
pending; CI configuration alone does not establish platform replay support.
No product Rust implementation changed. The maintenance reference documents
provisioning, evidence retention, failure behavior and the offline-test distinction.
## Mise integration checkpoint 93: real Go ZIP store qualification

The real Go 1.24.13 Windows amd64 ZIP (87,295,983 bytes; digest recorded in the
maintenance fixture evidence) was independently inventoried and exercised through
verified blob caching, ZIP staging, receipt/publication, leased native version
execution and changed-lock denial/recovery. All 15,738 payload entries matched on
Windows. The real Node regression also passed with its original 3,016-entry manifest.

The first Go attempt failed the default 200:1 per-file expansion guard: three
upstream test files exceed it, with a maximum about 799.21:1. The OEP explicitly
allows reviewed descriptor bounds larger than defaults. Runtime and schema now
retain default 200:1 but allow an explicit, layout-identity-bound ratio up to 1024:1;
the synthetic Go qualification plan requests 800:1. Entry, byte, per-file and depth
ceilings remain unchanged. This is not production Go descriptor approval. Runtime
and shared schema cases reject 1025:1; boundary tests cover 0, 1, 200, 800 and 1024.

The shared Python ZIP oracle includes implicit directories and reproduces the
original Node manifest exactly. Each native CI archive step now provisions and
selects its own exact test. The full Windows locked test suite, strict all-target
Clippy, formatting and all nine compiled CLI task scenarios passed. Linux layout
tests, both real archive parity/publication/changed-lock cases and strict all-target
Clippy passed with container networking disabled. Linux did not execute the Windows
payloads. Updated native CI, Go environment/build integration, other native Go
archives, publisher verification and production source admission remain
outstanding. The reference and code map describe bounds, migration and test scope.
## Mise integration checkpoint 94: native real Go metadata evidence

Fork run `36992169356` completed successfully on Windows, macOS and Linux at
head `b71e447bc10acfbce3627b5f4c8baeb4a0f2f6fb`, tested merge
`58fa4ec749d3af8638bd5aa582fb2928595f2739`. All three jobs passed strict
library/example and utility Clippy, ordinary boundary scenarios, live capture and
six-case real Go replay. This supersedes checkpoint 92's pending native execution.

All three fixture/report artifacts were downloaded before expiry. Independent
checks matched report-to-fixture digests, every response's UTF-8 byte hash/size,
and all six expected target records reconstructed from the retained official
catalog and checksum sidecars. Each fixture contains seven responses and 2,527,192
bytes; hashes and tested revisions are in the Go metadata replay evidence index.
The native runner network was available; only the earlier Linux Docker replay
provides OS-level network-disabled evidence. No product implementation changed in
this checkpoint, and no legal or production admission gate was approved.
## Mise integration checkpoint 95: reconcile current main for native CI

The draft integration branch conflicted with main at `9cc731abdc883810c3ee0455fca0c6a8cbe92a2f`,
preventing pull-request CLI workflows from starting. The merge preserves both
independent implementation-status histories and all incoming builder, acquisition
and executor changes. Node/Go real ZIP qualification steps remain in the combined
workflow. The combined Windows locked suite passed (120 library tests, two explicit
library ignores, plus integration suites), as did strict all-target Clippy,
formatting, all nine real CLI task scenarios and documentation checks. Native
CI on the merged revision remains pending; prior backend evidence is not broadened
to the newly merged behavior. No mise source admission or legal approval changed.
## Mise integration checkpoint 96: published Go compiler execution

The real Go ZIP qualification now declares installation-relative GOROOT in its
synthetic layout and holds its verified installation lease through native version
lookup, `go env GOROOT`, a real standard-library module build, and execution of
the resulting program. On Windows, the published Go 1.24.13 compiler completed
these steps with inherited environment cleared, private build caches and temporary
paths, `GOTOOLCHAIN=local`, `GOENV=off`, `GOWORK=off`, `GOPROXY=off`,
`GOSUMDB=off` and cgo disabled. All 15,738 inventory entries and the subsequent
changed-lock denial/recovery still passed. These settings prevent Go's ordinary
download paths; they are not OS-level network containment.

The Windows locked regression suite, strict all-target Clippy, formatting and all
nine CLI task scenarios passed. The existing native CI Go step selects this same
test, but execution of this increment in CI remains pending. Foreign hosts still
only materialize the Windows archive. This establishes real compiler use from a
verified published payload, not production environment projection, builder handoff,
source admission, publisher signatures or full OEP completion.
## Mise integration checkpoint 97: dependency alias provenance checks

The upstream observer now inspects renamed `package = "mise"` dependencies and
normal/build/development declarations in root and target tables. Every inspected
declaration must name the public fork, disable default features and agree on the
exact revision matched by Cargo.lock. Workspace inheritance and mise patch/replace
overrides fail explicitly until the observer can verify their provenance; an
unaccounted mise lock entry cannot be reported as absent integration. This closes
a false-negative observation path without importing mise or conferring admission.

Six maintenance regression tests passed, including alias/target wrong-source
rejection, conflicting pins, unsupported overrides and orphan lock records.
Detailed behavior and recovery are documented in the maintenance reference.
## Mise integration checkpoint 98: native Go ZIP matrix confirmation

Run `36993866491` at `ca32f4b82efd330b3b01911e643208a550189bbc` passed
the full locked Rust test and strict Clippy steps plus both real Node and Go ZIP
qualification steps on Windows, macOS and Linux. This resolves the native archive
matrix uncertainty in checkpoint 93: all three hosts matched the 15,738 Go entries,
published/verified the installation, and rejected the changed lock identity;
Windows additionally ran the native version command. macOS/Linux tested the
Windows payload as foreign-target data, not a native Go installation.

This run predates checkpoint 96's GOROOT/compiler additions. Its remaining job
steps were still running when checked, so the complete workflow is not claimed
passing. The current reference now distinguishes this native archive evidence
from later compiler tests and from still-missing production backend admission.
## Mise integration checkpoint 99: raw single-file materialization

TM-04 now supports `raw` layouts: exactly one required file supplies the portable
payload destination, with null strip-prefix and dot payload subtree. The verified
private blob is copied unchanged through the existing anchored filesystem layer;
implicit parents consume shared entry/depth budgets and total/per-file limits are
enforced. Existing receipt, layout identity, executable transform and publication
checks remain in force. No backend or executable is admitted by this operation.

Windows full locked tests, strict all-target Clippy, formatting and all nine CLI
task scenarios passed. New tests verify byte parity and leased publication and
reject ambiguous/missing/non-file destinations, escapes and exhausted bounds
without a receipt. Raw schema fixtures cover one valid and four invalid shapes.
Linux initially rejected the positive fixture because a raw Unix entrypoint needs
an explicit executable transform; adding that fixture declaration preserved the
runtime permission gate. All 12 Linux layout tests then passed with networking
disabled, and strict all-target Linux Clippy passed. The corrected fixture also
passed all 11 Windows layout tests and formatting; native macOS CI remains pending. The
reference, contract index, schema, code map and draft acquisition rule now describe
the exact mapping. Real jq/Aqua qualification remains outstanding.
## Mise integration checkpoint 100: real jq raw store qualification

The shared artifact harness now qualifies jq 1.8.1's original Windows amd64 raw
binary (1,026,560 bytes, SHA-256 recorded in the reference). Downloaded bytes match
both the release API digest and published checksum manifest. Original artifact,
checksum manifest and release COPYING are retained outside the checkout. The
synthetic Aqua layout preserves the binary bytes as jq.exe; no upstream code or
artifact is added to the repository or release package.

Windows verified caching, raw staging, receipt publication, held-lease jq version
execution and changed-lock denial/recovery. Linux passed the same materialization
case without executing the Windows binary, with Docker networking disabled, plus
strict all-target Clippy. Native CI now provisions and tests jq independently of
Node/Go, retaining release COPYING in runner temporary storage. Full Windows
locked regression, strict Clippy, formatting and all nine CLI scenarios passed.
All three real-artifact cases passed together, including Go GOROOT, compilation
and native program execution; updated native CI remains pending. This is store
qualification, not Aqua registry interpretation, publisher verification, legal
approval or production backend admission.

## Checkpoint 101: integration with current Node reporting work

Merged main through `fdd4dad` while preserving its native Mocha/report changes and
all tool-store changes. The only textual conflict was independent status-history
appends. Checkpoints 84–100 now live on this dedicated tool-management page beside
1–83; all 17 moved sections were verified unchanged apart from surrounding
whitespace. The general implementation-status index already links here. Future
tool-management evidence should be recorded here to avoid repeated conflicts with
unrelated implementation histories.

The combined Windows locked suite passed (122 library tests plus integrations;
two explicitly ignored library cases), as did strict all-target Clippy, formatting,
all nine CLI task scenarios and documentation checks. This merge does not broaden
backend qualification or claim complete captured-build/native CI success.

## Checkpoint 102: current fork notice evidence retained

Downloaded all three cargo evidence artifacts from successful fork run
`36992169356`, bound to tested merge `58fa4ec749d3af8638bd5aa582fb2928595f2739`
and current candidate head `b71e447bc10acfbce3627b5f4c8baeb4a0f2f6fb`.
The offline verifier matched embedded/external reports, every indexed notice's
raw and normalized hashes, sizes, package/path identities and missing-notice list.
The checked-in index now selects these artifacts and retains the previous run's
hash anchors under `previous_evidence`. Raw artifacts remain outside the checkout.

Counts are unchanged: Darwin 952 packages/1,501 notices/98 missing, Windows
975/1,545/96 and Linux 955/1,510/98. The current candidate still has no recorded
human approval in fork PR 2 and all evidence approval flags remain false. Owner
assignment is not substituted for review. This refresh prepares reviewable source
evidence; it is neither exact shipping-graph review nor production admission.
The maintained reference gives the current download and verification commands.

## Checkpoint 103: missing-notice source provenance audit

The union of missing-notice identities across the current three reports is 98
packages. An offline, no-extraction audit verified 89 cached registry crate
archives against their retained package checksums and captured original manifest
and VCS metadata hashes plus repository/commit/path pointers. The remaining nine
are fork workspace packages and need root/workspace notice applicability review.
The new source-pointer inventory records these facts without treating any notice
as recovered or any license as approved.

The initial age-core inspection confirms its cached crate omits standalone license
files and supplies a repository commit/path. Larger groups include 19 packages
pointing to conda/rattler, 13 to aubepkg/aube and nine to prefix-dev/sigstore-rust.
These groupings prioritize source review; they do not prove common license coverage.
No dependency, upstream source or production approval changed.
