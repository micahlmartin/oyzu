# Directory artifact integrity and materialization

The experimental build engine can collect, inspect and materialize a declared directory artifact. The [Node application adapter](node-applications.md) declares directory outputs for conventional Vite builds and explicit `node/app` custom builds using the `dist/` convention. Platform matrices now expand matching producer/consumer variants, but suitable target execution or established platform independence remain required before the full [directory-materialization example](../../examples/builds/materialize-directory/README.md) can pass. This does not infer directory outputs for arbitrary unclassified build scripts.

## Record and identity

A directory artifact retains the normal artifact identity, target, producing action, version and relative bundle path. Its `kind` is `directory`. In addition:

- `entries` is a complete sorted inventory of every contained file and directory, including hidden files and empty directories. Paths are relative to the artifact root; the root itself is omitted.
- Every entry has `path`, `kind`, `size` and `executable`. Files additionally carry their SHA-256 `digest`. Directories have zero size, false executable intent and no file digest.
- Artifact `size` is the sum of file bytes. Its `digest` is SHA-256 over `oyzu.tree.v1alpha1`, one NUL byte, then the JCS-encoded entries array. It is a logical tree identity, not an archive checksum.

This uses the engine's existing tree encoding and includes directory entries explicitly, including nonempty directories. Relative paths determine order; timestamps, absolute checkout paths and user/group IDs do not enter the hash. Collection uses the complete tree rather than source exclusions: names such as `dist`, `target` and `.hidden` remain artifact content. The [manifest schema](../contracts/v1alpha1/manifest.schema.json) requires the inventory for directories and rejects it on other artifact kinds.

## Inspection and copying

`oyzu inspect <bundle>` reads the actual directory without changing it and compares its inventory, total size and tree digest to the manifest. Changed bytes, extra or missing entries, missing empty directories, mismatched inventory metadata and invalid contained paths fail inspection. No tools, project scripts or network services run during inspection. Integrity verification does not authenticate a producer or establish release eligibility.

When a directory output is selected by `materialize`, its contents appear immediately beneath the configured destination. An artifact containing `index.html` copied to `site` becomes `site/index.html`, without an extra output-directory layer. The producer must have succeeded. The engine verifies its recorded content before copying and verifies the new copy afterward. Each consumer gets independent bytes; editing them cannot alter the producer bundle. Existing destinations, overlapping mappings and collisions with captured source fail. Producer test/coverage references retain their original scope.

Directory handling does not establish platform independence. Compatibility requires matching producer and consumer artifact target OS/architecture, independently of their worker platforms. Consumer requirements propagate backward through materialization to matching producer variants, as described in [matrix selection](runtime-matrices.md#platform-requirements-and-producer-selection). Cross-platform reuse requires additional builder facts and selection rules; arbitrary JavaScript-generated files do not receive that classification automatically. See [Docker target selection](docker-images.md#artifact-target-and-worker-platform) for the current distinction and limits.

## Limits, failures and verification

Traversal rejects symlinks, nonregular file types, nonportable names and case-colliding paths. It is bounded to 100000 entries and 10 GiB of file contents per tree. File changes detected during inventory/copy fail the operation. Failed capture can leave partial unrecorded output; that output is not a valid artifact and must not be reused as successful evidence. Restore an intact bundle or rerun the producer after resolving the underlying error.

Unix executable bits participate in inventory and copying. Windows currently captures files with `executable: false`; transferring Unix executable directory trees to Windows without preserving their mode intent is not supported by this implementation. Static data directories with no executable files avoid that mismatch. General portable executable metadata remains required work.

Rust tests exercise complete capture, stable identities, hidden/output-named content, empty directories, changed/added/missing content, inventory tampering, independent flattened copies, failed producers and overwrite rejection. Unix CI additionally checks links and executable-bit changes. Public `build::inspect` and compiled-CLI integration tests verify directory bundles without going through private adapter APIs. Windows checks passed locally; all three CLI jobs at ea204c8 passed, with Linux job 110779088872 explicitly recording the directory integrity, executable/link and public-inspection checks. Schema fixtures cover required inventories, duplicate entries and missing file digests. These checks do not prove a native builder emits directory artifacts or that EX-050's two-platform Docker build works; native output acceptance is tracked separately in [Node applications](node-applications.md).
