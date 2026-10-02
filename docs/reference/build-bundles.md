# Build bundles and retention

`oyzu build` finalizes its results under the workspace's `dist/`. The bundle contains the manifest, frozen plan when planning succeeded, execution envelope, native logs, retained reports and collected snapshot artifacts. A failed build can still produce a useful bundle; consult `manifest.json` for actual outcomes. `oyzu inspect dist` checks recorded content integrity and does not establish trusted CI identity or release eligibility.

When a frozen plan exists, inspection also requires matching manifest target records and declared artifact target/variant identities. Relabeling a runtime variant in the manifest without changing the corresponding plan is rejected. Runtime-specific reports remain attributable through their concrete target records; see [runtime matrices](runtime-matrices.md). This consistency check does not authenticate either record.

For an OCI image artifact, inspection additionally verifies that the archive's actual OS/architecture matches its target record. Collection enforces the same check, including after custom test or packaging tasks. A valid archive digest alone cannot establish that the image was built for the requested platform. See [Docker target selection](docker-images.md#artifact-target-and-worker-platform); structural image checks do not prove application execution on that target.

## Workspace ownership

An invocation holds an operating-system file lock on `.oyzu/build.lock` through staging and finalization. Another cooperating build in that workspace fails before execution with a workspace-lock diagnostic. The existence of the lock file alone does not indicate a live build: the operating-system lock is authoritative and is released when its owning process exits. Different workspace roots have independent locks.

The engine creates its staging directory under `.oyzu/bundle-*` on the destination filesystem. It admits an existing `dist` only when it is a directory containing an Oyzu build manifest. A preexisting application-owned `dist` must be preserved or moved by its owner before building; Oyzu does not delete it. State, lock, bundle marker and destination paths reject symbolic links, including dangling links, and Windows reparse points. History paths receive the same checks before use.

The invocation records the existing manifest's content identity, or the absence of a destination. Immediately before finalization it rechecks that state. A newly created destination, changed manifest, invalid destination or existing history slot stops finalization instead of overwriting it. This is a change check, not a proof that every byte in the older bundle remained unchanged.

## Finalization and recovery

When records are finalized, the previous `dist` is moved to `.oyzu/history/<new-run-id>`, and the staged directory becomes `dist`. No reports or artifacts are appended to the old manifest. If the second rename fails, Oyzu attempts to restore the prior bundle. If restoration also fails, the diagnostic identifies the history path containing the previous bundle.

A finalization error retains the new staging directory and includes its exact path in the error. Inspect that path directly when its records are complete:

```text
oyzu inspect .oyzu/bundle-EXAMPLE
```

Use the path from the diagnostic, not the literal example name. Preserve the conflicting destination and resolve the filesystem problem before rebuilding. Retained staging directories are not automatically considered successful builds or reused as input. An ordinary abandoned staging scope, such as `--plan` completing without a bundle, is cleaned up. Failures while writing records, abrupt termination and full disks can leave incomplete evidence; a retained directory alone does not prove a valid bundle.

The filesystem publication step does not contact a registry or require network access. It adds no configuration or UI dependency. `--plan` still performs the existing image/dependency preparation but never replaces `dist`.

## Verification and limits

Rust tests cover lock contention, first/repeated finalization, history retention, ordinary staging cleanup, changed or newly created destinations, history collisions, invalid run IDs, incomplete staging records and retained failure evidence. Windows tests create real directory junctions and verify rejection without modifying the target. Unix symlink checks run in the Linux/macOS test jobs. The existing captured-build suites exercise the same lifecycle with native artifacts, repeated builds and failed-build reports; consult [implementation status](../implementation-status.md) for revision-specific evidence.

The two renames are not a power-loss-safe transaction. There is no automatic journal recovery if the process or machine stops between them. Path checks and locks also do not make the store race-free against a hostile process with permission to replace directories. Inspection verifies recorded outputs separately from these destination checks. Remote storage and automatic history retention limits remain unfinished.

The [direct Node/Python/Go test workflow](direct-tests.md) reuses this transaction with host output preservation. Unlike captured builds, it can admit native application output in `dist`, copy that output into the new test-only bundle and retain the original directory in history. These bytes have a separate integrity inventory and are not new build artifacts. It removes prior engine report/evidence claims from the current bundle, preserves their original bytes in history, and rejects reserved-path collisions. Other direct test profiles remain to be integrated.
