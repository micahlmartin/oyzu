# Tool identity inspection

`oyzu tools inspect-lock [PATH]` validates an experimental format-2 tool lock
without resolving, downloading, installing or running a tool. PATH defaults to
`oyzu.lock` relative to `-C`, then `--root`, then the current directory. An
explicit absolute file path is also supported. Output is JSON regardless of
`--json`; success exits 0 and validation/read errors exit 2 on stderr.

```sh
oyzu -C /path/to/project tools inspect-lock
oyzu tools inspect-lock tests/fixtures/tool-lock/valid.toml
```

The second example uses synthetic identity data, not an installable artifact.
The command requires a regular UTF-8 TOML file and reads at most 8 MiB plus one
byte to detect overflow. All tool-record readers verify the opened object is a
regular file; Unix uses nonblocking open so a FIFO is rejected without waiting
for a writer. Explicit symlinks to regular metadata files remain supported; this
is not the installation store's no-follow containment boundary. It does not read project configuration or mise files,
use ambient mise settings, make network requests, or modify the lock. Profiles
and other configuration options do not filter inspection: the whole document
is validated. Scope paths are checked lexically; filesystem containment and
current configuration compatibility are not assessed by this command.

Validation rejects unknown/duplicate fields, unsupported formats, malformed
digests, record-key mismatches, verification subjects that differ from artifact
digests, duplicate scope/profile pairs, missing dependencies, cycles, excessive
depth, unreachable tools, conflicting canonical IDs in a closure and missing
platform distributions. Different scopes may lock different versions. Limits
are 1,024 environments, 4,096 tools, 64 distributions per tool, 16,384 edges,
depth 64 and 256 roots per environment. Profiles use configuration identifier
syntax. Logical source/artifact identifiers currently reject colon and path
separators in addition to controls; they cannot carry URLs or filesystem paths.

Output includes format, environment/tool counts, the sorted platform inventory
and selection digests for each environment's common supported platforms. Each
digest binds the exact transitive platform closure, including artifact bytes,
verification metadata, layout and backend identities. Comments and set-like
array ordering do not affect it. Changing a dependency artifact changes the
selection digest even when its tool ID and version are unchanged.

Each selection also contains `installation_keys`, mapping tool keys to recursive
installation identities. A dependency change invalidates its own installation
and dependent installations. Adding an unselected platform leaves the existing
platform's installation identities unchanged. These keys name intended content;
they do not establish that an installation exists or is authorized.

`validation: "structure-and-identity-only"` is deliberate: success does not
prove publisher authenticity, legal clearance, policy authorization, installed
content integrity or backend admission. Request digests are checked for shape,
not recomputed against current configuration/native constraints. Backend option
allowlists, version canonicalization and package-closure admission require the
future pinned backend adapter. A self-consistent malicious lock can pass this
inspection; it is not an execution capability.

The current CLI does not yet install, activate, switch or execute tools through
mise. Those operations remain work under [OEP-0003](../proposals/OEP-0003-mise-integration/README.md).
No automatic migration of experimental format 1 is provided. For invalid input,
repair the source records or restore a known valid lock; do not change digests
merely to suppress validation failures. Inspection does not repair files.

Unit tests cover graph failures, bounds and semantic identity. The CLI test
checks an independently computed golden digest, ignores malformed adjacent mise
and Oyzu configuration, preserves file bytes and rejects tampering. Local results
and platform limits are recorded in [implementation status](../implementation-status.md).

## Payload tree observation

```sh
oyzu tools inspect-tree /absolute/path/to/payload
```

This reads an existing directory and emits its sorted manifest, content byte
count, domain-separated tree digest and canonical manifest blob digest. It never
executes payload files, follows payload symlinks, writes a receipt or modifies
the tree. As with lock inspection, output is JSON, success exits 0 and errors
exit 2. Relative paths are based on `-C`, `--root` or the current directory.
Use a physical path without symlink/reparse-point ancestors; redirected roots
are rejected rather than silently resolved. Parent `..` components are rejected.

File records include content digest/size and Unix executable bits. Directory
entries and relative symlink targets are included. Windows executable bits are
zero because Windows does not provide Unix mode bits. Timestamps, owner IDs,
ACLs and host inode/file IDs do not enter the portable manifest. The scanner
checks hardlink identities locally and rejects links to files outside the tree;
internal hardlinks produce explicit file records for each path.

The reader anchors Unix child access to directory descriptors with no-follow
opens. Windows holds all ancestor directories without delete sharing and rejects
reparse points when opening regular files/directories. Link chains are resolved
against the manifest component by component: escaping, cyclic, dangling or
non-directory chains fail. Portable member names reject Windows devices, ADS,
trailing-dot/space aliases, controls and case collisions on every host. Root
directory names may be host-native; payload member names must be portable UTF-8.

Limits are 200,000 entries, 8 GiB of file content, 1 GiB per file, path depth 64,
64 symlink expansions, 16 KiB link targets and a 32 MiB aggregate path/target
budget. Hashing streams through a 64 KiB buffer. Special files fail before reads;
observable file size, identity, link-count or executable-mode changes during
verification also fail. Limits cannot be raised by project input.

`validation: "payload-observation-only"` does not claim an atomic snapshot against
a malicious same-user writer. The scanner reads all bytes and uses no mtime-only
cache, but a trusted receipt comparison, healthy monitor or private managed
materialization is still required at the selection/execution boundary. Those
admitted-selection and private managed-execution operations remain unfinished.

## Archive materialization foundation

The Rust library's `tools::materialize_archive` accepts an archive source, an
empty operation-owned staging directory, the expected SHA-256 and exact byte
size, and whether the archive is gzip compressed. This is a store implementation
boundary, not a CLI install command or a stable external plugin interface.
It does not acquire bytes, authorize a backend, execute anything, write a receipt
or publish an installation. Failed staging must be discarded by the caller.

Source and staging paths must have physical, non-symlink ancestors. The source
path must therefore use the physical temporary directory on systems where a
temporary-directory ancestor is an alias (for example macOS `/var`). The source
must be a regular file; no-follow opens reject symlinks and special files before
reads. The source is copied through a bounded buffer into a private unnamed file while
checking its locked identity. Extraction reads that verified file, never a
reopened source path. Unix writes use descriptor-relative no-follow operations;
Windows holds ancestor directories against replacement and creates new files
without following reparse points. Existing staging content is rejected.

The initial implementation accepts regular files, directories and bounded GNU
long-name/link extensions in tar or gzip-compressed tar. Symlinks are delayed
until all file writes finish, then checked against the complete payload graph.
Windows symlink materialization, hardlinks, PAX, sparse entries, ZIP and other
formats remain unsupported and fail; backend admission must account for these
limits. Ownership, set-ID and archive directory permissions are not imported:
directories are private and traversable, files are private with Unix executable
bits retained. A reviewed layout plan is still required for final permissions.

Bounds include 200,000 raw and expanded entries, 8 GiB input/expanded bytes,
1 GiB per file, depth 64, 16 KiB GNU extension bodies and a 200:1 gzip expansion
ratio. The decoder is drained after tar's end marker so gzip trailers and trailing
expanded data are checked. Paths reject traversal, absolute names, case aliases,
Windows devices/ADS and conflicting parent types. The returned tree observation
still does not attest policy, receipt authenticity or an atomic snapshot against
a malicious same-user writer. This is partial TM-04/MISE-13 evidence, not a
complete installation or backend acceptance result.

## Receipt/content verification foundation

The Rust library's `tools::verify_installation_selection` reads a format-2 lock,
an exact scope/profile/platform, a tool store root and a caller-trusted installer
release digest. It requires `installs/<installation-key-hex>/receipt.json` and
`payload/` for every tool in the selected closure. It returns the selection digest
only after all records and payloads match. It does not download, repair, execute,
publish or acquire leases, and is not an execution authorization API.

Receipts use the exact fields specified by OEP-0003, with format 1 and explicit
null `package_closure_digest` when absent. Unknown fields and duplicate JSON keys
fail. Every locked field, publisher-verification record, sorted direct dependency
installation-key list and installer release identity must match. Every payload
is rehashed through no-follow handles; no mtime cache or upstream installed-version
status can satisfy this operation. The current composition budget is one million
tree entries and 32 GiB of payload content across the selection, with the existing
per-tree limits also applied. Receipt JSON is capped at 2 MiB and uses the existing
strict JSON depth/entry limits. Larger tuples require reviewed limits rather than
project-controlled overrides.

The initial resolved launch records are closed tagged objects:

- Native: `kind: "native"`, `payload_relative_path`, ordered `prefix_args`.
- Interpreter: `kind: "interpreter"`, `payload_relative_path`, `interpreter`, ordered `prefix_args`.
- Installation path: `installation_key`, `relative_path`.
- Argument: `kind: "literal"` with `value`, or `kind: "path"` with an installation `path`.
- Environment value: `kind: "literal"` with `value`, or `kind: "paths"` with ordered installation `paths`.

Entrypoints must resolve to existing files; interpreters must also be executable
on Unix targets. Directory environment references and file/directory arguments
may use the exact current or direct-dependency installation only. `.` denotes
that installation's payload root for directory references. Symlinks resolve
through the already-verified manifest, not through unchecked filesystem opens.
Command/environment case collisions, invalid environment names, NUL literals,
excessive arguments and literal PATH replacement fail. Literal arguments remain
data; no shell is invoked.

The draft [format-1 receipt schema](../contracts/tools-v1/receipt.schema.json)
defines the closed JSON shape, tagged launch/environment records, required
nullable closure field and structural bounds. Validate it offline with
`python tooling/check-tool-contracts.py` after installing the pinned
`tooling/design-requirements.txt` in a validation environment. The same command
checks layout records. It uses only local schema references and changes no store
content. The shared receipt fixture is synthetic; it is rebound to real fixture
payload hashes by the Rust receipt tests, never executed as an installed tool.
Both validators reject the shared malformed-record corpus. JSON Schema measures
character lengths, whereas runtime applies UTF-8 byte limits and additionally
checks portable paths, case collisions, sorted dependencies, exact lock identity,
payload hashes, file types and executable permissions. Schema success alone does
not establish receipt validity or authority.

This verifies receipt/content binding, not receipt authenticity or compatibility
with an admitted layout plan. A caller must separately check backend/layout
admission, verification evidence, policy, leases and current authority before
using the data. A self-consistent altered receipt and tree is not an authorized
installation. The initial publication/OS-lease operation is described below;
admission, lifecycle integration and recovery remain separate unfinished work.

## Publication and OS lease foundation

`tools::lease_installation_selection` accepts the same exact selection and
installer identity plus optional staging. The caller must provision trusted,
private, physical store/staging roots and complete policy and backend/layout
admission first. The library does not choose an application state root, grant
authorization or turn arbitrary supplied receipts into approved installations.
Staging uses `installs/<installation-key-hex>/{receipt.json,payload/}`. Extra root
entries, redirected objects and hardlinked receipts fail.

Mutation locks live permanently under `locks/<key-hex>` and are acquired in
lexical order. All selected existing/candidate receipts and payloads are checked
before any new member is published. Regular files and receipts are flushed;
Unix also syncs directories. Linux uses `renameat2(RENAME_NOREPLACE)`, macOS uses
`renameatx_np(RENAME_EXCL)`, and Windows uses a write-through `MoveFileExW` without
replacement or cross-volume copying. Failure never deletes an existing install.
Windows directory fsync is unavailable; missing/damaged state after a crash must
still fail subsequent verification. macOS execution of this new native path
remains pending qualification.

Publication is atomic per installation, not across the whole closure. An error
after an earlier member committed may leave valid unreferenced entries; the
source lock is never edited. Failed or unused staging is retained for explicit
recovery. Existing valid entries are reused, while corrupt entries fail without
automatic replacement, quarantine or public download fallback.

Shared OS lease locks under `locks/<key-hex>.lease` are acquired before mutation
locks are released. The returned `InstallationLease` must remain alive until the
whole consumer action ends. Both lock kinds use a bounded 30-second contention
deadline. Lock files are never removed/replaced during ordinary operation.
Unix lock initialization first attempts exclusive no-follow creation. If the
entry already exists, it opens that same name without creation or truncation,
then requires a regular single-link inode. This preserves permanent lock identity
under concurrent initialization. Recovery's existing-only opens never create
missing lock evidence.
Prune must acquire mutation first, then try an exclusive lease; this protocol
has not yet been connected to a prune command. Process exit releases kernel locks.

Before returning a lease, the store atomically publishes a flushed format-1
`leases/<lease-id>.json` record while holding its mutation and shared lease locks.
The closed writer records `format`, `lease_id`, `owner_pid`,
`created_unix_nanos` (decimal string), `selection_digest` and sorted unique
`installation_keys`. `InstallationLease::lease_id()` exposes the diagnostic ID.
IDs use PID, timestamp and a process-local counter, with exclusive creation and
no-replace publication; they are not secrets or authorization tokens. Records
contain no workspace paths, environment values or credentials.

Ordinary lease destruction removes its own record before explicitly unlocking its
journal guard and installation leases. Explicit unlock prevents a transient
fork-inherited descriptor from extending the completed supervisor lifetime;
closing the handles remains the fallback if unlock fails. Callers must still retain
the lease until the entire supervised child tree ends.
Cleanup errors conservatively leave a stale record; destruction cannot report
them to the caller. Forced termination can also leave a final record, and a crash
during writing can leave a `.pending` file. Neither PID/timestamp nor a record's
presence establishes liveness: recovery must use OS locks and the relevant
reference/owner checks, never PID alone. Each new lease holds an exclusive
`locks/lease-<lease-id>` journal guard until destruction, including empty records.
These guard files remain permanently, like the installation lock files.
A journal creation/publication failure rejects selection and releases locks;
already committed installations remain valid but unreferenced. Unknown or
redirected lease directories fail without following links or replacing content.
The parent and file flush behavior has the same platform limits as publication.

`tools::recover_tool_leases(store, dry_run)` explicitly reaps final process-lease
records only. It is a library operation; no automatic startup sweep or CLI prune
command is enabled. The root must be a trusted physical store. It opens existing
directories and lock files without creating missing lock evidence. It requires a
canonical record ID, matching closed format-1 data, valid digests, sorted unique
keys and a regular single-link record. It then tries the journal guard, sorted
mutation locks and exclusive installation leases without waiting. Busy locks
retain the record. Older records lacking a guard, unknown names, `.pending` files,
malformed data, redirected files and unverified entries remain for manual review.
No PID polling, installation deletion, lock-file deletion or recursive deletion
occurs. This is a cooperative store contract, not protection against same-user
malicious writers.

The returned counts are `stale`, `removed`, `active_or_busy` and
`unverified_or_failed`, plus `dry_run`. Dry-run performs the same transient lock
checks but removes nothing. More than 4,096 directory entries fails before
processing; record reads are capped at 512 KiB each and 32 MiB aggregate, with an
extra byte probe for overflow. Unverified or failed records increment the last
counter without exposing record text. The sweep is not one transaction: an I/O
failure may follow earlier removals, and a failed directory sync leaves durability
uncertain even if unlink succeeded. Reinspect and retry after correcting storage
errors; never infer an active process solely from that counter or a leftover file.

This is a cooperative-store foundation. Persistent shell-session references,
operation owner/start-time journals, workspace reference tracking, staging/crash recovery,
quarantine, prune, shell-session retention and supervised child-tree lifetime
integration remain outstanding. A lease alone neither prevents same-user file
tampering nor kills descendants when a supervisor dies. The complete MISE-05
fault-injection and power-loss matrix is not yet satisfied.

## Verified blob cache and extraction handoff

`tools::cache_tool_blob(store, source, digest, size)` takes a caller-authorized
reader and exact SHA-256 identity. The existing store root must be private,
physical and trusted. Blobs are stored under `blobs/sha256/<digest-hex>`;
permanent `locks/blob-<digest-hex>` files serialize publishers with a 30-second
contention deadline. A cache hit is fully rehashed into a private temporary
snapshot and never reads the acquisition source. A corrupt or externally
hardlinked cache entry fails without replacement or reacquisition.

On a miss, the stream is copied and hashed using a 64 KiB buffer, with a maximum
size of 8 GiB. At most the expected size plus one byte is consumed; truncation,
excess, wrong digest and reader failure publish nothing. Interrupted reads retry.
The caller must implement transport timeouts and cancellation: an arbitrary
blocking `Read` cannot be interrupted by the store. The store neither chooses a
source nor performs HTTP, credentials, signature verification or policy decisions.

Only fully verified bytes are copied to an exclusively created temporary under
`staging/`, flushed and atomically moved without replacement to the blob path.
The same native publication primitives and durability limitations described above
apply. A handled publication error removes only this operation's temporary file;
a destination collision is never overwritten. Process death before rename may
leave unselected staging. Recovery journals, quarantine and reclamation remain
unfinished. Temporary names are collision-resistant identifiers, not credentials.

The returned `VerifiedBlob` exposes read/seek access and digest/size, with no write
or raw-handle API. Its bytes are an independent private snapshot: subsequent cache
mutation does not change the returned content. `tools::materialize_tool_blob`
consumes that snapshot into an empty staging payload without reopening the cache.
It applies the same tar/gzip limits and graph checks as `materialize_archive`,
which now uses the shared snapshot verifier. Extraction and final observation
retain the same native directory handle. This proves byte identity and safe
materialization only; backend/layout admission, publisher verification, receipts
and execution authority remain separate requirements.

## Data-only candidate finalization

`tools::stage_tool_candidate` consumes a `VerifiedBlob`, bounded format-1 layout
JSON and `ToolCandidateRequest`. The request identifies the exact lock selection,
tool key, trusted installer identity, empty operation-owned candidate directory
and an admitted layout digest obtained from compiled release data. Project input
must never supply the admission digest. There is no production descriptor registry
or automatic backend admission yet; this library operation cannot grant them.

Layout identity is domain `oyzu.archive-layout.v1` over the complete canonical
plan. It must match both the supplied admission digest and locked distribution.
Backend digest, target platform, single input blob digest and size must also match;
package-closure distributions are not admitted by this archive-only finalizer.
Unknown/duplicate JSON fields, omitted nullable fields and unsupported formats fail.
The initial [JSON Schema](../contracts/tools-v1/archive-layout.schema.json) records
the closed field names. Rust adds portable path, byte, graph, identity and content
checks that JSON Schema cannot establish.

Currently supported plans use `tar`, `tar.gz`, the ZIP subset described below, or
raw single-file artifacts. Archives allow an optional `strip_prefix`; raw plans
require null. All require `payload_subtree: "."`. Other archive kinds and subtree
projection fail explicitly.
On Unix, `executable_paths` may contain at most 4096 sorted, unique portable
payload-relative file paths. Each must resolve through ordinary directories to
a regular file with one hardlink; symlinks are rejected at every component. The
finalizer sets exactly private mode `0700` through the opened handle, flushes it,
and recomputes the payload tree before authoring the receipt. It never executes
the file. Windows requires an empty list because Unix permissions are not a
Windows launch contract. A transform failure leaves uncommitted staging without
a receipt; discard that staging and retry in a new empty directory. Entries outside the
strip prefix are rejected, including a prefix ancestor that is not an ordinary
empty directory. Strip-prefix removal does not rewrite symlink targets; the final
contained link graph must remain valid. Required paths are a sorted unique list
of exact payload paths and `file`, `directory` or `symlink` types.

`extraction_bounds` has positive integer `max_entries`, `max_bytes`,
`max_file_bytes`, `max_depth` and `max_expansion_ratio` fields. Entry, byte and depth
ceilings remain fixed. The default expansion ratio stays 200:1; an explicitly
admitted layout can request up to the hard ceiling of 1024:1, as allowed by the
OEP's descriptor-specific bounds. This ratio is part of the hashed layout identity,
not a project option or a value inferred from an archive. Production descriptors
need review before admitting larger bounds. Existing default-200 plans are unchanged;
older binaries reject layouts above their supported ceiling. `max_bytes` caps both total
payload bytes and the expanded archive stream (including archive metadata).
All runtime fixtures exercising smaller bounds must still fail before a receipt
is written. The schema permits the broader proposed archive vocabulary; schema
validity does not mean a tuple is currently implemented or admitted.

Entrypoint templates have `kind`, `payload_relative_path`, explicit nullable
`interpreter_tool_key` / `interpreter_relative_path`, and ordered `prefix_args`.
Native entrypoints require null interpreter fields. Interpreter entrypoints use
`self` or an exact direct dependency tool key, avoiding a circular self identity.
Path arguments and environment path lists use `owner` (`self` or an exact direct
dependency tool key) plus `relative_path`. Typed literal strings are never
evaluated. Resolution substitutes installation keys, never absolute paths.
Names, collisions, literals, declared dependency references, own payload path
types and own executable permissions are checked before receipt creation.

Success leaves only `payload/` and canonical `receipt.json` in the candidate,
and returns its installation key. It never edits `oyzu.lock`, publishes, performs
network operations or invokes tool code. Whole-selection publication then checks
all dependency references, receipts and current payload bytes before committing.
Failure can leave an incomplete candidate; callers must not adopt it. Receipt
creation does not itself verify publisher signatures or attestations: callers
must establish that evidence before invoking this boundary. Recovery, production
worker wiring, compiled descriptors and native backend parity are still required.

## Effective tool request identity

The experimental Rust `project_tool_requests` boundary consumes an already
captured `EffectiveConfig`, a caller-trusted unambiguous alias map, builder-owned
native constraints and required capability IDs. The caller must obtain that map
from an admitted catalog; project settings cannot add aliases or rebind canonical
IDs. The implementation does not yet load the maintained fork's catalog.

Effective `tools.*` strings become canonical requests; reserved `tools.allowed`
and `tools.catalogs` are policy/catalog inputs, not version requests. Unknown
aliases and multiple configured names for one canonical tool fail. Canonical
IDs present in the catalog may be requested directly. Expressions are preserved
verbatim for the backend; this function does not parse semver, pick versions,
intersect constraints or admit an installation. Effective profile overrides have
already been applied by the configuration resolver.

The returned identity hashes the complete object `requests`, `native_constraints`
and `required_capabilities` in domain `oyzu.tool-requests.v2`. Constraint arrays
and capability arrays are sorted; duplicates fail. Environment values, secrets,
origins and policy revisions are excluded. Policy must still be enforced for
every use even when this identity stays unchanged. The golden fixture for only
`core:node = "22"` with empty constraints/capabilities is independently computed
and checked against Rust.

Limits are 4096 alias entries, 256 root requests, 4096 constrained tool IDs,
16384 total native constraints, 256 capabilities, 16 KiB per text value and an
8 MiB aggregate text budget. Capabilities are at most 256 ASCII letters, digits
or `-_.:/`. Invalid/oversized values fail before cloning the bounded collections.
Nothing is read from disk, downloaded, executed or modified by projection.

After projection, prefer `select_for_tool_requests(workspace, directory, profile,
requests, platform)`: it checks both the computed digest and canonical request
map against one captured lock. A lock with a matching digest but an altered
request map fails `TOOL_LOCK_STALE`. Alias catalog loading, native builder
constraint production, backend resolution and worker wiring remain unfinished.

## Frozen environment selection

The experimental Rust `select_locked_environment(workspace, directory, profile,
request_digest, platform)` boundary chooses from `workspace/oyzu.lock` after the
caller has resolved effective configuration and computed its request digest.
It does not read Oyzu or mise configuration itself. Workspace and working
directory must exist as directories; both are resolved physically and an outside
working directory fails. Internal directory aliases select the physical scope.
Locked scope aliases are resolved physically too; two matching scope records for
the same physical ancestor fail `TOOL_LOCK_AMBIGUOUS`. Existing locked scope
directories for the selected profile must stay inside the workspace. Missing
unrelated scopes do not prevent using a present scope.

Among locked environments for the exact profile, the longest complete ancestor
scope wins. `app` does not match `application`. A nearer scope with a different
request digest fails `TOOL_LOCK_STALE`, even if the root entry would match.
No matching scope/profile fails `TOOL_LOCK_MISSING`; no complete selection for
the requested platform fails `TOOL_PLATFORM_UNAVAILABLE`. These failures require
an explicit resolution/update action, not a fallback to another profile, parent
or platform. The whole lock is validated before selection, including unrelated
dependency graph errors. Nothing is installed, downloaded, executed or edited.

Success returns the existing `LockedSelection` identity and installation keys.
The caller must still check current policy, compiled backend admission and
installed receipts and retain leases before use. This boundary is not a stable
new CLI command, a resolver, an authorization grant or race-proof filesystem
containment. This lower-level entrypoint checks the supplied digest only; use
`select_for_tool_requests` to also bind the projected canonical request map.
Worker integration remains pending.

## Explicit lock-edit transaction

The experimental Rust API `tools::ToolLockEdit` supports the publication part of
an explicit format-2 update. It does not resolve versions, authorize sources,
install tools or enable an `oyzu lock` CLI command. The caller supplies a complete
resolved candidate after applying its update scope and policy; records absent
from that candidate are deliberate removals. Existing frozen selectors never
call this API. No platform account or network is used.

```rust,ignore
let edit = oyzu::tools::ToolLockEdit::capture(lock_path)?;
let proposal = edit.propose(&resolved_format_2_bytes)?;
// Review proposal.changes() and proposal.preview(). For combined install,
// complete verification and installation before committing this proposal.
proposal.commit()?;
```

Capture accepts an existing parent directory and an absent or valid format-2
regular file. Existing source and candidate are independently bounded to 8 MiB
and validated by the same whole-graph parser used by inspection. Format 1 is
rejected; this API does not implement migration. File symlinks/reparse points and
nonregular inputs are rejected. Parent paths are canonicalized at capture; this
is a workspace edit boundary, not the store's hostile-filesystem containment API.

`changes()` returns ordered environment `(scope, profile)` or tool-key records
tagged `added`, `changed` or `removed`. Formatting/order-only differences are
excluded. `preview()` returns the exact proposed output bytes. Unchanged records
retain their TOML spelling and comments, including inline-array records. Changed
records use candidate formatting; editing may normalize line endings. A semantic
no-op retains the complete original bytes. The editor validates the final output
and compares its normalized graph with the candidate before returning a proposal.

Commit uses a permanent sibling `.oyzu-tool-lock-edit.lock` for cooperating
writers and compares the current file with the captured bytes both before staging
and immediately before publication. Missing-file creation uses no-replace
publication. Updates use a flushed same-directory temporary file and atomic
replacement, preserving existing permissions; Unix also syncs the parent
directory. Windows sharing/lock violations, including replacement access denial,
are retried for at most two seconds, with a fresh source comparison on each retry.
Read-only Windows sources fail before staging. Temporary files are cleaned up on
ordinary failure; the permanent coordination file remains and must not be deleted
while writers may be active.

An observed source change, active writer or failed replacement reports
`TOOL_LOCK_EDIT_CONFLICT`. Capture a fresh source and regenerate/review the
proposal to retry; never force a stale proposal over external changes. Invalid
graphs fail before publication. A directory-sync error after replacement explicitly
reports that the lock was published; inspect the current file before retrying.
Dropping an uncommitted proposal does not write the lock. This is optimistic
concurrency: a noncooperating process can still race the last comparison, and
replacement of the parent directory is outside this boundary. It is not a
filesystem compare-and-swap primitive or an authorization grant.

`tests/tool_lock_edit.rs` covers actual temporary-workspace publication, retained
comments, no-op byte identity, empty-lock transitions, semantic diffs, stale edits,
active-writer exclusion, invalid graphs and format rejection. Unix tests cover
symlink/FIFO denial; Windows exercises bounded sharing-denial recovery and
read-only-source cleanup. Complete
resolver/update orchestration, migration, install-before-lock lifecycle and native
CI qualification remain separate OEP requirements.

## Backend descriptor contract fixtures

The draft [backend descriptor schema](../contracts/tools-v1/backend-descriptor.schema.json)
encodes the OEP's closed identity record: exact source commit, patch-register
digest, embedding ABI and adapter revision, nullable registry/plugin digests,
verifier digest and unique string features. Nullable fields are required even
when null. Positive revision integers stay within the canonical JSON safe range.
The synthetic fixtures are not compiled backend descriptors or approved pins.

Run `python tooling/check-tool-contracts.py` with
`tooling/design-requirements.txt` installed to check descriptor, layout and receipt
shapes offline. The checker rejects duplicate JSON keys and nonlocal schema
references. Descriptor fixtures cover four valid shapes and thirteen rejected
shapes, including floating refs, omitted identities, open records and unsafe
integers. Schema checks do not establish sorted feature order, provenance,
canonical descriptor hashing, compatibility or admission. The Rust parser checks the shared fixtures and additionally requires features to
be sorted and unique. No stable project configuration syntax is introduced.

`oyzu tools inspect-backend PATH` reads a regular JSON file, bounded to 2 MiB
plus one overflow probe. Relative paths use `-C`, then `--root`, then the current
directory. It emits JSON with `digest`, `source_pin` and
`validation: "structure-and-identity-only"`, regardless of `--json`. Success exits
0; malformed, oversized or unreadable input exits 2. It rejects unknown/duplicate
fields, omitted nullable identities, malformed digests, floating source refs and
unsafe revisions. It never reads project/mise configuration, accesses the network,
executes backend code or changes files. Repair input explicitly and rerun on error.

```sh
oyzu tools inspect-backend tests/fixtures/tool-backend/descriptor.json
```

The example is synthetic and cannot authorize installation. The digest uses
`oyzu.backend.v1`, a zero byte and canonical JSON through the shared record encoder.
A checked-in golden identity and per-field mutation tests verify identity binding;
formatting differences do not change it. Compiled descriptor admission, provenance
validation, source licensing approval and compatibility remain separate unimplemented
gates. A self-consistent malicious descriptor can pass inspection.

### ZIP candidate materialization

`stage_tool_candidate` accepts `archive_kind: "zip"` for bounded ZIP32 archives
using stored or DEFLATE compression. The standalone tar materialization API is
unchanged. This is a library staging capability, not a production installation
command or backend qualification. It uses the same verified blob, admitted plan,
path checks, required payload checks and receipt boundary as tar layouts.

Inspection of the maintained fork's `zip` 8.6.0 dependency
found that its high-level archive reader builds a filename-indexed map that
collapses duplicate names, after allocating central-directory metadata. Checking
only `ZipArchive::len()` and then applying Oyzu path checks would therefore miss
duplicate records and apply resource limits too late. Its streaming visitor also
ends central-directory parsing on an error, so successful visitation alone does
not establish a valid complete directory.

Oyzu therefore preflights the complete central directory before initializing the
decoder. Entry counts, total local/central metadata (32 MiB), individual and total
expanded sizes, and per-file expansion ratios are bounded. The expanded archive
budget includes local headers, central metadata and the footer/comment as well as
payload. Names must be ASCII or explicitly flagged UTF-8. Duplicate raw names,
inconsistent local/central names, sizes, methods and CRCs, overlapping local
records, unconsumed gaps and trailing data fail. Both signed and unsigned ZIP32
data descriptors are checked. Decoder output must reach EOF with the declared
size and valid CRC; a corrupt payload never receives a receipt.

Only ordinary files and empty directories from DOS/Unix originating systems are
admitted. Unix executable bits are preserved in otherwise private file modes;
Windows launch validation still uses receipt rules. ZIP64, encryption, split
archives, self-extracting stubs, Unicode override extra fields, links and special
entries are rejected. Archives requiring these features need further qualified
implementation; they are never silently interpreted as another format. On any
failure, discard the incomplete staging directory and retry with an admitted
artifact in new empty staging. Extraction has no network access or subprocesses.

The exact `zip` 8.6.0 dependency uses only `deflate-flate2`, with default features
disabled. Original notices for it and its new transitive dependency `typed-path`
are preserved in [third-party notices](../../third-party/README.md). This does not
approve the complete shipping dependency graph or establish publisher authenticity.
Tests cover stored/DEFLATE staging and receipt publication, CRC corruption,
header disagreement, duplicate metadata, limits, traversal, case collisions and
both data-descriptor encodings. Real upstream archive parity and native platform
qualification remain separate gates.

Archive path admission is centralized under `store/archive/paths`: raw duplicate
detection, strip-prefix validation, case/type collision checks, expanded entry and
depth limits, and anchored destination traversal use one rule set. The 32 MiB
name budget now includes original names retained for duplicate detection, even
when stripping skips a prefix ancestor, as well as expanded destination names
and link targets. Archives that previously escaped this accounting can fail
earlier; no receipt is written on failure. TAR and ZIP use this same path owner.

### Real Node archive store qualification

The opt-in `tool_native_archive` integration test uses the real Node 22.14.0
Windows x64 ZIP. This fixed historical release is a reproducible fixture, not a
recommendation for project versions. Provision its archive outside the checkout
from [the official release](https://nodejs.org/dist/v22.14.0/). The preparation
script checks the pinned size and SHA-256 from that release's HTTPS checksum
list, then independently inventories every file through Python's ZIP decoder.
It makes no network calls and uses exclusive creation for its output manifest.

```sh
python tooling/prepare-node-store-fixture.py --archive /fixtures/node-v22.14.0-win-x64.zip --manifest /fixtures/node-store-manifest.json
export OYZU_NODE_STORE_ARCHIVE=/fixtures/node-v22.14.0-win-x64.zip
export OYZU_NODE_STORE_MANIFEST=/fixtures/node-store-manifest.json
cargo test --locked --test tool_native_archive real_node_zip_publication_parity_and_changed_lock_denial -- --ignored --exact --nocapture
```

In PowerShell, set those two variables with `$env:NAME = 'absolute path'`.
Python 3.11+ and the normal Rust prerequisites are required. Missing inputs,
wrong digests, archive discrepancies or a preexisting manifest fail explicitly;
choose a new manifest path to repeat preparation. Normal `cargo test` lists
the test as ignored rather than treating missing fixtures as a pass. CI explicitly
provisions the pinned archive and runs the test on Windows, macOS and Linux.

The test compares all 3016 stripped entries, file lengths and hashes with the
independent inventory, publishes a locked receipt and revalidates committed
content. Changing only the locked archive digest must deny cached verification
and lease acquisition; restoring the lock must recover the original selection.
On Windows it also runs the committed `node.exe --version` with a cleared
environment while retaining the store lease. Other hosts verify foreign-target
materialization without trying to execute a Windows binary. All test stores are
temporary; source archives and project configuration are unchanged.

Admission and installer identities remain explicit test fixtures. This does not
prove production backend planning, publisher signatures, policy authorization,
resolver behavior, supervised exec, shims or shell activation. Full backend
qualification still needs those boundaries and the remaining OEP matrix.


### Real Go archive store qualification

The same opt-in harness also checks the real Go 1.24.13 Windows amd64 ZIP. Its
pinned identity comes from the independently captured official catalog and sidecar:
87,295,983 bytes and SHA-256
`40b16bc8f00540a2cb02dff4de72b73e966fdd8d65f95e33d8e4080b48a2459a`.
Provision it outside the checkout from the official Go download source. No Go
source or license files are copied into the repository; the original archive and
its notices remain intact in the external fixture. This is a historical test
release, not a version recommendation or approved production backend.

```sh
python tooling/prepare-go-store-fixture.py --archive /fixtures/go1.24.13.windows-amd64.zip --manifest /fixtures/go-store-manifest.json
export OYZU_GO_STORE_ARCHIVE=/fixtures/go1.24.13.windows-amd64.zip
export OYZU_GO_STORE_MANIFEST=/fixtures/go-store-manifest.json
cargo test --locked --test tool_native_archive real_go_zip_publication_parity_and_changed_lock_denial -- --ignored --exact --nocapture
```

PowerShell uses `$env:NAME = 'absolute path'`. Python 3.11+ and normal Rust
prerequisites apply. The shared Python ZIP oracle validates archive identity before
inventorying files without extraction, includes implicit parent directories and
exclusively creates its output manifest. Node's existing manifest is unchanged by
the shared oracle. Missing/wrong inputs or an existing manifest fail explicitly.

The synthetic Go test layout explicitly requests 800:1 expansion: the original
200:1 attempt correctly rejected three highly compressible upstream test files,
including 65,535 bytes compressed to 82 bytes. The implementation still defaults
to 200:1 and caps an explicit override at 1024:1, with entry, total-byte, per-file,
path and depth guards unchanged. This synthetic layout is not production admission.
The harness compares all 15,738 entries, publishes/verifies receipts, retains a
lease during Windows `go version`, `go env GOROOT`, compilation of a small real
Go module using the standard library, and execution of that program. The synthetic
layout declares GOROOT as the installation root. The build uses private caches and
temporary directories, clears inherited settings, disables automatic toolchain and
module downloads (`GOTOOLCHAIN=local`, `GOPROXY=off`, `GOSUMDB=off`), and disables
cgo. These Go settings are not an OS network sandbox or product environment
projection. The harness then checks changed-lock denial/recovery. Foreign
hosts materialize the Windows payload without executing it. Native CI provisions
Node and Go separately so each test requires only its own fixture inputs.

Windows qualification passed for all 15,738 entries, publication, changed-lock
denial/recovery, native version execution, GOROOT lookup and compilation/execution
of a standard-library program with the explicit 800:1 test layout.
Linux foreign-target materialization, publication and changed-lock checks also passed
with container networking disabled; it did not execute the Windows payload.
Native CI run [36993866491](https://github.com/micahlmartin/oyzu/actions/runs/36993866491)
at `ca32f4b82efd330b3b01911e643208a550189bbc` passed both Node and Go archive
steps on Windows, macOS and Linux. That revision verifies inventory, publication,
changed-lock denial and Windows version execution; it predates the added Go build
and GOROOT checks, whose native CI remains pending. These results do not establish
product Go build/environment integration, other native Go archives, publisher
signatures or end-to-end product installation.

### Raw single-file candidate materialization

The finalizer accepts `archive_kind = "raw"` for a verified artifact distributed
as one file. Its admitted layout must set `strip_prefix` to null, retain
`payload_subtree = "."`, and contain exactly one `required_paths` entry of kind
`file`. That entry's portable relative path is the destination, for example
`bin/jq`; no filename is inferred from a URL or untrusted download header. Parent
directories are implicit and count toward the entry and depth budgets. Other
entrypoints and environment references remain subject to normal receipt checks.

Bytes come only from the private verified blob snapshot and are copied unchanged
through anchored no-follow filesystem operations. Both total and per-file byte
limits apply; there is no decompressor or expansion amplification. Executable
permissions remain an explicit Unix layout transform and are unavailable on
Windows. A raw file used as a Unix native entrypoint must appear in
`executable_paths`; omitting that declaration fails the receipt entrypoint check. No downloaded code is executed. Errors produce no receipt; a partial
caller-owned candidate may remain and must not be published or reused as complete.

This adds materialization on Windows/macOS/Linux using the existing portable store
boundary without changing the layout format. Earlier binaries reject raw layouts;
existing archive layouts are unchanged. Synthetic tests cover exact bytes,
receipt publication, ambiguous/missing/non-file destinations, path escapes and
entry/depth/file-size bounds. This does not admit jq/Aqua or prove any real raw
backend's provenance, publisher verification, environment or execution behavior.
