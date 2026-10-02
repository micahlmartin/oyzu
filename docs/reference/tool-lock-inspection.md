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

The tar implementation accepts regular files, directories, bounded GNU
long-name/link extensions and local PAX `path`/`linkpath` records in tar or
gzip-compressed tar. The PAX names use the same containment/conflict validation
as ordinary names; this supports non-ASCII filenames in official Go archives.
Other PAX fields and global PAX records remain unsupported. Symlinks are delayed
until all file writes finish, then checked against the complete payload graph.
Windows symlink materialization, hardlinks, sparse entries, ZIP and other
formats remain unsupported and fail; backend admission must account for these
limits. Ownership, set-ID and archive directory permissions are not imported:
directories are private and traversable, files are private with Unix executable
bits retained. A reviewed layout plan is still required for final permissions.

Bounds include 200,000 raw and expanded entries, 8 GiB input/expanded bytes,
1 GiB per file, depth 64, 16 KiB GNU/PAX extension bodies and a 200:1 gzip expansion
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

The lease retains the receipt snapshot verified during acquisition. Call
`lease.command("node")` to obtain borrowed `LeasedToolCommand` metadata from that
exact selection: selection/tool/installation identities, tool ID, version,
platform and a typed `ToolLaunch`. Native launches retain a payload-relative
file and ordered prefix arguments. Interpreter launches additionally retain an
explicit interpreter installation/path. `ToolArgument` distinguishes literal
UTF-8 arguments (including empty strings) from typed installation-relative paths;
no quoting, shell parsing or wrapper inference occurs.

Lookup searches the entire selected closure and fails `TOOL_COMMAND_AMBIGUOUS`
if multiple receipts supply the requested command. Unknown commands fail
`TOOL_COMMAND_MISSING`; path-like or invalid names fail `TOOL_COMMAND_INVALID`.
It never searches ambient PATH or adds/removes `.exe`. Windows-target lookup
uses the receipt validator's uppercase command comparison; Unix targets use exact
spelling. A syntactically supported foreign target can be inspected this way,
but the metadata does not imply it can run on the current host.

The result borrows the lease and contains symbolic paths, not executable host
paths or execution authority. Lookup does not reopen receipts, rehash payloads,
run tools, contact a service or observe later edits. A new lease acquisition
rechecks current bytes; a retained lease is not a change monitor or protection
against same-user mutation. Future launch consumers must separately enforce
current authorization/content validity, resolve paths within leased payloads and
supervise the child tree. No `which`/`exec` command is enabled by this library API.

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
`InstallationLease::selection_digest()` borrows the identity from the verified
receipt snapshot used for command lookup and lease journaling. There is no
separately mutable digest field. Experimental Rust callers must replace
`lease.selection_digest` with `lease.selection_digest()`; lock, receipt, journal
and CLI formats are unchanged. This identity describes the acquired snapshot,
not current policy authorization or protection against later same-user edits.
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

`ToolRequestIdentity::parse(json_bytes, admitted_ids)` reads the serialized
identity record for an internal consumer. The caller supplies canonical IDs
from a trusted catalog, not from the record. Exactly `requests`,
`native_constraints`, `required_capabilities` and `digest` are required; extra
fields and duplicate JSON keys fail. It shares projection's normalization and
identity calculation, requires already sorted sets and checks the supplied digest
against the recomputed value. Literal version syntax remains unchanged.

Decoding is bounded to 8 MiB, nesting depth 32 and 10,000 JSON entries before
typed conversion, followed by the limits above. No files, network or tools are
accessed. Catalog membership is only a consistency check, not source trust or
permission to use a backend. A caller must also bind the record's digest to its
trusted operation context; anyone can compute a digest for a different request.
This parser supplies the normalized request portion of a future worker payload,
not the complete resolve-operation schema, broker session or catalog provenance.

`ToolRequestIdentity` is immutable after projection or checked parsing. Rust
consumers use `requests()`, `native_constraints()`, `required_capabilities()` and
`digest()` to borrow its contents. This replaces the experimental public fields;
callers needing a changed request must project or parse a new identity so its
digest is recomputed. JSON field names and digest calculation are unchanged.
Frozen selection uses these accessors to compare the same validated request map
and digest. Immutability preserves consistency, not policy approval or freshness.

The [draft request-record schema](../contracts/tools-v1/request-identity.schema.json)
and `tests/fixtures/tool-requests/` share malformed-shape cases with the Rust
parser. Run `python tooling/check-tool-contracts.py` with the design requirements
installed to check the schema fixtures. Schema string lengths count characters;
the runtime additionally enforces UTF-8 bytes, aggregate budgets, sorted order,
catalog membership and digest recomputation. Passing shape checks does not prove
those semantic checks or qualify a backend.

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
Source pins and digests require exact whole-string matches; trailing newlines
are rejected by both the schema and runtime parser. Shared malformed fixtures
exercise all five identity fields.

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

### Real jq raw-artifact qualification

An opt-in store test uses the original jq 1.8.1 Windows amd64 release binary:
1,026,560 bytes, SHA-256
`23cb60a1354eed6bcc8d9b9735e8c7b388cd1fdcb75726b93bc299ef22dd9334`.
Its release API digest and `sha256sum.txt` agree with the downloaded bytes. This
is declared checksum consistency, not publisher signature verification. Retain
the original artifact and upstream release `COPYING` outside the checkout.
Neither is incorporated into Oyzu or its distributed package by this test.

```sh
python tooling/prepare-jq-store-fixture.py --archive /fixtures/jq-windows-amd64.exe --manifest /fixtures/jq-store-manifest.json
export OYZU_JQ_STORE_ARCHIVE=/fixtures/jq-windows-amd64.exe
export OYZU_JQ_STORE_MANIFEST=/fixtures/jq-store-manifest.json
cargo test --locked --test tool_native_archive real_jq_raw_publication_parity_and_changed_lock_denial -- --ignored --exact --nocapture
```

Python 3.11+ is required. The helper rejects size/hash mismatch and exclusively
creates the manifest without executing the binary. The layout maps the raw bytes
to `jq.exe`, with synthetic Aqua admission and installer identities. The test
verifies caching, exact payload parity, receipts, publication and changed-lock
rejection/recovery; Windows runs `jq --version` while retaining the lease. Other
hosts materialize the Windows artifact without executing it. Each native CI job
provisions this fixture separately from Node and Go. No network is used by the
Rust case itself. Windows execution and Linux foreign-target materialization
(with Docker networking disabled) passed locally; updated native CI remains
pending. This is not Aqua registry/resolver qualification, legal approval or a
production installation command, and does not qualify other native jq targets.

### Second Go release fixture

The same harness also supports the pinned Go 1.25.0 Windows amd64 ZIP, whose
67,418,204-byte identity and SHA-256
`89efb4f9b30812eee083cc1770fdd2913c14d301064f6454851428f9707d190b`
come from the retained official catalog/sidecar evidence. Its independent manifest
contains 16,087 entries. This is a historical qualification fixture, not a
recommendation or production backend admission.

```sh
python tooling/prepare-go-store-fixture.py --version 1.25.0 --archive /fixtures/go1.25.0.windows-amd64.zip --manifest /fixtures/go125-store-manifest.json
export OYZU_GO125_STORE_ARCHIVE=/fixtures/go1.25.0.windows-amd64.zip
export OYZU_GO125_STORE_MANIFEST=/fixtures/go125-store-manifest.json
cargo test --locked --test tool_native_archive real_go125_zip_publication_parity_and_changed_lock_denial -- --ignored --exact --nocapture
```

The provisioner accepts only `1.24.13` (the unchanged default) and `1.25.0`;
each selection validates its own exact size/hash before inventorying bytes.
Existing Go fixture commands retain their meaning. Separate environment names
allow both fixtures in one test run without sharing archives or manifests.
The second release uses the same explicit 800:1 synthetic layout bound and checks
all inventory entries, receipt publication, changed-lock rejection/recovery and,
on Windows, native version, GOROOT, compilation and execution while leased.
Other hosts only materialize the Windows payload. CI provisions and selects the
case independently; results do not establish native Linux/macOS Go behavior or
production resolution, acquisition, policy or builder integration.

Windows Go 1.25.0 qualification passed all 16,087 entries, publication and
changed-lock checks, GOROOT lookup and real compilation/execution. Linux
foreign-target verification also passed with networking disabled. Updated native
CI confirmation is tracked separately in the
[tool-management status](../tool-management-status.md).

### Real Temurin Java archive qualification

The opt-in Java case uses the historical Temurin `jdk-21.0.6+7` Windows amd64
HotSpot JDK ZIP from the [official release](https://github.com/adoptium/temurin21-binaries/releases/tag/jdk-21.0.6%2B7).
Its exact size is 204,643,847 bytes and its SHA-256, checked against the release's
original checksum sidecar, is
`897c8eebb0f85a99ccecbd482ebae9a45d88c19d6077054f6529ebab49b6d259`.
This is a reproducible test fixture, not a current-version recommendation,
publisher-signature check, license clearance or production backend admission.

Provision the archive outside the checkout, retaining its original legal files.
Python 3.11+ inventories all 576 payload entries without extracting or executing
them. The helper rejects a mismatched size/hash or existing output manifest.
For a new inventory, choose a fresh manifest path; do not overwrite the archive.

```sh
python tooling/prepare-java-store-fixture.py --archive /fixtures/OpenJDK21U-jdk_x64_windows_hotspot_21.0.6_7.zip --manifest /fixtures/java-store-manifest.json
export OYZU_JAVA_STORE_ARCHIVE=/fixtures/OpenJDK21U-jdk_x64_windows_hotspot_21.0.6_7.zip
export OYZU_JAVA_STORE_MANIFEST=/fixtures/java-store-manifest.json
cargo test --locked --test tool_native_archive real_java_zip_publication_parity_and_changed_lock_denial -- --ignored --exact --nocapture
```

On PowerShell set the same variables with `$env:NAME='path'`. The synthetic
`core:java` layout uses version `temurin-21.0.6+7.0.LTS`, strips `jdk-21.0.6+7`,
preserves the complete payload including every legal file, declares `bin/java.exe`
and `bin/javac.exe`, and supplies self-relative PATH and JAVA_HOME metadata.
It retains the default 200:1 extraction bound. Independent inventory parity,
receipt publication, lease lookup and changed-lock denial/recovery run on all
hosts; only Windows executes this Windows payload. While holding the selection
lease, Windows checks the exact runtime build, selects the compiler through the
same lease, compiles a class with annotation processing disabled and runs it.
The commands clear inherited environment/options, use private working/temp paths
and an explicit classpath. They make no package-manager requests; this is not
OS network containment or a product Java build integration.

The locked tool version follows mise's catalog spelling, including `.0.LTS`;
the archive directory, upstream release tag and runtime build have different
spellings. The [captured catalog observations](../proposals/OEP-0003-mise-integration/java-catalog-evidence.json)
bind the three public metadata responses by size/hash and record the exact JDK
identity, URL and declared checksum. The Windows checksum agrees with the retained
official release sidecar and archive. These observations are not backend replay
or archive verification on the other targets. Older experimental manifests using
`temurin-21.0.6+7` must be regenerated at a fresh output path with the helper;
the fixture does not alias or silently migrate that synthetic identity.

CI provisions the pinned archive separately on Windows, Linux and macOS.
Provisioning needs public network access; the test runs from local fixtures.
Native Linux/macOS JDK execution, production resolution, admission, acquisition,
publisher verification and builder handoff remain separate acceptance work.
Measured results are tracked in [tool-management status](../tool-management-status.md).

### Draft tool-selection grants

`docs/contracts/tools-v1/selection-grant.schema.json` defines a proposed closed
payload shape for a future compact signed grant. The fixture files under
`tests/fixtures/tool-grant/` are unsigned synthetic data and grant no authority.
Run `python tooling/check-tool-contracts.py` with the existing pinned design
requirements to check them offline; no agent, management account or signing key
is needed, and no configuration or store data is modified.

The shape requires protocol/request/context/decision identity, policy revision,
selection digest, operation list, validity, offline flag, revocation epoch,
issuer/audience/tenant/subject. It rejects unknown fields, missing bindings,
duplicate/unknown operations, wrong audience/protocol, malformed digest/UUID and
non-integer or unsafe numeric fields. An allow payload is the only grant shape;
resolve cannot share an operation list with execution, and offline-enabled grants
list only activate/exec/build. Times are proposed whole Unix UTC seconds. These
are new draft wire choices, not an accepted or deployed service contract.

The schema alone checks shape. The initial Rust `VerifiedToolGrant::verify`
library API additionally verifies compact JWS signatures against a caller-supplied
pinned Ed25519 key map and exact `ToolGrantContext`. The caller must obtain keys,
issuer/tenant/subject, request/context identity, policy revision, revocation epoch
and selection digest independently from authenticated agent state. Supplying these
values from the token or project would defeat their binding. This API does not
authenticate its caller or fetch keys, contact an authorizer or authorize a launch.

Only `alg=EdDSA` is accepted, with a nonempty `kid` and optional
`typ=oyzu-tool-selection+jws`; other headers, duplicate JSON and unknown payload
fields fail. Limits are 128 KiB per token, 8 KiB decoded header, 64 KiB decoded
payload and 256 trusted keys. Existing strict JSON depth/entry bounds also apply.
The configuration-policy verifier retains its different `alg=Ed25519` header
and policy audience unchanged.

Call `verify(token, keys, context, unix_seconds, Instant::now())` on online
receipt, then `check(context, unix_seconds, Instant::now(), offline)` before each
use. Signed ordinary lifetimes cannot exceed 60 seconds. Offline-enabled grants
permit only activate/exec/build and cannot exceed 900 seconds. Online receipt and
online use are limited to the first 60 seconds of either kind of grant. Offline
use requires its explicit flag and the same live object; no restart persistence
is provided. Fixed monotonic deadlines use the remaining signed time at receipt,
so neither mode changes nor wall-clock movement extend validity.

Changed context bindings, expiry, backwards wall-clock movement beyond five
seconds or monotonic reversal permanently invalidate the instance. An operation
outside the signed list is denied. `invalidate()` handles explicit invalidation;
the owning agent must call it on logout, key/policy changes and uncertain resume.
Those notifications are not wired yet. Recovery requires a fresh online grant,
never editing project/lock data or retrying a permanently invalidated instance.
Tokens and verified objects have no persistence API and errors omit token data.

The implementation uses portable Rust clocks/cryptography; focused tests use
synthetic signatures and the shared valid/invalid payload fixtures. This remains
an initial library boundary, with no deployed service interoperability, agent
lifecycle integration or grant-backed launch path. The protocol remains a draft.

### Internal worker control framing

`ToolWorkerChannel<T>` provides the initial private-channel byte framing for
OEP-0003. It takes an already established duplex `Read + Write` transport; it
does not discover a channel, start a subprocess or read worker stdin/stdout.
The supervisor must authenticate and restrict inherited channel handles and
enforce deadlines/cancellation independently. A blocking `Read` or `Write` is
not made interruptible by this codec. This is an unstable internal interface,
not a public worker/plugin compatibility promise.

The sequential `ToolWorkerChannel` is suitable only when no read/write overlap
is needed. For cancellation while waiting on a response, use
`split_tool_worker_channel(read_handle, write_handle)` to obtain a
`ToolWorkerReceiver` and `ToolWorkerSender`. Supply already established halves of
the same trusted channel; this function neither creates nor duplicates handles.
Each half can be owned by a separate thread. There is one shared atomic byte
budget and failure state, and no synchronization lock spans transport I/O.
The sender can therefore transmit a cancel while the receiver is blocked.
The receiver reserves prefix bytes before reading them and body bytes before
allocation/read; the sender reserves its complete frame before writing. In-flight
reservations count against the same limit and are not refunded after failure.

Failure in either half prevents subsequent successful operations in both.
`abort()` on either half marks the shared state closed, and an in-flight read
that later completes cannot return a successful frame. It does not interrupt the
underlying syscall or retract bytes already written. The supervisor must still
apply OS deadlines and explicitly shut down/cancel the underlying transport on
timeout; dropping or invalidating a framing object is not process cancellation.
Neither half is cloneable, so callers cannot accidentally interleave multiple
framed writers through this API.

Construct one channel per operation with `ToolWorkerChannel::new(transport)`.
`send(json_bytes)` validates one UTF-8 JSON object, writes a four-byte big-endian
length and the original bytes, then flushes. `receive()` reads exactly one frame
and returns an untrusted JSON value. Both directions share a 32 MiB lifetime
budget, including length prefixes; each body is at most 8 MiB. Empty bodies,
non-object roots, duplicate keys, malformed/nonfinite values and excessive nesting
fail. The shared strict JSON parser also caps aggregate entries at 10,000.
Oversized lengths and exhausted budgets fail before allocating or reading bodies.
Malformed outgoing JSON emits no bytes.

All failures permanently invalidate the channel instance. This includes clean
EOF, short reads, invalid input and write/flush failures; retry requires a new
supervised operation rather than attempting stream resynchronization. Errors
use `TOOL_WORKER_*` codes without payload or transport-error text. No files,
network connections or credentials are created by the codec itself.

Framing success is not envelope admission. `ToolWorkerExchange`, described below,
adds closed outer envelopes and response correlation; operation-specific payload
admission and worker-side dispatch remain unimplemented. Complete authenticated
bootstrap, handshake/operation deadlines, embedded backend dispatch and executor
containment are also absent. Windows restricted process inheritance is implemented
separately below; Unix process creation remains outstanding. The native allocation API creates
local endpoints only. Tests exercise fragmented reads/writes,
exact frame bounds, combined budgets, invalid/truncated JSON and terminal errors;
they do not qualify a running worker or complete TM-05.

Concurrent tests additionally hold a receive pending while sending cancellation,
then check shared abort and budget exhaustion. A Unix-only test exchanges control
frames over a real close-on-exec socketpair. It does not launch a process, verify
inherited descriptor restrictions or establish Windows handle-list support.

### Native private worker endpoints

`native_tool_worker_channel()` creates two connected `NativeToolWorkerEndpoint`
values without stdin/stdout, an environment token, a filesystem socket name or a
listening network port. Unix uses a socketpair, with duplicate descriptors for
independent read/write ownership. Windows uses two anonymous pipes through the
existing standard-library [pipe API](https://doc.rust-lang.org/std/io/fn.pipe.html).
No new crate dependency or executable is added. Creation checks `FD_CLOEXEC` on
Unix or absence of `HANDLE_FLAG_INHERIT` on Windows and fails if any returned
handle is inheritable. Dropping an endpoint closes its owned handles.

An endpoint implements `Read + Write` for sequential framing. Its `split()`
consumes it and returns `NativeToolWorkerReader` and `NativeToolWorkerWriter`
without additional duplication. Pass those to `split_tool_worker_channel` for
concurrent framing with a shared budget. The halves expose borrowed native
descriptors/handles through `AsFd` or `AsHandle` for future explicit supervisor
inheritance and cancellation; they do not transfer ownership or reopen a name.

This API allocates blocking transports. It does not spawn a worker, authenticate
an inherited channel, configure a Windows process handle list, install an OS
deadline, interrupt pending I/O or contain backend code. A dropped peer produces
EOF, which framing treats as terminal failure. A blocked read/write with a live
peer still requires supervisor-owned OS cancellation/deadline enforcement.
The factory must not be presented as a deadline-bounded production worker.

Windows and Linux tests transfer a 2 MiB JSON payload in both directions using
the native factory and framed split transport, then verify peer-close failure
propagates to both framing halves. Creation itself checks non-inheritance.
Additional Windows/Linux regressions leave a length prefix incomplete or write
a 2 MiB frame while the peer never drains the transport. Closing the peer then
causes each pending operation to fail and makes the other framing half terminal.
These tests depend on all peer endpoint handles closing. They do not prove that
killing a child closes handles inherited by descendants, or that a live peer can
be interrupted; the supervisor must establish those lifecycle properties.
The test watchdog bounds the test harness, not production I/O. macOS uses the
Unix implementation but native verification of this increment remains pending.
The Windows process primitive below now implements explicit cross-process
inheritance. Unix inheritance and actual worker dispatch remain unfinished.

### Interruptible native control I/O

`NativeToolWorkerIo::new(endpoint)` takes ownership of one private native endpoint
and wraps the existing shared framing budget. This experimental supervisor API
provides one dedicated read thread and one dedicated write thread at most, with
no frame queue. `start_receive()` and `start_send(owned_bytes)` begin a frame;
`try_receive()` returns `None` while its read is pending and `try_send()` returns
`false` while its write is pending. Completed operations are joined before results
are returned. Another send can therefore deliver a cancel while a response read
is blocked. A second start in the same direction before collecting completion
fails with `TOOL_WORKER_IO_PENDING`; polling without a started operation fails
with `TOOL_WORKER_IO_NOT_STARTED`.

The existing 8 MiB frame, 32 MiB session and JSON shape limits still apply. Send
input is validated before creating its thread; invalid input closes the operation.
Results are untrusted JSON, not admitted worker responses or installation receipts.
This API has no network, backend, filesystem configuration or process-spawn effects.

The supervisor calls `shutdown(absolute_monotonic_deadline)` after cancellation,
expiry or transport failure. It permanently closes framing, interrupts native I/O,
discards racing results and joins both threads. Unix shuts down the retained socket;
Windows repeatedly requests synchronous I/O cancellation against the owned,
dedicated thread handles until completion is observed. Those threads never execute
an unrelated operation or return to a thread pool. No peer closure is required.
This follows Microsoft's [synchronous I/O cancellation guidance](https://learn.microsoft.com/en-us/windows/win32/fileio/canceling-pending-i-o-operations): requesting cancellation alone does not establish completion.

`TOOL_WORKER_IO_SHUTDOWN_TIMEOUT` means cleanup is still incomplete: the owner
retains pending threads and handles, permits another shutdown attempt and refuses
all result retrieval or new I/O. Do not report successful cleanup or publish a
result on this path. Dropping the owner also interrupts and joins; it can wait
if the operating system fails to complete cancellation. It never deliberately
detaches pending I/O threads. This is not a guarantee against a stuck OS driver.
Shutdown uses at most 5 ms polling sleeps, shortened to the remaining deadline.

Tests exercise simultaneous partial-frame read and backpressured write with a
live peer, duplex request/cancel traffic, cancellation independent of another
channel, discarded completed results, pending-operation rejection, joining on
drop and retained ownership after cleanup timeout. Run
`cargo test --locked --lib tools::worker::io`. Native process creation and job
termination are separate Windows components described below. These I/O tests do
not establish channel authentication, deadline scheduling, backend dispatch or
authorization to use a backend.
The six tests pass on Windows GNU with Rust 1.94 and Linux with Rust 1.95
(network disabled). At commit `63e8073`, the full locked unit/integration test,
strict Clippy and formatting steps also passed on Windows, macOS and Linux in
[native CI](https://github.com/micahlmartin/oyzu/actions/runs/37031170986).
That observation covers those completed steps, not every downstream CI job or
later process-lifecycle changes.

### Windows worker process-tree cleanup

On Windows, `WindowsToolWorkerJob::new()` creates an unnamed, non-inheritable
job with kill-on-last-handle-close enabled and no process breakaway permission.
The native spawn adapter must create the initial worker suspended, call
`assign_suspended(process_handle)`, and only then resume its primary thread.
Assignment is permitted once, including when the native assignment fails. If it
fails, the caller must terminate and reap the still-suspended initial process;
it must never resume it or retry assignment on that job. This API does not
verify the image, inspect suspension, transfer the supplied process handle,
establish a sandbox or authenticate a control channel.

`terminate(absolute_monotonic_deadline)` requests termination of every process in
the job, then queries native job accounting until no active processes remain.
This includes descendants and any associated console-host processes. Success
requires an empty job, not just a successful termination request. A repeated
termination call is permitted. Assignment is permanently closed once termination
starts, even for an initially empty job. The supervisor must independently join
control I/O and finish handle/receipt cleanup before reporting operation cleanup.

`TOOL_WORKER_JOB_CLEANUP_TIMEOUT` or a native query/termination error means exit
has not been confirmed. The owner retains its job handle so cleanup can be
retried. Closing the last handle requests job-wide termination as a fallback;
dropping this value alone does not claim observed exit. Native errors preserve
their OS cause under stable `TOOL_WORKER_JOB_*` diagnostic context.

The Windows-only tests launch the test executable suspended, assign it before
resuming, and have it create a real descendant. They check breakaway denial,
membership of both known processes, confirmed exit of both after termination,
empty job accounting, independence of another live worker, last-handle-close
termination, failed-assignment sequencing and empty-job shutdown. Console hosts
were observed as additional job members on Windows; the tests intentionally
require cleanup of all members rather than assuming a fixed process count.
Run `cargo test --locked --lib tools::worker::windows_job`. The ignored child
fixture is invoked by those tests; it is not a product worker entry point.
All four lifecycle tests pass on Windows GNU with Rust 1.94, as do the full
locked suite, strict all-target Clippy, formatting and real CLI task scenarios.
At commit `d9c18de`, the locked unit/integration tests, strict Clippy and formatting
also passed on native Windows MSVC in
[CI run 37032992735](https://github.com/micahlmartin/oyzu/actions/runs/37032992735).
The same checks passed on Linux and macOS; the Windows-only job tests do not run
on those hosts. The complete workflow was still running when these completed
steps were recorded; downstream build/task acceptance is not inferred from them.

This is the Windows job component. The process primitive below supplies pinned
same-binary creation and an explicit inherited handle list. Combined process/I/O
deadline scheduling and backend dispatch remain unimplemented.
Unix process groups and their equivalent cleanup verification remain outstanding.
The implementation follows Microsoft's [job-object lifecycle contract](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

### Windows same-binary process creation

`WindowsToolWorkerProcess::spawn_current(expected_image_digest, arguments,
environment, directory, inherited_handles)` is an experimental native supervisor
primitive. It resolves the running executable from the OS, never PATH or a project
image setting. The expected `sha256:` digest must come from independently trusted
release admission; computing a hash of whatever file happens to be present does
not establish that admission. This API does not approve the release, admit backend
capabilities, authenticate a protocol bootstrap or expose an install/exec command.

Before creation it opens the image and all canonical parent directories without
following reparse points at those opens. Parent handles deny directory replacement;
the image handle denies writes/deletion while held. It hashes the opened image in
bounded chunks and requires the expected digest and exact observed length. Images
larger than 512 MiB are rejected by this initial implementation. The handles stay
owned for the process owner's lifetime. A changed release fails with
`TOOL_WORKER_IMAGE_DIGEST_MISMATCH`; retry requires trusted release selection, not
accepting the newly observed hash. No image or parent directory is modified.

Image pinning is owned by `worker/windows_process/image.rs`. Disposable-file
tests exercise write, delete, image rename and parent-directory rename denial
while a pin is held, then successful replacement after release. A digest rejection
must release every acquired handle so the update/retry path remains available.
These tests use ordinary temporary files rather than a loaded executable, so the
Windows loader's own image lock cannot substitute for the pin's protection. The
path-taking helper is private; production still selects only the running executable.
Run `cargo test --locked --lib tools::worker::windows_process::image`.
These two pin-lifetime tests pass on Windows GNU Rust 1.94. Native MSVC verification
of the extracted image module and its new tests remains pending.

The caller supplies an absolute, admitted private working directory and literal
bootstrap arguments. Arguments are quoted for native Windows parsing, without a
shell. There are at most 256 arguments and 32,767 UTF-16 code units in the encoded
command line including its terminator; NUL values are rejected. An explicit
environment map replaces the inherited environment. It permits at most 256 ASCII
alphanumeric/underscore names of 1-256 bytes, rejects case-insensitive duplicates
and embedded NULs, and bounds the complete UTF-16 block to 32,767 code units.
The empty map produces an empty environment block. The supervisor must supply
only admitted settings; the primitive does not interpret or authorize their values.

At most 16 caller-owned capability handles are consumed by launch. Their raw
values remain unchanged for bootstrap arguments; the caller must bind their kinds
and purpose independently. The child receives only these selected handles and
one null-device handle used for its standard streams. Standard streams carry no
control traffic and inherit no parent console output handle. Parent copies of
the consumed handles close before the child is resumed. This uses the native
[process handle-list attribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute), not broad inheritance into this child.
Selected handles are temporarily inheritable in the parent during launch;
coordination with any concurrent broad-inheritance launchers remains a supervisor
integration obligation. The worker bootstrap must validate and clear inheritance
on received handles before executing backend code.

Creation is hidden and suspended. The process is assigned to its private job
before its primary thread resumes. A failed assignment/resume terminates and
waits for the initial process instead of leaving a suspended orphan. Earlier
failure closes consumed handles without resuming a child. Native startup itself
is synchronous; automatic handshake/operation deadline scheduling is not supplied
by this primitive. `try_wait()` reports the full native DWORD exit code of the
initial process. `terminate(deadline)` confirms an empty job and initial-process
exit; it does not join I/O. Call the I/O owner's shutdown as part of the same
supervised cleanup. Dropping the process owner requests job termination through
kill-on-close, without claiming observed exit.

Tests spawn this same test binary through the primitive. They verify literal
argument parsing, explicit environment, access to a selected event but no access
to an unrelated inheritable event, request/response traffic on private pipes,
termination with a descendant retaining those pipes and blocked I/O, partial-frame
child exit, preservation of a high-bit DWORD exit code, wrong-image rejection and
handle closure after failed native creation. A separate native Windows argument
parser checks empty, quoted, Unicode and trailing-backslash argument round trips.
Run `cargo test --locked --lib tools::worker::windows_process`.
All four native process tests pass on Windows GNU Rust 1.94, together with the
full locked suite, strict all-target Clippy, formatting and real CLI task scenarios.
At `b68b983`, the locked unit/integration suite, strict Clippy and formatting also
passed on Windows MSVC in
[run 37036054434](https://github.com/micahlmartin/oyzu/actions/runs/37036054434).
Those steps passed on Linux/macOS too, where Windows-only tests are excluded.
That observation predates the extracted image module and its two pin-lifetime
tests, and does not establish success of the still-running full workflow.

The focused `Tool worker lifecycle` workflow runs formatting, strict all-target
Clippy and `cargo test --locked --lib tools::worker` on Windows, macOS and Linux
with Rust 1.94. It runs for relevant changes on `main` and the active integration
branch, and supports manual dispatch. Native tests launch their own ignored
subprocess fixtures; ordinary `--ignored` execution is not required. This workflow
does not run archive qualification, CLI task scenarios or captured builds. The
full `CLI and scenarios` workflow remains the broader regression/acceptance suite;
a green worker workflow cannot establish full OEP-0003 completion.

The tests are lifecycle qualification, not mise execution. Release-manifest
admission, the product worker entry point and authenticated bootstrap, automatic
deadlines, backend dispatch, Unix spawning and the complete supervisor remain
unfinished. No separate mise executable is used.

### Worker exchange correlation

`ToolWorkerExchange::new(request_bytes)` checks a closed outer request: exact
protocol, canonical UUID, context/backend-release digests, a recognized target
platform, operation enum, sorted unique capability IDs and an object payload.
Use one exchange per supervised operation. `operation()` exposes the selected
operation; `untrusted_payload()` deliberately does not imply payload admission.
Recognized platform syntax does not establish backend/platform support, and
capability names do not prove executor capabilities.

Both `ToolWorkerExchange::context()` and `ToolWorkerSession::context()` return
a borrowed `ToolWorkerRequestContext` containing `request_id`, `context_digest`,
`backend_release_digest`, `target_platform` and the sorted capability IDs. These
are the fields from the validated original request, not a second parse or a
separately mutable identity. Use them to compare the request with independently
trusted supervisor state before dispatch. They remain inspectable after the
operation ends; inspecting this snapshot does not renew the operation, authenticate
the channel, admit a capability or grant execution permission. No wire format or
request validation rule changes with this accessor.

`finish(response_bytes)` compares protocol, request ID and context digest with
the original request. It accepts exactly one `ok` response with an object result,
or `error`/`cancelled` with a closed diagnostic containing only `code`. Codes are
1-64 uppercase ASCII letters, digits or underscores; arbitrary worker message
text is not accepted. Even successful results use the explicitly named
`ToolWorkerOutcome::UntrustedResult` variant and require operation-specific
validation, containment/digest checks and current authorization before use.

Call `cancel()` once to obtain a correlated envelope with the same protocol,
request ID and context digest and `operation="cancel"`. It marks cancellation
before transport; any write failure requires aborting the operation. A later
success response is discarded rather than allowing a cancelled operation to
publish. `abort()` handles timeout, transport loss or supervisor cancellation.
Any response attempt, including malformed or mismatched input, terminates the
exchange; a second response is rejected. This state machine does not send bytes,
interrupt a blocking transport, cancel a process or authenticate the channel.

The [outer-envelope schema](../contracts/tools-v1/worker-envelope.schema.json)
and shared fixtures cover request/response shapes. Sorting, response correlation
and lifecycle rules are runtime checks. Cancel encoding, code-only diagnostics
and cancellation-race handling are initial draft wire choices requiring contract
review. Operation-specific request/result records and backend cancellation
remain outstanding; this is not the complete typed worker schema or a deployed
worker protocol.

### Worker-side operation session

On Windows, `WindowsToolWorkerLifecycle::new(process, io)` joins ownership of an
already-created process and its control channel. The trusted caller must associate
the matching endpoint; this constructor does not authenticate their relationship.
Use `control()` for protocol I/O and `try_wait()` for the initial process status.
`shutdown(absolute_cleanup_deadline)` permanently denies further control access,
interrupts both I/O directions, terminates and observes the job, then attempts I/O
joining even if process cleanup failed. Both phases use the same deadline.
Success returns the native initial-process exit code and confirms job-wide exit
and joined threads. Failure retains ownership for another shutdown attempt;
`TOOL_WORKER_PROCESS_CLEANUP_FAILED`, `TOOL_WORKER_CONTROL_CLEANUP_FAILED` or
`TOOL_WORKER_CLEANUP_FAILED` preserve the relevant causes. A retry never reopens
the protocol. Synchronous native calls are not preempted by this deadline.

Dropping the lifecycle interrupts I/O and releases the process/job owner before
the I/O owner joins its threads. Job close requests descendant termination;
only explicit successful shutdown confirms process cleanup. Drop may still wait
if native I/O cancellation fails to complete. Native tests cover descendants
holding both control pipes, exhausted-budget retry, denial of protocol reuse and
drop cleanup with a live transport peer. This Windows-only composition adds no
backend dispatch, launch authorization, bootstrap protocol or Unix supervisor.

For bounded synchronous collection of an already-started native I/O operation,
`NativeToolWorkerIo::wait_receive_frame(deadline)` and `wait_send(deadline)` use
an absolute `std::time::Instant`. They check expiry before polling and before
returning a completed result. Expiry returns `TOOL_WORKER_IO_DEADLINE_EXCEEDED`,
closes both directions and requests native interruption. An already-completed
result collected after expiry is discarded. No elapsed-time extension occurs
between polls; sleeps are at most one millisecond, subject to OS scheduling.
The caller must still terminate the worker process and call `shutdown` with its
separate cleanup deadline to confirm that all I/O threads joined. Expiry is not
proof of process exit or completed cleanup, and the closed instance cannot be
reused. These optional blocking waits do not schedule protocol defaults or a
cancellation grace period. A supervisor that must concurrently observe process
exit or user cancellation should use the nonblocking collection methods instead.
Native tests keep peers open during incomplete reads and backpressured writes,
then verify deadline failure, rejection of reuse and confirmed thread cleanup.

`ToolWorkerExchange::request_digest()` commits to the exact validated request body
using SHA-256 over `oyzu.tool-worker.request.v1` followed by a NUL byte and the
request bytes. The four-byte frame prefix is excluded. The supervisor must compute
this expectation from its independently admitted request before launch.
`ToolWorkerSession::from_committed_request(bytes, expected_digest)` validates the
digest format and envelope, then rejects a mismatch with
`TOOL_WORKER_REQUEST_COMMITMENT_INVALID`. This binds payload bytes as well as the
outer identity. Even semantically equivalent JSON with different whitespace or
field ordering fails against the original commitment; send the original bytes.
After failure, tear down the operation rather than replacing the expectation
with a digest computed from received input.

For this startup path, `ToolWorkerChannel::receive_frame()` and the receiver's
equivalent return `ToolWorkerFrame`, preserving the bounded original body through
`bytes()` alongside `untrusted_value()`. `NativeToolWorkerIo::try_receive_frame()`
collects the same single pending read as `try_receive()`; collecting through one
consumes it for both. Existing value-only receive APIs remain available. Frame
limits and shared accounting are unchanged. These APIs perform no network access
or persistent writes, and require no new configuration or lockfile fields.

The digest is an integrity commitment, not a credential. Supplying both request
and expectation from an untrusted source establishes no authority. Authenticated
launch context, capability-handle validation and typed payload admission remain
separate requirements. Tests cover an independent digest vector, payload
substitution, changed JSON spelling and exact-byte exchange over native channels
between test threads; they do not exercise a product worker entrypoint.

`ToolWorkerSession::new(request_bytes, expected_operation, expected_context)`
applies the same bounded outer-request validation as the supervisor exchange,
then requires exact equality with independently trusted supervisor state before
creating a session. It compares operation, request ID, context digest, backend
release digest, target and the complete ordered capability list. A mismatch returns
`TOOL_WORKER_REQUEST_BINDING_INVALID`; neither adding nor dropping capabilities
is accepted. Expected values must come from the supervisor operation, never from
the received request. This is identity binding, not channel authentication or
validation of payload values and capability handles. `operation()` and `untrusted_payload()` expose only the validated
operation name and untrusted payload. One session represents one operation; it
does not authorize serving multiple workspaces in a persistent worker process.

`accept_cancel(bytes)` accepts exactly the correlated cancel envelope once.
Unknown fields, duplicate JSON keys, another initial request, foreign identities,
malformed JSON and repeated cancellation permanently end the session. A valid
cancel prevents later success output. The dispatcher must still interrupt or
stop backend work; accepting the envelope alone has no process effect.

`finish(ToolWorkerOutcome)` encodes one correlated terminal response and reuses
the supervisor's validation, including frame limits, object results, code-only
diagnostics and cancellation-race rejection. Invalid output consumes the attempt;
it cannot be replaced with a second response. `abort()` ends the session after
transport loss or dispatcher failure. A send failure requires teardown, with no
retry or new context in the same worker. Result contents remain untrusted and
require independent supervisor validation before any publication or execution.

Before recursive response serialization, the shared JSON tree validator checks
the same depth-32 and 10,000-entry limits used on input. Encoding then writes into
a bounded buffer and stops before an append would exceed 8 MiB, counting JSON
escaping and envelope fields. Geometric buffer reservation is capped at that
frame size; output is not fully serialized into an unbounded temporary first.
This bounds the encoded buffer, not memory a backend already used to construct
its result. Encoding failure ends the session and emits no response bytes.

Focused tests cover all three terminal outcomes, invalid/oversized output,
malformed/foreign/duplicate cancel, repeated response and success-after-cancel.
A native private-channel test sends a request and cancel, receives the correlated
cancellation diagnostic and observes peer closure. The 15-second test watchdog
does not implement production deadlines. Run `cargo test --locked tools::worker`;
no mise dependency, network or backend execution is required. Worker process
composition and channel authentication, actual dispatch, OS deadline scheduling,
backend cancellation and typed operation payloads remain absent. The Windows
native process primitive above separately implements image pinning and restricted
inheritance; the session itself does not perform those operations.
