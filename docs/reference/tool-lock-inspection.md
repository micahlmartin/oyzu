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
byte to detect overflow. It does not read project configuration or mise files,
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

Ordinary lease destruction removes its own record before releasing kernel locks.
Cleanup errors conservatively leave a stale record; destruction cannot report
them to the caller. Forced termination can also leave a final record, and a crash
during writing can leave a `.pending` file. Neither PID/timestamp nor a record's
presence establishes liveness: recovery must use OS locks and the relevant
reference/owner checks, never PID alone. Records are not yet automatically reaped.
A journal creation/publication failure rejects selection and releases locks;
already committed installations remain valid but unreferenced. Unknown or
redirected lease directories fail without following links or replacing content.
The parent and file flush behavior has the same platform limits as publication.

This is a cooperative-store foundation. Persistent shell-session references,
operation owner/start-time journals, workspace reference tracking, crash recovery,
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

Currently supported plans use `tar` or `tar.gz`, optional `strip_prefix`,
`payload_subtree: "."` and an empty `executable_paths` array. Other archive kinds,
subtree projection and executable overrides fail explicitly. Entries outside the
strip prefix are rejected, including a prefix ancestor that is not an ordinary
empty directory. Strip-prefix removal does not rewrite symlink targets; the final
contained link graph must remain valid. Required paths are a sorted unique list
of exact payload paths and `file`, `directory` or `symlink` types.

`extraction_bounds` has positive integer `max_entries`, `max_bytes`,
`max_file_bytes`, `max_depth` and `max_expansion_ratio` fields. These may tighten,
but never raise, the existing extractor ceilings. `max_bytes` caps both total
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
