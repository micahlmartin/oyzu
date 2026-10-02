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
receipt/publication operations are not implemented yet.

## Archive materialization foundation

The Rust library's `tools::materialize_archive` accepts an archive source, an
empty operation-owned staging directory, the expected SHA-256 and exact byte
size, and whether the archive is gzip compressed. This is a store implementation
boundary, not a CLI install command or a stable external plugin interface.
It does not acquire bytes, authorize a backend, execute anything, write a receipt
or publish an installation. Failed staging must be discarded by the caller.

Source and staging paths must have physical, non-symlink ancestors. The source
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
data; no shell is invoked. A versioned external JSON Schema remains outstanding.

This verifies receipt/content binding, not receipt authenticity or compatibility
with an admitted layout plan. A caller must separately check backend/layout
admission, verification evidence, policy, leases and current authority before
using the data. A self-consistent altered receipt and tree is not an authorized
installation. Publication and lease management remain separate unfinished work.
