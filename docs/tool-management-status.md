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

## Checkpoint 104: exact-commit supplemental notice collection

Preserved 18 original root notice files from 14 source commits identified in the
checksum-verified crate metadata audit. These commits correspond to 45 package
entries across rage, aube, rattler and sigstore-rust. Downloaded bytes were checked
against commit-tree Git blob identities, SHA-256 and sizes, then independently
rechecked from retained local files. The supplemental evidence index records
immutable URLs and binds the source-pointer inventory; original text remains
outside the checkout. No package disposition changed.

Aube's root also contains a separate licenses directory, explicitly left unreviewed
in this root-file inventory. Source/package applicability, nested notices, headers,
shipping graph and obligations still need review; collection does not close the
license gate or reduce the original missing-notice counts. Documentation structure
checks passed.

## Checkpoint 105: nested Aube notices retained

The pinned Aube licenses directory contains four files: pnpm-LICENSE,
pnpm-plugin-trusted-deps-LICENSE, vltpkg-benchmarks-LICENSE and
yarnpkg-extensions-LICENSE. All were retained unchanged outside the checkout and
verified against tree `47ed0fd78b3a5e4ae97abd3ad5063269803b1b0a` and their
Git blob/SHA-256/size identities. The supplemental index now covers 22 files across
the same 14 source commits and 45 package entries; all 22 retained files were
rechecked. This closes collection of that directory, not license applicability
or shipping-graph review. Approval and missing-notice dispositions are unchanged.

## Checkpoint 106: scope historical experiment reruns to their inputs

The macOS historical experiment workflow previously rebuilt on every integration
branch push, including notice-index and status-only edits. Its automatic path
filter now covers the complete experiment directory, its own workflow and checkout
line-ending attributes. Manual dispatch remains available. Inspection confirmed
its preparation and runtime inputs are owned by the experiment directory and it
uses an explicitly pinned compiler, not the root Cargo project.

Workflow YAML and representative path matches were checked. Production CLI and
contract/observation triggers are unchanged; no existing run was cancelled and
no missing result was converted into a pass. Documentation describes when manual
requalification is needed and requires adding future out-of-directory inputs to
the filter.

## Checkpoint 107: repeatable supplemental notice verification

Added an offline verifier for the supplemental evidence index and retained files.
It checks the source-pointer byte hash, package/repository/commit bindings,
immutable URLs, safe retained paths, both file hashes and sizes, bounded reads and
explicit unapproved flags. Duplicate fields/records and mismatched inputs fail;
no network, extraction, execution or mutation occurs. The existing maintenance
workflow discovers its tests automatically.

All 22 retained files across 14 source commits passed, totaling 78,323 original
bytes. Fourteen maintenance tests passed, including missing/modified files, source
identity drift, path escapes, duplicate JSON/records, Git hash mismatch and an
attempt to present approved evidence. This checks consistency of trusted evidence,
not completeness or legal applicability. The reference and code map are updated.

## Checkpoint 108: second real Go release store/compiler qualification

Go 1.25.0 now has a separate pinned Windows ZIP fixture and independent inventory
of 16,087 payload entries. Its digest/size match the existing official catalog
evidence. The provisioner accepts an explicit version choice while preserving
1.24.13 as its default, rejects wrong/unsupported versions without writing output,
and gives the new case independent GO125 fixture variables. Native CI provisions
and selects both releases separately.

Windows passed complete inventory, receipt publication, changed-lock denial and
recovery, GOROOT lookup and native compilation/execution while leased. Linux
passed foreign-target materialization with networking disabled and strict all-target
Clippy. Full Windows locked tests, strict Clippy, formatting and all nine CLI task
scenarios passed. Workflow YAML and documentation checks passed. Native CI for
this increment remains pending; two real store/compiler cases do not substitute
for production resolver, broker, policy, native Linux/macOS or backend admission.

## Checkpoint 109: draft tool-selection grant payload schema

TM-01 now includes the closed allow-grant payload schema with synthetic fixtures.
The checker passed four valid/34 invalid ordinary mutations and one valid/two
invalid offline cases, alongside existing tool contracts. Fields, safe integers,
protocol/audience and digest/UUID shapes, resolve-only scope and offline operation
restrictions are explicit. Unix-second validity encoding and separated offline
operation scope remain proposed wire choices requiring maintainer review.

Inspection confirmed the existing policy signature verifier uses the Ed25519
algorithm label, whereas this OEP specifies EdDSA. No policy behavior was changed.
Tool signature verification, request/context binding, lifetime enforcement, agent
lifecycle and runtime/schema agreement remain unimplemented; unsigned fixtures
are not grants. Contract/reference documentation and the implementation sequence
now state the exact boundary. Documentation checks passed.

## Checkpoint 110: initial signed tool-grant verifier

TM-01/TM-09 now have a first-party compact EdDSA verifier using the existing
cryptography dependencies. It rejects unknown/duplicate fields, unpinned keys,
bad signatures, mismatched caller-supplied context and unauthorized operations.
Signed lifetime caps and fixed remaining-time monotonic deadlines bound online
and explicitly allowed offline use. Binding changes, clock rollback/reversal,
expiry and explicit invalidation cannot revive the same instance. No token or
verified-object persistence API is provided. Existing configuration-policy
signature behavior is unchanged.

Five focused tests passed on Windows and Linux, including shared valid/invalid
schema fixtures, tampering, wrong keys/identities, operation separation, expiry,
rollback and late receipt. Full Windows locked tests, strict all-target Clippy,
formatting and all nine CLI task scenarios passed; the added focused cases were
also rerun after the full suite. Linux strict all-target Clippy passed with
networking disabled. Schema and documentation checks passed. Native macOS checks
for this increment remain pending.

Caller context/key authentication, agent invalidation notifications, authorizer
interoperability and actual launch integration remain absent. The documented
interpretation of the draft 60-second online/900-second offline windows still
requires maintainer/service-contract review. These checks neither complete
MISE-11 nor approve the production mise dependency or distribution obligations.

## Checkpoint 111: bounded worker control framing

TM-05 now has an initial duplex Read/Write control codec: four-byte big-endian
lengths, 8 MiB JSON-object bodies and one 32 MiB budget across both directions,
including prefixes. It reuses strict JSON parsing, rejects oversized lengths
before body allocation/read, validates outgoing JSON before transmission and
makes all input/transport failures terminal. Errors omit payload and transport
implementation details. No dependency or upstream code was added.

Five focused tests passed on Windows and Linux: fragmented reads/writes, exact
frame limits, shared budgets, malformed/duplicate/nonfinite/deep JSON, truncated
frames and permanent failure handling. Full Windows locked tests, strict
all-target Clippy, formatting and all nine CLI task scenarios passed. Linux
strict all-target Clippy passed with networking disabled. Documentation checks
and diff checks passed; native macOS verification remains pending.

Typed envelopes, unknown-field/protocol/identity admission, request/result/cancel
sequencing, private OS channels, deadlines, embedded dispatch and executor
containment remain unimplemented. The codec cannot interrupt arbitrary blocking
Read/Write operations and provides no authorization. The reference, code map
and OEP distinguish this framing component from a qualified worker; TM-05 is
not complete.

## Checkpoint 112: worker envelope correlation and terminal exchange

The worker now has closed outer request/response envelopes, exact protocol and
request/context correlation, recognized operation/platform syntax and bounded
sorted capability IDs. One exchange consumes at most one terminal response;
malformed or mismatched responses cannot be retried against the same instance.
An explicit cancel may be emitted once, and a racing success after cancellation
is discarded. Abort handles caller-observed timeout/transport loss. Diagnostics
are code-only records. Payloads remain explicitly untrusted, not typed backend
requests or publication authority. Shared request UUID and platform validation
reuse existing tool identity rules.

Nine framing/exchange tests passed on Windows and Linux, including shared schema
fixtures, unknown fields, identity mismatches, terminal-state reuse and cancellation
races. The schema checker passed one valid/10 invalid request envelopes and one
valid/seven invalid response envelopes, plus existing contracts. Full Windows
locked tests, strict all-target Clippy, formatting and all nine CLI scenarios
passed; the added shared-fixture test and five grant regressions were rerun after
the full suite. Linux strict all-target Clippy passed with networking disabled.
Documentation/diff checks passed. Native macOS checks remain pending.

The new schema describes only outer envelopes. Operation-specific payloads,
worker-side cancel/dispatch, private OS channels, handshake/operation deadlines,
process supervision and executor containment are still absent. Cancel encoding,
code-only diagnostics and cancellation-race handling remain draft contract choices.
Reference documentation, code ownership and OEP implementation/runtime pages now
state these boundaries; this does not complete TM-01 or TM-05.

## Checkpoint 113: expanded exact-commit supplemental notices

Retained 31 additional original root notice files from 22 source commits,
associated with 31 more entries in the missing-notice inventory. The complete
supplemental index now contains 53 files (234,668 bytes), 36 source commits and
76 package entries. Every added file matched the recorded commit tree's Git blob
identity, SHA-256 and size. Original bytes remain outside the checkout; public
immutable URLs and identities are retained in the index.

The offline verifier now handles narrowly defined GitHub publisher URL variants
(trailing slash, .git and tree/ref/path), while still requiring the exact separate
crate VCS commit and canonical immutable notice URL. Traversal/foreign hosts,
query strings and changed commits fail. Empty file inventories fail, and output
now reports entries with and without supplemental candidates. All 15 maintenance
tests and the complete real-byte verifier passed, as did documentation/diff checks.
No Rust runtime or production dependency changed in this increment.

Twenty-two entries still lack supplemental candidates: 11 root-name searches
found none, one repository is outside the current GitHub verifier scope, one
crate lacks a recorded VCS commit and nine entries need fork workspace review.
This is collection evidence, not legal interpretation: original missing-notice
counts, unreviewed dispositions and false approval/release flags are unchanged.
The maintenance reference now documents the exact normalization and remaining
search limitations. Production source admission still requires recorded review.

## Checkpoint 114: typed command lookup on verified leases

Installation leases now retain the receipt snapshot verified during acquisition
and expose borrowed command metadata from the exact selected closure. Native and
interpreter launch shapes preserve installation-relative paths and typed ordered
arguments. Missing or invalid names fail without PATH/wrapper/extension guessing;
multiple selected receipts providing the requested command fail ambiguity.
Windows-target lookup follows receipt uppercase comparison; Unix uses exact names.

Windows passed all 16 active receipt tests, the full locked Rust suite, strict
all-target Clippy, formatting and all nine CLI scenarios. Linux passed 17 active
receipt tests and strict all-target Clippy offline. Tests cover typed interpreter
references, literal argument preservation, name rules, ambiguity across dependencies
and the distinction between retained metadata and a newly reverified selection.
The real jq fixture now derives its executable from leased command metadata;
Windows passed publication, lookup, native version execution and changed-lock
denial, and Linux passed foreign-target lookup/materialization without execution.
Documentation/diff checks passed. Native macOS checks remain pending in CI.

Lookup does not rehash current bytes, monitor edits, return executable host paths
or authorize a process. The returned borrow is tied to the held lease; launch
consumers still need current content/authority checks, path resolution and process
supervision. No CLI which/exec, shims or production backend admission is enabled.
The reference and code map document this TM-07 foundation and its limits.

## Checkpoint 115: concurrent worker framing and shared byte reservations

Worker framing now supports separately owned reader/writer halves sharing one
atomic control-byte budget and terminal state. A blocked response read holds no
lock needed by cancellation writes. Failure or explicit abort in either half
prevents subsequent successful operations in both, including an in-flight read
that later completes. The sequential channel API remains available. Framing now
lives in worker/framing.rs; exchange identity/lifecycle remains separate.

Review found and fixed a prefix-budget edge case: receive reserves its four-byte
prefix before transport access, then reserves the body before allocation/read.
Send reserves its entire frame before writing. Concurrent operations cannot
consume more control bytes than reserved; failure does not refund reservations.

The final Windows full locked suite passed, alongside 12 focused worker tests,
strict all-target Clippy, formatting and all nine CLI scenarios. Linux passed
13 focused tests and strict all-target Clippy with networking disabled. Tests
cover cancellation writes during pending reads, shared abort, simultaneous budget
exhaustion and rejection before prefix read. Linux additionally exercised a real
close-on-exec Unix socketpair. Documentation/diff checks passed; native macOS
verification remains pending.

No supervisor/process channel is enabled. Generic Read/Write remains blocking;
abort cannot interrupt a syscall or retract in-flight bytes. Actual OS deadlines,
transport shutdown, inherited descriptor/Windows handle restrictions, worker
payload admission, backend dispatch and executor containment remain unfinished.
The reference documents these caller obligations; this does not complete TM-05.

## Checkpoint 116: native private control-channel allocation

The worker transport factory now creates two connected endpoints using a Unix
socketpair or two Windows anonymous pipes, with no stdin/stdout, public socket
name, listening port or environment capability. It checks close-on-exec/non-
inheritable flags on every returned native handle. Endpoints can be consumed
into separately owned reader/writer halves for shared-budget framing; borrowed
native handle traits support future explicit supervisor integration. No new
crate dependency or external executable was added.

Windows passed 13 focused worker tests, the full locked suite, strict all-target
Clippy, formatting and all nine CLI scenarios. Linux passed 14 focused tests and
strict all-target Clippy with networking disabled. The new native test transfers
a 2 MiB JSON body both ways and verifies peer closure invalidates both framing
halves; the factory checks non-inheritance during allocation. Documentation/diff
checks passed. Native macOS verification remains pending.

Allocation is not a running or isolated worker. Process image validation,
restricted child inheritance, inherited-channel authentication, OS deadlines,
pending-I/O interruption, dispatch and executor containment remain unimplemented.
The test watchdog bounds only the harness. The reference, code map and OEP
runtime/implementation pages distinguish this native allocation foundation from
complete TM-05 acceptance.

## Checkpoint 117: real Temurin archive and leased Java compilation

The independent archive oracle now pins the official Temurin 21.0.6+7 Windows
amd64 JDK ZIP at 204,643,847 bytes and SHA-256
`897c8eebb0f85a99ccecbd482ebae9a45d88c19d6077054f6529ebab49b6d259`.
The original archive and checksum sidecar are retained outside the checkout;
all legal files remain in the materialized payload and byte inventory. The
fixture has 576 payload entries and uses the unchanged 200:1 extraction bound.
No dependency or upstream code was imported into the production crate.

Windows passed independent inventory parity, publication, held-lease Java and
javac lookup, exact runtime-build output, real class compilation/execution and
changed-lock denial/recovery. Linux passed the same Windows-payload store case
without executing it, with container networking disabled. Both hosts passed
strict all-target Clippy; Windows additionally passed the full locked Rust
suite, formatting and all nine CLI task scenarios. Documentation, workflow YAML
parsing and diff checks passed. CI now provisions the fixture on all three hosts;
new-revision CI and native macOS store results remain pending.

The owning reference, code map and OEP implementation companion describe the
fixture. Admission remains synthetic. This does not establish publisher-signature
verification, license/distribution approval, Java backend admission, native
Linux/macOS JDK execution, OS process containment or product builder integration.
Production mise import and the remaining OEP-0003 requirements are still open.

## Checkpoint 118: worker-side single-operation session

The worker-side session now shares the supervisor's bounded outer-request and
terminal-response validation. It accepts one exactly correlated cancel envelope;
malformed, foreign or duplicate cancellation permanently ends the session.
It encodes at most one terminal outcome, rejects success after cancellation and
consumes an invalid output attempt rather than permitting a replacement result.
Results remain untrusted. A transport send failure requires teardown, not retry.
The new session module owns sequencing, not backend effects or process lifetime.

Windows passed 16 focused worker tests, the full locked suite, strict all-target
Clippy, formatting and all nine CLI scenarios. Linux passed 17 focused tests and
strict all-target Clippy with networking disabled. The new tests cover invalid
and oversized output, repeated terminal attempts, malformed/foreign/duplicate
cancel, abort and cancellation races. A real native channel carries the initial
request, cancel and terminal diagnostic, then observes peer closure. Documentation
and diff checks passed; native macOS verification of this increment is pending.

Reference, code-map and OEP runtime/implementation pages now describe both sides
of the control sequence. This is not a running mise worker: same-binary process
creation, image/channel authentication, restricted inherited handles, OS deadlines,
backend interruption, typed payload admission and actual dispatch remain absent.
The native test uses threads and its watchdog only bounds the test itself.

## Checkpoint 119: bounded worker response encoding

Worker terminal encoding previously serialized the full result before checking
its frame size. It now validates the existing shared depth/entry budget before
recursive serialization, then stops before an append would exceed 8 MiB,
including JSON escaping and envelope fields. Buffer reservation grows
geometrically within the frame cap. Encoding failure poisons the session and
returns no response bytes. This bounds the serialized buffer, not allocations
already made by a backend while constructing its result.

The JSON tree validator remains owned by config/policy.rs and is reused unchanged
for decoded input and pre-encoding checks. There is no new dependency, wire-format
change or separate copy of the depth/entry rules. Tests cover exact-limit output,
escaping-induced overflow, excessive nesting/entries and terminal failure after
invalid encoding. Windows passed 17 focused worker tests, the full locked suite,
strict all-target Clippy, formatting and nine CLI scenarios. Linux passed 18
focused tests and strict all-target Clippy offline. Documentation and diff checks
passed; the reference, code map and OEP runtime now explain pre-write bounds.

Separately, run 37002649274 at Java-fixture commit 6d32382 completed its Ubuntu
Build CLI job successfully, including real Temurin parity/publication/changed-lock
checks. Its macOS Java step is running and Windows Java step remains pending.
Those historical CI results do not verify this encoding change or prove native
Linux/macOS Java execution. Production worker dispatch, OS deadlines, authenticated
inheritance, full tool-management integration and licensing admission remain open.

## Checkpoint 120: real Java catalog identity correction

Bounded capture of the three public mise Java catalogs found the exact Temurin
JDK record uses canonical version `temurin-21.0.6+7.0.LTS`, not the earlier
synthetic store fixture spelling `temurin-21.0.6+7`. The fixture and manifest
provisioner now use the actual catalog identity. Archive bytes, stripped directory
and runtime build string are unchanged. Existing experimental manifests must be
regenerated at a fresh path; no automatic alias or lock migration is introduced.

The new java-catalog-evidence.json records each original catalog's size, SHA-256,
record count, selected target, JDK identity, archive URL and declared checksum.
All three observations were verified against retained original response bytes
outside the checkout. The Windows checksum also agrees with the retained official
Temurin release sidecar and tested archive. Linux/macOS target archives were not
acquired, and this capture alone is not Rust backend replay or publisher proof.

With the corrected version, real Java store qualification passes on Windows and
Linux: 576 entries, publication and changed-lock denial/recovery, with actual
compiler/runtime execution on Windows. Windows full locked tests, strict Clippy,
formatting and nine CLI scenarios pass; Linux strict Clippy passes offline.
Documentation and diff checks pass. The reference explains the distinct canonical,
release, archive-directory and runtime spellings and the fixture update remedy.

Historical CI run 37002649274 at 6d32382 has successful Ubuntu and macOS Build CLI
jobs, including the real Java store case with the older synthetic version. Its
Windows Java step remains running. These results do not verify this corrected
identity on macOS or complete Java backend admission. Real metadata replay through
the fork, publisher verification and production integration remain outstanding.

## Checkpoint 121: actual Java backend metadata replay

The maintained fork's library-only harness now accepts a captured Java fixture
through OYZU_JAVA_METADATA_FIXTURE. A fresh Java-only session supplies original
catalog bytes through the explicit callback; the real Java backend resolves the
canonical Temurin 21.0.6+7.0.LTS identity for all three targets, matching expected
archive URLs and checksums, rejecting an unavailable version and issuing exactly
three captured requests. No network fallback, archive acquisition or mise CLI is
used. Linux passes 22 ordinary cases plus this three-target replay with container
networking disabled; strict library/example Clippy and pinned formatting pass.

The new first-party capture helper retains bounded original catalog responses
and independently selects exact target records. All 18 maintenance tests pass,
including wrong variant/target/checksum, ambiguity, malformed encoding and size
bounds. A fresh 8,491,223-byte fixture has SHA-256
f21a1f34bec60e46c0672acc598a41cb028b6dc46e00592a90be6779846cc05a and remains outside
the checkout. Reference/code-map updates describe provisioning and limitations.
Windows/macOS fork replay is pending native CI; declared checksums remain
unauthenticated publisher evidence. No production mise import, dependency/notice
change, legal approval or completed backend-admission claim is introduced.

## Checkpoint 122: bounded embedded Go/Java response collection

Fork commit aad17fd6cb748fdc67540388430f41f67169d698 introduces a shared bounded
HTTP collector on the existing authorization/embedding callback path. Go catalog
collection is capped at 16 MiB and sidecars at 128 bytes; embedded Java catalogs
are capped at 16 MiB with a subsequent 100,000-record check. Declared/body lengths
and cumulative decoded chunks are checked before append. A stream error returns
no partial result. This is not a cap on provider buffering, JSON parser allocation
or total process memory, and does not implement worker operation deadlines.

Linux passed two focused HTTP boundary tests, strict utility and library/example
Clippy, pinned formatting, all 22 ordinary conformance cases and combined captured
Go (six cases) and Java (three targets) replay with networking disabled. Tests
cover exact-size acceptance, declared oversize, unknown-length chunk overflow and
interrupted streams. Fork inventory remains consistent with 1,109 unreviewed
records; 24 compliance tests pass with two Windows symlink skips. No dependency,
license/notice or approval changed. The owning reference and fork change record
explain limits and evidence; workflow YAML and documentation/diff checks pass.

Earlier run 37004801980 at da430ba passed Java replay steps on Windows/macOS.
Ubuntu failed during public Go metadata capture with HTTP 404, skipping Java;
a fresh local capture of all seven responses succeeded. This does not identify
the CI failure's root cause. Java CI is now independent of Go capture once shared
boundary checks pass, while any failed step still fails the job. New native CI
for the bounded reader remains pending. Production import, worker/broker wiring,
backend admission and remaining OEP requirements remain open.

## Checkpoint 123: immutable lease selection identity

InstallationLease no longer exposes a separately mutable selection-digest String.
Its selection_digest() accessor borrows the identity from the same verified
receipt snapshot used by command lookup and journal publication. This removes a
possible contradictory caller-visible identity without changing lock, receipt,
journal or CLI formats. Experimental Rust callers and archive/layout/receipt
consumers now use the accessor; the reference documents that API migration.

Windows passed the full locked Rust suite, strict all-target Clippy, formatting
and all nine CLI scenarios. Linux passed 12 layout and 17 active receipt tests
plus strict all-target Clippy with networking disabled. Existing assertions bind
command metadata and journal identity to the accessor and independently verified
selection. Documentation/diff checks pass and the code map records one snapshot
as the identity owner. No additional backend/native archive execution is claimed
for this API-only change, and no dependency or licensing status changed.

A lease identity still represents its acquired snapshot, not current policy or
protection against subsequent same-user edits. Installation prune/recovery,
consumer process/shell lifecycle and production integration remain outstanding;
this correction does not complete TM-04 or OEP-0003.

## Checkpoint 124: native Java metadata replay evidence

Fork run 37005919694 at aad17fd6cb748fdc67540388430f41f67169d698
(tested merge fb74cdee070ab6b9a44d6d9dc3e6a6e4d8016067) passed bounded
HTTP utility tests, strict library/example checks and real Go/Java replay steps
on Windows amd64, macOS arm64 and Linux amd64. Linux/macOS jobs completed;
Windows was still in cache cleanup when this checkpoint was recorded. No final
workflow conclusion is inferred from successful test steps.

Downloaded all three original Java fixture/report artifacts outside the checkout.
Each 8,491,223-byte fixture matches its capture report, and the pinned capture
helper independently reproduces all three target cases from its retained response
bytes. java-native-replay-evidence.json records artifact IDs, per-host raw hashes,
exact source/merge revisions, helper identity and target metadata. The reference
now replaces the prior pending-native-replay limitation with this measured result.

This verifies exact metadata selection across native hosts and foreign targets,
not Java range resolution, publisher signatures, native archive execution on all
hosts, OS network containment or production backend admission. Java vendor/build
identities cannot use the current canonical-semver-only Node/Go selector unchanged.
Legal/release approval remains false; production integration and the other OEP
requirements remain incomplete.

## Checkpoint 125: targeted Temurin version selection

Fork c3c9ca73d adds Session::resolve_java_version for explicit Linux amd64 GNU,
macOS arm64 and Windows amd64 MSVC targets. It reuses Java's existing ordering
and fuzzy matcher, admitting GA Temurin HotSpot JDK metadata with no extra
features. Numeric/vendor prefixes, exact versions and latest intersect all
constraints; full vendor/build identity is preserved. Unsupported selectors,
other vendors, unknown targets and excessive constraints fail before acquisition.
The same ordering helper serves ordinary Java listing and embedded selection;
feature preference saturates at zero instead of underflowing on overlong lists.

Linux strict library/example Clippy, formatting and library-only build pass.
Network-disabled replay passed all 22 ordinary scenarios plus real Go and Java
fixtures. Java exercised exact/prefix/latest with exact constraints, preserved
catalog identity, conflicts, absent versions, invalid request/target syntax and
257-constraint rejection, including no transport calls for invalid inputs.
Compliance inventory is unchanged; 24 tests pass with two Windows symlink skips.
The owning reference and fork change record document the API and its limits.
Documentation checks pass. Native CI for this new selection seam is pending;
the preceding bounded-reader run 37005919694 completed successfully on all hosts.

This does not complete the resolver: native builder range languages, explicit
prereleases, ambiguous-catalog rejection, returned metadata provenance and the
full target matrix remain open. Production import, admission, worker/broker
integration, legal approval and all other outstanding OEP work remain required.

## Checkpoint 126: native transport closure under pending I/O

Added two native transport regressions for the shared TM-05 worker boundary.
One leaves a frame length prefix incomplete; the other writes a 2 MiB frame
while the peer never drains its transport. Neither completes during the initial
observation window. Closing the peer then makes the pending operation fail and
marks the opposite framing half terminal. Windows and Linux pass both cases
and the existing large-frame roundtrip/peer-close test.

Windows passes the full locked Rust suite, strict all-target Clippy, formatting
and nine CLI task scenarios. Linux passes the three focused native-channel tests
and strict all-target Clippy with container networking disabled. Documentation
and diff checks pass. The owning reference describes exactly what closing the
peer establishes; no production API or behavior changed in this increment.

These tests do not implement deadlines, child process creation, inherited-handle
restrictions, descendant cleanup or same-binary dispatch. The existing executor
supervisor controls Docker containers and cannot be reused directly as the native
tool-worker supervisor. Publication still needs confirmed worker shutdown and
independent result validation; this increment does not complete TM-05 or OEP-0003.

## Checkpoint 127: native supervisor implementation constraints

Inspected existing executor shutdown: it is Docker-specific and does not supply
native tool-worker process ownership. Checked stable Rust's Windows process API:
spawn_with_attributes remains nightly-only, so the restricted-handle-list adapter
must use native Windows bindings without changing the compiler channel or falling
back to broad inheritance. The runtime companion now records this constraint,
subsystem ownership and source-linked synchronous-I/O cancellation limitations.

The draft also specifies native spawn failure, unrelated inherited handle,
incomplete-frame exit, unread-response, cancellation/success race, descendant
handle retention and concurrent-worker cleanup cases. These extend the process
acceptance details; they are not passing test claims or accepted design status.
Documentation/diff checks pass. No production behavior changed this turn.
Native spawning, deadlines and process shutdown remain unimplemented.

## Checkpoint 128: shared validated worker request context

ToolWorkerExchange and ToolWorkerSession now expose context() as a borrowed
ToolWorkerRequestContext containing request ID, context digest, backend-release
digest, target platform and capability IDs. These fields come from the same
validated owned request used for cancel/response correlation. Future dispatch
can compare them with trusted supervisor state without reparsing untrusted JSON
or keeping a separately mutable identity. The context remains an inspectable
snapshot after termination and never grants authority or renews an operation.

The existing native-channel request/cancel/terminal integration test now verifies
all exposed header fields before worker cancellation. Windows passes the full
locked Rust suite, strict all-target Clippy, formatting and nine CLI scenarios.
Linux passes 20 worker tests and strict all-target Clippy with networking disabled.
Reference and code-map updates describe ownership and admission limits;
documentation and diff checks pass. No wire-format, dependency or CLI change.

Native spawning, image/channel authentication, OS deadlines, capability admission
and backend dispatch remain open. Exposing syntactically valid identifiers is not
a replacement for those checks and does not complete TM-05 or OEP-0003.

## Checkpoint 129: targeted selection passes all native CI hosts

Fork run 37008019904 completed successfully on Windows amd64, macOS arm64 and
Linux amd64 at c3c9ca73dde45ad7e497ca25536ba8aec91c04b8. This supersedes the
pending-native-CI note in checkpoint 125. All three jobs ran the embedding
library checks and captured Go/Java replay, including targeted Java selection,
constraint intersection and invalid-input rejection. Windows cache cleanup also
completed; the workflow conclusion is now success rather than inferred from
individual test steps. The owning reference links the exact run and revision.

This is qualification evidence for the experimental library boundary. It does
not establish the missing full resolver, native worker supervision, production
import, license approval or end-to-end tool-management commands. Documentation
and diff checks pass; no runtime code changed in this checkpoint.

## Checkpoint 130: normalized request records share projection validation

ToolRequestIdentity::parse now decodes a closed JSON identity record against a
caller-trusted canonical catalog-ID set. It reuses configuration projection's
normalizer and digest calculation, preserves literal version syntax, requires
sorted unique constraint/capability sets and rejects digest drift. Unknown fields,
duplicate JSON keys, absent catalog IDs and malformed records fail. The shared
strict parser bounds bytes, depth and JSON entries before typed conversion.
The per-value projection bound remains before copying configuration values.

The roundtrip test starts with actual effective configuration projection, then
rejects changed requests/digests, unsorted and duplicate sets, unknown command
fields, duplicate JSON keys and an empty admitted catalog. Existing golden
request identity and frozen-selection tests remain passing. Windows full locked
suite, strict Clippy, formatting and nine CLI scenarios pass; after restoring
the pre-copy value check, all five request tests were rerun on Windows/Linux.
Linux strict all-target Clippy also passes with networking disabled. Reference
and code-map updates describe one owner for normalization and its trust boundary;
documentation/diff checks pass. No dependency or CLI surface changed.

This is the normalized-request portion needed by the future typed resolve payload,
not that complete payload or dispatch implementation. Catalog provenance, broker
handles, trusted operation-context binding, native process supervision, backend
admission and the remaining OEP requirements are still open. A recomputed request
digest is not authorization and does not identify a trusted sender.

## Checkpoint 131: immutable normalized tool request identity

ToolRequestIdentity fields are now private, with borrowed requests(),
native_constraints(), required_capabilities() and digest() accessors. Only
configuration projection and checked record parsing construct the identity.
Callers can no longer mutate constraints or requests independently of their
computed digest. Frozen selection and all existing consumers use the accessors.
Serialized field names, canonical normalization and golden digests are unchanged.
The reference documents migration from the experimental field API and the code
map records the immutable identity boundary.

Windows passes the full locked Rust suite, strict all-target Clippy, formatting
and nine CLI task scenarios. Linux passes all five request tests, including
projection/parse roundtrip and stale-lock denial, plus strict all-target Clippy
with networking disabled. Documentation/diff checks pass. No dependency or CLI
behavior changed. Immutability establishes internal consistency, not permission,
trusted catalog provenance, operation freshness or sender authentication.

The complete resolve payload and native worker supervision remain unfinished;
this invariant correction does not complete TM-03/TM-05 or OEP-0003.

## Checkpoint 132: shared normalized-request shape contract

Added the draft request-identity JSON schema and one golden valid plus 18
malformed-shape fixtures shared by the schema checker and Rust parser tests.
The valid digest is checked against actual effective-configuration projection.
Fixtures cover missing/unknown fields, non-map requests, malformed canonical IDs,
nonliteral/empty/control-containing versions, empty/duplicate constraints,
duplicate/invalid capabilities and malformed digests. Shared testing caught and
corrected regex end anchoring that initially accepted a final newline in a version.

All tool schema fixture checks pass. Windows passes the full locked Rust suite,
strict all-target Clippy, formatting and nine CLI scenarios. Linux passes six
request tests and strict all-target Clippy with networking disabled. Documentation
and diff checks pass. Reference and contract index distinguish shape validation
from runtime UTF-8 byte/aggregate limits, ordering, catalog membership and digest
recomputation. This adds no stable API claim or maintainer design acceptance.

The complete resolve-operation payload, catalog/broker context, native process
supervision and production integration remain outstanding. Schema agreement for
this record does not qualify a backend or complete OEP-0003. Existing CI run
37010483750 remains active at the preceding immutable-request revision; it is
not evidence for this later schema/test commit.

## Checkpoint 133: native archive CI results at immutable-request revision

[CI run 37010483750](https://github.com/micahlmartin/oyzu/actions/runs/37010483750)
tests commit 1c2ffaf9236bd835bc3ae1ba764e1292bd501fc2. Linux and macOS Build CLI
jobs completed successfully. Windows job 110848763744 completed formatting,
strict lint, unit/discovery tests, and all five real archive qualification steps:
Node, Go 1.24, Go 1.25, Temurin and jq. Each archive step includes store parity
and changed-lock denial. The Temurin step completed at 2026-10-02T13:28:53Z;
the jq step completed at 13:28:55Z. Native policy integrity checks also passed.

At this observation Windows release CLI compilation remains in progress; later
Cargo conformance steps and the complete workflow are not yet verified. This run
does not cover subsequent request-schema commit 084fcfd. Archive qualification
uses the existing synthetic admission harness and does not establish production
backend admission, distribution approval, full tool CLI behavior or completion
of OEP-0003. No product behavior changed in this evidence update.

## Checkpoint 134: exact backend descriptor schema identity matching

Corrected backend descriptor schema end anchoring so source pins and all four
SHA-256 identity fields reject a trailing newline, matching the existing Rust
parser. Five shared invalid fixtures reproduce the discrepancy before the fix
and pass after it. All tool schema fixture checks and both Windows backend
parser tests pass, including unchanged golden identity hashing. Documentation
and diff checks pass. No Rust implementation or wire field changed; the reference
now describes this exact-string requirement. This closes a TM-01 schema/runtime
disagreement, not compiled backend admission or the remaining OEP-0003 work.

## Checkpoint 135: consistent whole-string tool schema validation

Extended the exact end-of-string correction across worker envelopes, grants,
archive layouts and receipts. Added 19 shared malformed identity/target fixtures;
the worker request UUID case reproduced schema acceptance before the fix. All
schema fixtures now pass, and 36 targeted Windows Rust tests pass across worker
exchange, grants, layouts and receipts (one direct child fixture is intentionally
ignored and exercised by its parent test). No Rust implementation changed.

The schema grammar still permits newlines in literal text where explicitly
allowed; this correction only prevents a final newline bypassing a whole-string
pattern. Contract documentation describes the boundary. Documentation and diff
checks pass. These tests establish schema/runtime agreement for the fixtures,
not backend admission, native supervision or full OEP-0003 completion.

## Checkpoint 136: complete root CI at immutable-request revision

[Run 37010483750](https://github.com/micahlmartin/oyzu/actions/runs/37010483750)
completed successfully at 1c2ffaf9236bd835bc3ae1ba764e1292bd501fc2. All 16 jobs
passed: native CLI builds and compiled CLI scenarios on Windows, macOS and Linux;
the Linux isolated BuildKit worker contract; captured Python, Docker, Rust, Go,
Java, Node, Helm and core build suites; and the combined captured-build acceptance
job. This supersedes the pending workflow state recorded in checkpoint 133.

These results cover the existing compiled CLI and synthetic-admission tool store
qualification at that revision. They do not cover subsequent schema commits,
the pending fork Python metadata change, production mise import, native tool-worker
supervision or the full OEP-0003 tool-management lifecycle. No product behavior
changed in this evidence update. Documentation and diff checks pass.

## Checkpoint 137: bounded fork Python precompiled metadata

Fork commit 23598b3db1df9f3f1f8e11c35a69a9bff1c5d7a2 reuses bounded HTTP
collection and caps gzip expansion on both Python precompiled catalog paths.
The qualification companion records exact limits, passing local regression,
Clippy/format/harness evidence, current catalog size observations and remaining
Python admission work. Native embedding run 37016853259 is active on all three
hosts. Compliance inventory checks and 24 guard tests (two Windows symlink skips)
pass; current-head human review remains required by the separate CI gate.

This is input-boundary work for TM-05/TM-06, not production Python tool selection,
installation or completion of OEP-0003. No root Rust dependency or behavior changed.
Documentation and diff checks pass.

## Checkpoint 138: Python catalog boundary passes native CI

Fork embedding run 37016853259 completed successfully on Windows, macOS and
Linux at 23598b3db1df9f3f1f8e11c35a69a9bff1c5d7a2. All three jobs passed
the Python gzip regression, formatting, strict library/example checks, existing
fresh-process embedding scenarios and real Go/Java metadata replay. This replaces
the pending native result in checkpoint 137 and the qualification companion.

The separate sensitive-change human-review gate remains unresolved. These results
do not approve distribution, establish Python selection/archive qualification or
complete production integration. No root implementation changed. Documentation
and diff checks pass; full OEP-0003 work remains active.

## Checkpoint 139: embedded Python locked-artifact substitution denial

Fork commit 4595732a494af3afc442945d671844210c41e627 now rejects upstream's
replacement artifact when an embedded explicit-target query supplied a locked
filename. The qualification companion documents its exact scope and passing local
regressions, Clippy, formatting and existing harness checks. Native embedding
run 37020409067 is pending. The ordinary upstream refresh behavior is preserved.

This is a frozen-identity safeguard for TM-03/TM-06, not a complete Python adapter
or worker implementation. Direct locked-URL acquisition, provenance and admission
remain separate requirements. No root Rust implementation changed. Documentation
and diff checks pass; OEP-0003 remains in progress.

## Checkpoint 140: retained Python catalog inputs

Added an independent Python capture helper that preserves exact compressed catalog
bytes and compressed/decoded hashes for the initial three targets. Four regressions
cover byte retention, both size limits, gzip damage/truncation, UTF-8, empty inputs
and transport failure. A live capture on 2026-10-02 produced a 37,004-byte external
fixture with SHA-256 529c646efb5101876adea94a7dc5cfb858a57c7fcbef109923a0224fbd6045ba.
Decoded sizes were 195,798 / 181,436 / 158,412 bytes for Linux/macOS/Windows.
The operational reference documents usage and limitations. This retains inputs;
it does not claim Rust backend replay, Python resolution or installation success.
OEP-0003 remains incomplete.

## Checkpoint 141: native locked Python catalog checks pass

Fork run 37020409067 completed successfully on Windows, macOS and Linux at
4595732a494af3afc442945d671844210c41e627. All three jobs passed the Python
catalog regressions and existing library/embedding checks. This supersedes the
pending result in checkpoint 139 and the qualification companion. It establishes
bounded catalog decoding and embedded locked-filename retention within the tested
paths, not Python resolution, worker integration or installation support.

The separate compliance workflow 37020409077 requires current-head human approval
from @micahlmartin. No approval is inferred from passing technical checks.
Documentation and diff checks pass; full OEP-0003 implementation remains active.

## Checkpoint 142: actual Python catalog decoder and selector replay

Fork revision 5da9db2287c3ab94783b752ec700c53a3020100d adds opt-in replay
of the independently captured Python catalogs. Linux passed the Rust test with
networking disabled: compressed/decoded hashes match, all three exact target
artifacts are retained, and removing each locked artifact rejects substitution.
Rust 1.95 formatting and strict library/example Clippy pass; compliance inventory
is consistent and 24 guard tests pass with two Windows symlink skips.

Native CI now captures using a revision/hash-pinned helper and retains fixtures
and reports. Those new native results remain pending. The maintenance reference
contains reproduction instructions and boundaries: this is private parser/selector
coverage, not HTTP transport, a Python resolver API, installation, or approval.
Documentation and diff checks pass; full OEP-0003 remains in progress.

## Checkpoint 143: Python catalog-only artifact facts

Fork c31fc1decd06313ec3f883e555722f9e9b5a1aaf adds the explicit-target
Session Python catalog API, reusing upstream selection without lock-resolution's
artifact acquisition/provenance effects. The maintenance reference defines its
inputs, outputs, transport, supported layouts and remaining obligations. Linux
passes real-catalog replay, formatting, strict library/example Clippy and all
25 embedding scenarios; three new cases exercise the public API and denials.
Compliance inventory remains consistent; 24 guard tests pass with two symlink
skips. Native API CI is pending. No production mise import is introduced.

This advances TM-03/TM-06 metadata handling, not Python version resolution,
artifact checksum/attestation verification, installation or distribution approval.
Documentation and diff checks pass. OEP-0003 remains in progress.

## Checkpoint 144: Python declared checksum metadata

Fork 36e1b5a54d81badd5eea17391c60568755e4b9c9 composes Python catalog
selection with bounded release SHA256SUMS acquisition through supplied transport.
The shared strict parser requires a unique checksum for the exact selected file;
the result preserves both catalog and checksum snapshot identities. Linux passes
formatting, strict library/example Clippy and all 28 harness scenarios, including
valid, duplicate and missing checksums across all three initial target tuples.
Compliance inventory remains consistent and 24 guard tests pass (two skips).
Native validation remains pending. The maintenance reference defines the API and
limits; documentation and diff checks pass.

No artifact bytes, publisher attestation or installation are verified by these
metadata scenarios. Version discovery, production worker/broker integration and
shipping approval remain outstanding. Full OEP-0003 remains in progress.

## Checkpoint 145: retained real Python checksum inputs

Extended the independent capture helper with optional --with-checksums. It retains
exact catalog and checksum bytes for a fixed Python 3.12.13 replay family, selects
the newest dated stripped artifact, and verifies unique checksum membership for
all three targets. Twenty-three maintenance tests pass, including duplicate,
missing, malformed, invalid-UTF-8 and oversized checksum failures.

A live 2026-10-02 capture retained a 201,388-byte external fixture with SHA-256
708c94ca489b5a248b178f658a171e6a7890657a05bffc8e224a6806bbcb44ec.
All selected artifacts were release 20260807 and had one matching checksum.
This establishes retained inputs, not public Rust API replay or artifact integrity.
The reference documents limits and reproduction. Documentation and diff checks
pass; full OEP-0003 remains active.

## Checkpoint 146: real Python metadata through public Session

Fork bb56ea99001b7b743738a564192b9ff9bebd1f4d replays the independently
retained Python catalogs and checksum bytes through Session::python_archive_metadata.
Linux passed all 28 ordinary scenarios plus the real three-target replay with
networking disabled. Formatting, strict library/example Clippy and compliance
inventory checks pass. Native CI uses the updated revision/hash-pinned capture
helper; these new native results remain pending. The earlier private decoder replay
run 37023404189 at 5da9db228 completed successfully on all three native hosts.

The reference includes reproduction and evidence boundaries. Metadata equality
is not artifact integrity, publisher authentication, installation or production
integration. Documentation and diff checks pass; OEP-0003 remains active.

## Checkpoint 147: Python constraint candidate native probe passes

Run 37027499477 at root 87973b366e67cd72f116cc2af84404a14d9e623b passed
the isolated retained parser probe, strict Clippy and formatting on Windows,
macOS and Linux with Rust 1.95.0. Candidate evidence and the qualification companion
record the exact revision and scope: 15 matching cases, six invalid specifiers,
and the retained locked experiment graph. This resolves the macOS probe gap.

No product dependency was added, license alternative selected or distribution
approved. Full constraint conformance, dependency obligations and resolver wiring
remain unfinished. Documentation and diff checks pass; OEP-0003 stays active.

## Checkpoint 148: native Python checksum API scenarios pass

Fork run 37025615534 completed successfully on Windows, macOS and Linux at
36e1b5a54d81badd5eea17391c60568755e4b9c9. This supersedes the pending
native checksum API result in checkpoint 144. The public reference now links the
completed run. All three hosts passed the ordinary harness, including valid,
duplicate and missing Python checksums, along with their other workflow checks.

The later public real-metadata replay at bb56ea990 is a separate run and remains
pending. These checks establish metadata behavior, not installation, publisher
verification, native worker supervision or distribution approval. Documentation
and diff checks pass; OEP-0003 remains active.

## Checkpoint 149: public Python metadata replay passes natively

Fork run 37026158376 completed successfully on Windows, macOS and Linux at
bb56ea99001b7b743738a564192b9ff9bebd1f4d. This supersedes the pending
public replay result in checkpoints 146/148. The workflow passed ordinary
embedding scenarios, private Python decoder replay and public Session replay
against independently captured real catalogs/checksum manifests on each host.
The reference now links the completed run and its precise scope.

This completes that metadata validation run, not full Python admission. Version
constraint resolution, acquired artifact/publisher verification, installation,
worker supervision, production wiring and shipping approval remain unfinished.
Documentation and diff checks pass; OEP-0003 remains active.

## Checkpoint 150: worker sessions bind trusted supervisor context

ToolWorkerSession construction now requires an independently expected operation
and envelope context. Exact request ID, context/backend digests, target and the
complete capability set must match before the session exists. Regression cases
mutate each field while retaining valid envelope syntax; all are rejected, including
capability additions and removals. Existing terminal and cancellation tests now
construct sessions against separate fixture expectations. The reference documents
the changed experimental constructor and its trust boundary.

Windows cargo test --locked, strict all-target Clippy, formatting, real CLI task
scenarios, documentation and diff checks pass. Ignored external-archive scenarios
were not run in this check. This identity binding does not authenticate channels,
admit typed payloads or implement process supervision. Native spawn, OS deadlines,
backend dispatch and full OEP-0003 integration remain unfinished.

## Checkpoint 151: native worker I/O interruption and confirmed joining

NativeToolWorkerIo now owns one pending frame per direction on dedicated threads,
reusing the existing framing budget and closed state. A supervisor can send cancel
while receiving. Shutdown closes admission, interrupts live native I/O and joins
both threads before reporting I/O cleanup complete. Unix uses socket shutdown;
Windows targets retained dedicated thread handles with CancelSynchronousIo until
completion is observed. An elapsed cleanup deadline retains ownership and denies
racing output; repeated shutdown is allowed. Drop interrupts and joins instead of
detaching threads, and can wait if the OS does not complete cancellation.

Six new tests pass on Windows GNU Rust 1.94 and Linux Rust 1.95 with network disabled:
blocked read/write with the peer kept open, duplex cancellation traffic, isolation
from an independent channel, racing-result discard and drop cleanup, invalid and
pending operations, and ownership retention after cleanup timeout. Windows full
cargo test --locked, strict all-target Clippy, formatting and nine real CLI task
scenarios pass. Ignored external-archive tests were not run locally for this change.
Documentation and diff checks pass. Reference, runtime and code-map pages describe
ownership, failure recovery and limitations; the existing windows-sys dependency
only gains its native I/O API feature, with no package/version or license change.

The full cross-platform run 37030050394 remains active at prior commit 99b6b68;
it does not test this increment. New native macOS validation is pending. Same-binary
spawn, image/channel authentication, restricted inheritance, process-tree cleanup,
deadline scheduling, typed payloads, backend dispatch and full OEP-0003 completion
remain outstanding. Successful I/O shutdown does not prove a worker process exited.

## Checkpoint 152: Windows worker job lifetime and descendant termination

The Windows worker lifecycle owner now creates an unnamed non-inheritable job
with kill-on-close and no breakaway permission. It permits one initial process
assignment, requiring the spawn adapter to create suspended and resume only after
successful assignment. A failed assignment consumes the attempt. Job termination
retains ownership and queries native accounting until no active processes remain;
errors/timeouts do not establish cleanup and permit retry on the same owner.
Closing the final job handle is a fallback request, not an observed-exit result.

Four lifecycle tests pass on Windows GNU Rust 1.94. Tests create this test binary
suspended, assign it, resume it and create a real descendant; attempted breakaway
is denied. Both known process handles become signaled and native accounting is
empty after termination, while an independent job stays alive. Last-handle-close,
failed assignment and empty-job sequencing are covered. Initial fixed-count
assertions failed because native job inspection identified additional conhost.exe
members; corrected assertions check known-process membership and require all job
members to exit. The ignored child fixture is explicitly run by these tests.

Windows full cargo test --locked (152 library tests passed, three fixture/native
opt-ins ignored at the top level), strict all-target Clippy, formatting and all
nine real CLI task scenarios pass. External-archive opt-ins were not rerun locally.
Documentation and diff checks pass. The existing windows-sys dependency gains job
and threading API features without a package/version or license-alternative change.
Reference, code map and OEP runtime describe the component and recovery obligations.

Separately, completed test/lint/format steps in native run 37031170986 at prior
commit 63e8073 pass on Windows, macOS and Linux. This confirms native I/O test
coverage on macOS, superseding checkpoint 151's pending observation. That workflow
is still active; it does not test the Windows job increment or establish all
captured-build/task results. Windows MSVC job verification remains pending.

Production same-binary creation, image validation, explicit control-handle lists,
combined process/I/O cleanup, deadline scheduling, Unix process lifecycle and
backend dispatch remain unfinished. This component is not a sandbox, shipping
approval, or completion of OEP-0003.

## Checkpoint 153: Windows MSVC worker job verification

Native run 37032992735 at d9c18de48bedcad8036ea9ac4434cb7c582ad53b now
reports successful locked unit/integration tests, strict all-target Clippy and
formatting on Windows MSVC. The normal library suite includes the four Windows
job tests added in checkpoint 152; their parent tests explicitly invoke the
ignored subprocess fixture. This supersedes the pending Windows MSVC observation
in checkpoint 152 and the corresponding reference paragraph.

Those same steps passed on Linux and macOS, where the Windows job module/tests
are excluded by cfg. No Unix process-lifecycle support is inferred. The workflow
is still running its remaining archive/release/build/task work; full workflow
success is not claimed. This checkpoint changes verification evidence only.
Native production spawn, image and channel admission, explicit inheritance,
combined process/I/O cleanup and the remaining OEP-0003 implementation stay open.
Documentation structure and diff checks pass.

## Checkpoint 154: Windows same-image spawn with restricted inheritance

WindowsToolWorkerProcess now pins the running image against an independently
supplied SHA-256 release identity, holding the image and canonical ancestors
against writes/deletion/replacement while owned. Native startup bounds and quotes
literal arguments, constructs a fully explicit environment with case-collision
rejection, and uses PROC_THREAD_ATTRIBUTE_HANDLE_LIST for at most 16 consumed
capability handles plus null standard streams. It creates hidden and suspended,
assigns the private job, closes the supplied parent handle copies and resumes.
Failed assignment/resume terminates and waits for the initial child. Explicit
termination confirms the empty job and initial-process exit, preserving its DWORD
exit code; I/O joining remains a separate required cleanup step.

Four new native process tests pass on Windows GNU Rust 1.94. They launch this test
binary through the real native primitive, exchange private-pipe frames, verify
literal arguments and an explicit environment without parent PATH, and confirm
access to a selected event but no access to an unrelated inheritable event. A
nested descendant retains the control pipes while both directions block; job
termination and I/O shutdown complete. Additional cases cover partial-frame child
exit with a high-bit DWORD, wrong-image rejection, failed-spawn handle closure,
invalid startup inputs and native Windows argument-parser round trips. The ignored
fixture is explicitly invoked by the parent tests and is not a product worker.

Windows full cargo test --locked, strict all-target Clippy, formatting and all
nine real CLI task scenarios pass. External-archive opt-ins were not run locally
for this increment. Documentation structure and diff checks pass. Reference,
code map and OEP runtime describe the native primitive and supersede earlier
statements that Windows creation/restricted inheritance were entirely absent.
No dependency, lockfile or notice change was needed. Native MSVC verification of
this increment remains pending; earlier CI does not cover it.

This is not release admission or completed tool execution. The supervisor must
supply independently admitted release identity, capabilities, arguments, environment
and private cwd. Coordination with concurrent broad-inheritance launchers, product
worker bootstrap and received-handle validation, automatic deadlines, backend
wiring, Unix creation and the complete supervisor remain unfinished. All OEP-0003
acceptance requirements remain tracked at their full scope.

## Checkpoint 155: image-pin lifetime and focused native worker CI

The Windows image pin now has its own private owner module beneath windows_process.
Production still selects only the running image; no alternate-image public API
was added. Two disposable-file tests verify denial of writes, deletion, image
rename and parent-directory replacement while a pin is held, then successful
replacement after release. Digest rejection also releases all acquired handles.
Using plain fixture files distinguishes this protection from the OS loader's
independent lock on a running executable.

Windows GNU Rust 1.94 full locked tests pass (158 library tests, four top-level
fixture/native opt-ins ignored). Strict Clippy initially rejected test-module
placement; after moving the tests to the end of the owner module, the two focused
tests, strict all-target Clippy, formatting and nine real CLI task scenarios pass.
Documentation and diff checks pass. External archive opt-ins were not rerun locally.
No dependency, notice, public constructor or wire-format changes were made.

A focused Tool worker lifecycle workflow now runs formatting, strict all-target
Clippy and the worker library tests on Windows, macOS and Linux using Rust 1.94.
Relevant pushes on main/the integration branch trigger it, and manual dispatch
is available. Its 10-minute test-step limit bounds hung regression runs. It does
not replace the full CLI/task/captured-build workflow or prove OEP acceptance.
The new workflow and new image tests await native CI results.

Separately, run 37036054434 at b68b983 reports successful locked unit/integration,
lint and formatting steps on all three native hosts, including Windows MSVC
process-creation tests. This supersedes checkpoint 154's pending MSVC observation
for that revision only. The broader workflow is still active. Product bootstrap,
release/capability admission, automatic deadlines, Unix spawning, backend wiring
and full OEP-0003 implementation remain unfinished.

## Checkpoint 156: exact request commitment for worker startup

Worker framing now optionally retains the original bounded JSON body alongside
its parsed value. The native asynchronous receiver exposes the same frame without
reserializing it, while retaining the existing single-collection lifecycle.
The exchange computes a domain-separated SHA-256 commitment, and the worker
session can reject any request-byte mismatch against a preselected expectation.
Tests reject payload substitution and equivalent JSON with changed spelling;
an independent Python hashlib vector checks the digest, and a native-channel
thread test checks exact-byte receipt and a correlated response.

The pending Windows GNU Rust 1.94 verification completed successfully: full
locked tests, strict all-target Clippy, formatting and nine real CLI task scenarios.
These checks do not establish full managed-tool execution. No dependencies,
license alternatives, notices, configuration fields or wire fields changed.

Focused native worker CI run 37037310454 passed on Windows, macOS and Linux at
10750d8, superseding checkpoint 155's pending observation for that revision.
The request-commitment changes still require their own native CI run. This is an
integrity foundation; trusted bootstrap, received-capability admission, typed
payloads, backend dispatch, Unix spawning and full OEP acceptance remain unfinished.

## Checkpoint 157: absolute native control-I/O deadlines

NativeToolWorkerIo can collect an already-started read or write under a supplied
absolute monotonic deadline. Expiry closes both directions and requests native
interruption; results observed after expiry are rejected even if the I/O already
completed. Ownership is retained for explicit shutdown and confirmed joining.
The supervisor still owns process termination, deadline selection, concurrent
cancellation and the cancellation grace period; this is not the complete event loop.

Windows GNU Rust 1.94 library tests passed (163 passed, four ignored), including
new live-peer incomplete-read/backpressured-write deadline tests and late-result
rejection. Full integration tests, strict Clippy and CLI scenarios are still
running at this checkpoint. Documentation structure and diff checks passed before
this status append. No dependencies, wire fields or license choices changed.
Native CI for the preceding request commitment at 010fd62 reports Windows and
macOS success; Linux remains in progress in run 37039123362. Those results do not
cover the new deadline waits. Full OEP-0003 implementation remains outstanding.

Checkpoint 157 verification completion: the full locked Rust suite, strict
all-target Clippy, formatting and all nine real CLI task scenarios subsequently
passed. Documentation and diff checks also passed. Run 37039123362 at 010fd62
has now completed successfully on all three native hosts; new deadline-wait
coverage still requires its own native run.

## Checkpoint 158: joint Windows process and control cleanup

WindowsToolWorkerLifecycle now owns an already-created worker and its matching
control I/O. Cleanup closes protocol access, interrupts pending I/O, attempts
job termination and attempts I/O joining even if process cleanup fails. One
absolute deadline covers both phases; a failure retains ownership for retry.
Explicit success confirms job-wide exit, the initial exit code and joined threads.
Drop interrupts I/O and releases process/job ownership before joining I/O; this
requests termination but does not report confirmed process cleanup.

The native descendant-held-pipe scenario now uses this lifecycle owner, and a
new test verifies drop cleanup with a running same-image process and live peer.
The initial Windows GNU full library run passed (164 passed, four ignored).
Integration checks and the follow-up expired-budget retry test remain pending
at this checkpoint. Documentation and diff checks passed. No new dependency,
license alternative, wire field or launch authorization was introduced.

The preceding absolute-deadline change at 17455e0 passed focused worker CI on
Windows, macOS and Linux in run 37039629532. This does not cover the new lifecycle
composition. Product bootstrap, typed payloads, actual backend dispatch, Unix
process supervision and full OEP-0003 acceptance remain unfinished.

Checkpoint 158 verification completion: the full locked Rust suite, strict
all-target Clippy, formatting and nine CLI task scenarios passed. After adding
the expired-budget retry assertion and renaming the drop test, the focused native
process suite passed (five tests, one subprocess fixture ignored) and formatting
passed again. Documentation and diff checks passed; new lifecycle CI is pending.

## Checkpoint 159: first real Oyzu install/exec user flow

The maintainer authorized the controlled development import of mise revision
9290bcac695c8ff8a56760ccebd785d5062b459c and reserved hardening for future goals.
The root Cargo manifest now pins that public Git dependency behind the opt-in
mise-integration feature; Cargo.lock contains the actual consumer resolution.
The upstream MIT notice is retained under third-party/mise. This is development
permission, not distribution approval or a completed transitive notice audit.

The feature-enabled Oyzu executable now exposes install and exec. It reads the
actual Oyzu configuration, invokes the linked mise library in a fresh same-image
child, uses the existing broker for public Node metadata/archive acquisition,
checks the declared checksum, stages and publishes through the existing store,
commits a format-2 oyzu.lock, and executes through a verified installation lease.
No separate mise executable participates. Initial support is standalone Node at
the workspace root; managed configuration is rejected rather than treated as
standalone. Internal child stdio and direct-child lifetime are development choices;
full worker hardening is deferred.

Linux amd64 Rust 1.95 build and actual user acceptance passed: Node 22.15.0 and
22.14.0 install into one shared store from two separate TOML projects, each exec
uses its locked version, literal arguments/cwd/configured environment pass, exit
code 7 propagates, locks remain unchanged during exec, and revisiting both projects
selects the correct version. The runner downloads and executes the actual tools.
Initial integration failures (reserved test environment name, counting tool-policy
settings, URL used as artifact ID and absent store directory) were corrected in
this same flow rather than replaced by component-only tests.

Windows GNU default-feature full locked tests, strict all-target Clippy,
formatting and nine existing CLI task scenarios passed with the new lockfile.
Feature-enabled Clippy remains in progress. Native Windows/macOS integrated
acceptance is pending; the new Tool management user acceptance workflow runs the
same user flow on all three hosts. Other tools, scoped/multi-tool update, offline
cached install, corporate transport, managed selection, npm commands, shims,
activation and builder handoff remain functional implementation work. See the
[development reference](reference/tool-management-development.md).

Checkpoint 159 verification completion: feature-enabled Linux Rust 1.95 strict
all-target Clippy and formatting passed. The newer lint required an equivalent
match-guard rewrite in the existing BuildKit image check; no executor behavior
changed. Documentation structure and diff checks passed. This is the first
connected Node user-flow evidence, not completion of the remaining functional
OEP scope. Hardening is reserved for future goals by explicit maintainer direction.

## Checkpoint 160: frozen install reuse and executable lookup

The Node development path now exposes `install --frozen` and `which node`.
Frozen install requires an existing matching lock; repeat and frozen installs
verify and reuse an existing installation before acquiring anything. Exec and
which share one frozen lookup and held installation lease. These changes advance
TM-03/07 and the MISE-01/03 user flows without claiming the full OEP complete.

The real Linux Rust 1.95 acceptance run passed Node 22.15.0 and 22.14.0 from two
Oyzu TOML projects, missing-lock frozen rejection, ordinary/frozen install reuse,
which/exec executable agreement, argument/cwd/environment/exit behavior, unchanged
locks and shared-store project switching. Running the store on a separate mounted
filesystem exposed cross-device publication failure; staging now lives inside the
store so atomic publication works. This was corrected and the full flow reran.

Windows GNU default locked tests, strict all-target Clippy, formatting and nine
existing task scenarios passed. Linux feature-enabled strict all-target Clippy
and formatting passed after the publication fix. Network-disabled replay of the
retained projects is in progress; native product CI remains pending. The pinned
fork's embedding run 37041298439 completed successfully on all three native hosts.
The development reference and owner map describe the new commands and limits.

Checkpoint 160 verification completion: the identical retained two-project flow
passed under Docker `--network none`, including both kinds of install reuse,
which/exec agreement and unchanged locks. This proves existing installation reuse,
not restoration from cached archives. Root CI run 37045134221 at 73bb33d completed
successfully on Linux; its original macOS real install/exec step passed while
remaining checks were still running. Windows remained in its build step. The new
frozen/which paths still need native Windows/macOS acceptance. Documentation and
diff checks passed. Hardening remains reserved for future goals.

## Checkpoint 161: restore locked Node tools offline from cached archives

`install --offline` now prohibits metadata/artifact acquisition and requires a
compatible lock plus a verified installation or its cached archive. Missing
installations are materialized and published from the store-owned verified blob
snapshot. Missing cache content produces an online-install remedy; corrupt cache
content remains an error. Frozen installation does not commit a lock edit.
This connects TM-03/04/07 offline functionality through real CLI commands.

The retained Linux acceptance projects selected Node 22.15.0 and 22.14.0. Their
installed trees were moved aside while preserving the actual downloaded archives.
Under Docker `--network none`, both restored with `install --frozen --offline`,
passed which/exec agreement, version, argument, cwd, environment and exit-status
checks, retained byte-identical locks, and switched correctly using one mounted
store. Offline install against an empty cache failed. Original trees remain in
the acceptance workspace backup. No mock tools or archive substitutes were used.

Windows GNU default locked tests, strict all-target Clippy, formatting and nine
real CLI task scenarios passed. Linux feature-enabled strict all-target Clippy
and formatting passed. The acceptance workflow now retains its projects and runs
cached restoration on all three native hosts. Native restoration is pending.
The earlier original install/exec flow at 73bb33d passed its real user scenario on
Linux, Windows and macOS in run 37045134221; this does not prove the later offline
features there. Detailed usage, restoration limits and measured platform evidence
are maintained in the development reference. Hardening remains deferred.

## Checkpoint 162: explicit Node version updates through the real CLI

`install --update`, `install --update node` and `install --update core:node`
resolve the configured Node requirement again, display previous/proposed exact
versions, install and then commit through the existing CAS lock transaction.
Ordinary install reports TOOL_LOCK_STALE with an update remedy for changed
requirements. Update conflicts with frozen/offline operation. This first path is
limited to one root/profile/host selection; unsupported existing environments and
platforms are refused so they are not silently dropped. Broader scoped and
multi-tool updates remain part of the active objective.

Linux real acceptance changed a project's TOML from Node 22.15.0 to 22.14.0,
verified ordinary install/exec rejection with the old lock preserved, updated and
executed 22.14.0, then restored 22.15.0 through a canonical-name update. The second
project's lock and actual execution remained unchanged. Bare update with unchanged
resolution preserved lock bytes. Unsupported update names and conflicting flags
failed without changing the lock. No fake executable or mocked resolver was used.
The native acceptance workflow now includes this update scenario before offline
restoration; Windows/macOS update evidence remains pending.

Default Windows GNU locked tests, strict all-target Clippy, formatting and nine
real task scenarios passed. Linux feature-enabled strict all-target Clippy and
formatting passed, including the final unsupported-target guard. Documentation
structure and diff checks passed. The preceding cached-restoration revision
01e81be passed its complete Linux CI job in run 37047681781; macOS restoration and
Windows build were still running when inspected. Hardening remains deferred.

## Checkpoint 163: shared environment inspection and mise shell rendering

`oyzu env` and `env --json` now inspect frozen installed selection with values
redacted. Explicit `env --shell bash|zsh|pwsh` uses mise's EnvDiff and shell
assignment renderer inside the fresh same-image child. Exec and env share one
owned environment composition function; all three consumers reuse frozen lookup.
An already leading binary directory is not prepended again. Exec rejects --json.
This advances TM-08's real shell integration without claiming prompt activation,
reversible state, automatic directory switching or active-shell retention.

Under Docker --network none, real Linux Bash and Zsh applied generated assignments
and executed installed Node 22.15.0. Their version, exact executable, literal TOML
values and environment matched direct Oyzu exec. Quotes, dollar expressions,
backticks and shell metacharacters remained literal; the sentinel command was not
executed. JSON inspection redacted values and lock bytes remained unchanged.
The runner restores its temporary TOML edit and requires each requested real shell.
CI now adds Bash on Unix, Zsh on macOS and PowerShell on Windows. Native macOS and
PowerShell environment acceptance is pending; no shell lifecycle completion claim.

Windows GNU default locked tests, strict all-target Clippy, formatting and nine
real CLI task scenarios passed. Linux feature-enabled build, strict all-target
Clippy and formatting passed. Documentation structure and diff checks passed.
Separately, the prior Windows revision 1acb16f passed installation, explicit
updates, offline restoration, feature lint and formatting in run 37048375166;
macOS 01e81be passed the complete cached-restoration job in run 37047681781.
These close prior native evidence gaps, not the remaining functionality scope.
Hardening remains reserved for future goals.

## Checkpoint 164: upstream-rendered automatic shell sessions

`activate bash|zsh|pwsh`, the internal hook-env entrypoint and `deactivate` now
connect upstream-generated hooks to Oyzu's frozen local selection. The renderer
uses a placeholder frontend while adapting generated names, then inserts the real
frontend through upstream quoting so path/config text is never namespace-rewritten.
Command-not-found automatic installation is disabled. No separate mise executable
is invoked and no shell profile is modified.

The new shell-session owner records changed values under a random temporary token.
Transitions roll back prior owned values, apply the current project's environment
and expose ready/unavailable status. Unchanged identity emits no delta; a missing
selection clears the preceding project and reports once per changed failure.
Scalar edits are preserved. Exact PATH restoration and removal of an unchanged
leading owned insertion are supported; general ambiguous PATH edits remain open.

Real Linux Bash and Zsh passed with Docker networking disabled: activation into
Node 22.15.0, automatic cd to 22.14.0, empty unchanged-hook output, missing-selection
cleanup, user scalar edit preservation, return to the first project and explicit
deactivation. The final cleanup replay also passed. The native workflow now runs
activation on Bash, Zsh and PowerShell; macOS and PowerShell results are pending.
Previous Windows environment application at 3eefcc2 passed its entire job in run
37049209850. This is initial TM-08 product behavior, not full lifecycle completion.

Default Windows GNU locked tests, strict all-target Clippy, formatting and nine
real task scenarios passed. Linux feature-enabled build, strict all-target Clippy
and formatting passed before a final relative-root preservation correction; its
final lint check is in progress. Detailed reference and owner map describe the
session and its limits. Shims, complete PATH alignment, independent/nested session
qualification, active-shell retention, profile installation and abandoned-session
recovery remain functional work. Hardening remains reserved for future goals.

Checkpoint 164 verification completion: final feature-enabled strict all-target
Clippy and formatting passed, including absolute capture of an explicit root.
Documentation structure and diff checks passed. Full OEP completion is not claimed.

## Checkpoint 165: native shims resolve each invocation's frozen selection

Install and activation now retain a versioned native frontend shim for Node,
using a hardlink where possible and an identical copy otherwise. The adjacent
manifest binds release digest, command and shared/current-directory store mode;
it never pins a project or Node version. Invocation is detected before ordinary
CLI parsing from the executable basename and validated manifest/image, then uses
the same frozen execution path and active session options. It never acquires a
missing tool or substitutes PATH. Windows uses node.exe, never a cmd wrapper.

Active sessions put their shim ahead of ambient tools and keep it there when the
current project is unavailable. Prompt hooks do not create shims. Deactivation
restores the prior PATH. Linux Bash and Zsh real acceptance passed under Docker
--network none: native-shim identity, literal arguments, exit status, project
version switching, unchanged hooks, missing-selection failure, edited scalar
preservation and cleanup. A separate direct probe made real Node 22.15.0 available
later on PATH and confirmed the shim still returned exit 2 for the missing lock.
The checked-in runner now includes that ambient-Node setup for native CI.

Windows GNU default locked tests, strict all-target Clippy, formatting and nine
real task scenarios passed. Linux feature-enabled build and final strict
all-target Clippy/formatting passed. Documentation structure and diff checks
passed. Windows/macOS shim evidence remains pending. Separately, original
PowerShell activation/switching/deactivation at 83049fa passed in run 37050580618.
Full shell lifecycle, shim pruning/command expansion, other tools and the rest of
OEP-0003 remain open; hardening is reserved for future goals.

## Checkpoint 166: real standalone Node proxy acquisition (2026-10-02)

Administrative tools routes now select explicit host TOML connector bindings.
The first-party acquisition adapter owns endpoint mapping and credential lookup;
mise metadata uses the same host broker as archive downloads through the existing
development worker channel. Project routes remain rejected by configuration.

The real Linux acceptance runner forwarded public Node index, checksum and archive
bytes through an authenticated loopback proxy, installed and executed Node 22.15.0,
rejected missing bindings and denied authorization without public fallback, and
reused the frozen installation offline without credentials or additional requests.
No proxy endpoint or credential appeared in the lock. This is a standalone host
binding proof, not managed grants, production proxy qualification or OS containment.

Default Windows tests, strict clippy, formatting and nine real task scenarios passed.
Feature-enabled Linux build, all-target strict clippy and formatting passed. Further
hardening remains deferred; managed acquisition and the remaining OEP functional
scope stay open. The detailed development reference owns usage and limitations.

## Checkpoint 167: explicit shell profile installation and removal (2026-10-02)

The opt-in CLI now implements shell install/remove for Bash, Zsh and PowerShell
with a required --profile-path. The profile owner edits only one marked block,
quotes the absolute frontend, preserves surrounding bytes and permissions, and
refuses modified or ambiguous blocks. It does not run the profile during editing.
Activation/session behavior continues to use the existing shell subsystem.

Real Linux Bash and Zsh acceptance installed Node into a fresh project, loaded
managed profiles, executed the locked Node via the native shim, and deactivated.
Repeated installation, user-edited block refusal and byte-exact removal passed.
PowerShell/macOS profile evidence remains pending; CI now runs those native flows.
The earlier Windows native-shim scenario at e335d70 passed in run 37051685318.

Default Windows locked tests, strict clippy, formatting and nine real task
scenarios passed. Linux feature-enabled build, strict all-target clippy and
formatting passed. Documentation structure and diff checks passed. Full OEP-0003
functional scope remains incomplete; additional hardening stays deferred.

## Checkpoint 168: explicit executable paths in development exec (2026-10-02)

Exec now accepts explicit absolute/relative executable paths in addition to the
selected Node command. Relative paths resolve against the invocation directory;
unknown bare names never use ambient PATH. The same frozen lookup, eligibility,
environment composition and installation lease apply. An explicit executable is
not represented as a locked tool and managed execution remains unavailable.

The Linux real-process acceptance ran with container networking disabled. An
explicit interpreter launched the locked Node through PATH, received literal
arguments, cwd and TOML environment, and propagated exit status. A relative Node
path resolved against -C; bare unknown and missing paths failed. Lock bytes were
unchanged. Native Windows/macOS explicit-path acceptance remains pending in CI.

Feature-enabled Linux build, all-target strict clippy and formatting passed.
The initial default Windows suite hit WAIT_TIMEOUT (258) in the unchanged
real_spawn_restricts_handles_environment_and_cleans_descendant_held_control_pipes
assertion. The full locked suite passed with --test-threads=2, including that test;
no lifecycle implementation or test was changed. Default strict clippy/formatting
also passed. This records the intermittent cleanup observation without claiming
it fixed; cleanup hardening remains deferred.

CI run 37051685318 at e335d70 now confirms macOS Node install/update/offline reuse,
Bash/Zsh environment and native-shim activation, as well as the previously recorded
Windows acceptance. These results do not cover the later proxy/profile/exec changes.

Nine real Windows task scenarios and documentation/diff checks also passed.

## Checkpoint 169: real Go installation and compiler execution (2026-10-02)

The maintained fork's existing Go catalog/version/archive APIs now feed the same
standalone install, lock, lease, exec, which, environment and shim flow as Node.
The backend adapter owns Go's GOROOT and default local toolchain mode. Acquisition
maps Go catalog and archive keys through the host broker, including explicit
administrative connector bindings; corporate Go proxy qualification is pending.
No upstream pin, executable or license choice changed.

Linux Go 1.24.13 was acquired from actual public metadata/artifact sources,
installed and reused frozen. Which/exec, GOROOT, local toolchain mode, compilation
and execution of a real standard-library program, literal arguments/TOML env and
Bash native shim activation passed with unchanged lock bytes. Official Go tar
entries required local PAX filename support; the same existing path validation
applies and unsupported/global fields remain rejected. Existing archive rejection
checks pass, including empty PAX records.

Node explicit-exec and the full Bash native-shim scenario still passed with
networking disabled. The Windows default locked suite (two test threads), strict
clippy, formatting and nine task scenarios passed; targeted archive checks passed
after the PAX change. Final feature-enabled Linux build/clippy/formatting passed.

The earlier profile CI run 37053758584 exposed a PowerShell fixture extension
error: .pwsh was classified as Application, while .ps1 is ExternalScript. The
runner now uses .ps1; native PowerShell profile acceptance remains pending and no
product success is inferred from the classification probe alone. Native Go tests
are now included in the existing three-host acceptance workflow.

The final Go replay moved installations aside and restored them from cached bytes
with container networking disabled. Which/exec, GOROOT, local toolchain and actual
compilation/execution passed again; lock bytes stayed unchanged. Documentation
and diff checks passed. Full OEP-0003 scope remains active, with hardening deferred.

## Checkpoint 170 - mixed Node/Go selection and selective updates (2026-10-02)

The product now installs both configured roots into one format-2 environment,
leases the complete selection for exec/which/env and shell hooks, and composes
both backend environments. Installation orchestration lives under
src/tools/development/installation.rs and uses the existing request projector,
lock transaction and store publication contracts. A selected update retains
unselected tool records exactly; a changed unselected requirement fails without
changing the lock. Single-tool JSON inspection retains its existing fields.

The complete Linux runner tooling/test-tool-mixed.py passed from a fresh project:
Node 22.15.0 with Go 1.24.13, Node-only update to 22.14.0, exact Go-record
preservation, stale/unselected-request rejection, frozen reuse, both executables,
Node launching the selected Go, redacted environment inspection and unchanged
frozen lock bytes. The initial run exposed an absent staging installs directory
when every payload was already installed; creating the required empty directory
fixed that actual integration failure, and the complete runner then passed.
Separate mixed replay and Bash activation passed with networking disabled, as
did the existing two-project Node offline regression flow.

Default Windows locked tests (two test threads), strict all-target Clippy,
formatting and nine real task scenarios passed. Final Linux feature-enabled
build, strict all-target Clippy and formatting passed. Documentation and diff
checks passed. Native mixed acceptance is added to the three-host workflow but
has not yet passed CI. Scoped/profile/multi-platform updates, other backends and
the remaining OEP functional requirements remain open. Hardening is deferred.

## Checkpoint 171 - physical project paths for default stores (2026-10-02)

CI run 37054415913 at efefc93 failed macOS profile acceptance during real install:
open blob store root returned ENOTDIR. The selected temporary project path had
an OS symlink ancestor, which was passed into the store's no-follow traversal.
The feature-enabled CLI now canonicalizes the selected directory before deriving
relative paths, including the default .oyzu/tools store. Explicit store contents
and profile-file symlink rules are unchanged.

The Unix profile runner now deliberately selects a directory symlink. Linux
Bash and Zsh passed the full real Node installation, profile activation and
execution, repeated installation, edited-block refusal and byte-exact removal.
Native macOS verification of this correction remains pending. The prior Windows
profile failure uses the already-corrected .pwsh fixture; its .ps1 replay remains
in the newer CI run and is not claimed successful here.

The Linux feature build, strict all-target Clippy and formatting passed. Default
Windows locked tests with two test threads, strict all-target Clippy, formatting
and all nine real task scenarios passed. Documentation and diff checks passed.
This fixes a demonstrated end-to-end usability failure; the full OEP scope
remains active and hardening remains deferred.

## Checkpoint 172 - registry names and Windows Go layout (2026-10-02)

Development configuration now uses the existing normalized request projector with
mise's pinned alias map before selecting backend adapters. Install and frozen
lookup share that projection. Canonical keys and short names produce the same
lock identity; selected updates accept registry names. Shared configuration
eligibility compares allowed and configured names through the supplied admitted
map, while other consumers retain literal behavior without a map.

The real Node runner test-tool-aliases.py passed with networking disabled:
canonical configuration reused the short-name lock and installed executable,
short/canonical administrative allowed values agreed, duplicate names and denied
tools failed, and lock bytes stayed unchanged. A separate online canonical-name
update passed. The first test setup incorrectly put admin-only tools.allowed in
project TOML; it was corrected to use an explicit disposable-host admin policy.
The three-host workflow now includes canonical-name update acceptance.

Windows Go installation in run 37056475928 failed because product layout used
Node's 200:1 expansion limit. The official Go 1.24.13 Windows archive, SHA-256
40b16bc8f00540a2cb02dff4de72b73e966fdd8d65f95e33d8e4080b48a2459a,
contains src/compress/flate/testdata/huffman-null-max.in at 799.207:1. The backend
now supplies the existing real Go ZIP qualification's 800:1 bound, retaining 200
for other admitted archives. Native product replay of the fix remains pending;
archive inspection alone is not claimed as installation success.

Feature-enabled Linux build, strict all-target Clippy and formatting passed.
The full default regression run is recorded after completion below. Full OEP
functional scope remains active; hardening stays deferred.

The final default Windows locked test suite passed with two test threads, followed
by strict all-target Clippy, formatting and all nine real task scenarios.
Documentation and diff checks passed. Native product CI remains pending.
