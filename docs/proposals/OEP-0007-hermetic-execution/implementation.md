# Executor and filesystem implementation contract

## Backend contract and initial capability floor

The initial product implementation targets a Linux isolated executor and a BuildKit image adapter. Windows and macOS CLI hosts connect to a local Linux virtualized/container executor; they are supported hosts only after that transport and lifecycle pass conformance. This does not imply native Windows/macOS build targets. Native target executors must satisfy the same capability contract before advertisement. No runtime is silently installed, no Docker daemon socket is mounted into application actions, and no host fallback occurs when the executor is unavailable.

Define an internal Executor interface: `capabilities()`, `prepare_workspace(input_tree, mounts)`, `execute(action, cancellation)`, `collect_outputs(output_contract)` and `destroy_workspace()`. All operations return typed errors and bounded streams. Capabilities include execution platforms, isolation implementation/version, network-denial enforcement, read-only inputs, process-tree containment, CPU/memory/time limits, UID separation, safe filesystem metadata, and supported service-network scopes. The planner tests required capabilities, not a single boolean named hermetic.

A local administrator or hostile kernel can alter local evidence. Record local executor assertions as local observations. Production authority requires externally verified execution identity and controls. The same builder/action implementation is used in both cases.

## Captured trees and mount map

Input trees are content-addressed manifests of relative paths, type, content digest/size, executable bit and permitted internal symlink target. Ignore mtimes, UID/GID and incidental inode numbers. Sort UTF-8 path bytes after requiring normalized portable paths; reject nonportable path/case collisions rather than silently renaming. Symlinks must resolve inside the captured tree; hardlinks are represented as independent files sharing a blob digest. Devices, sockets, FIFOs, setuid/setgid and host ACL propagation are rejected. Preserve executable intent explicitly even when invocation host filesystem lacks Unix mode semantics.

Use stable logical paths `/workspace`, `/tools`, `/inputs`, `/out`, `/tmp` inside Linux actions; adapters translate these to supported native-executor paths later. Source and captured dependency stores are read-only. Managers that require mutable state receive a private copy-on-write worktree/cache initialized from captured content, never the user's live home cache. Outputs are captured from declared areas after the process tree exits. Compile-generated files inside a writable worktree are permitted only within declared boundaries and are not written back to source.

Capture rejects source changes during read by checking metadata/content and retrying a bounded snapshot pass (three attempts); persistent mutation fails. After capture, further source edits cannot change the running plan. Output capture checks for concurrent writers and terminates lingering child processes first. Materialized content is copied/reflinked only with consumer-private write semantics; mutable hardlinks to the producer store are forbidden.

## Environment and network

Construct environment from builtin adapter requirements, effective nonsecret project env and declared CLI values. Exclude HOME credential directories, cloud variables, Git credentials, agent sockets, Docker sockets and inherited proxies. Supply an empty action-local HOME. Build tools resolve only through the locked tool tree. Process startup may need OS variables; these remain executor bootstrap inputs and cannot expose user secrets.

Default action network has no external route, DNS, host loopback or broker route. A native `--offline` flag is additional behavior, not enforcement. Tool/dependency acquisition occurs in a different sandbox via a broker session restricted by source and package authorization. Installation scripts do not inherit upstream tokens. Explicit service-test networks are isolated per run and record endpoints/capabilities; an external-service action has weaker evidence and policy decides eligibility.

Normalize locale to C.UTF-8 where supported, timezone UTC, umask 022, deterministic archive metadata, and SOURCE_DATE_EPOCH to the captured commit time (zero for source without a commit). Record any adapter override. Network isolation does not eliminate time/randomness effects; reproducibility remains a separate repeated-output assertion. Disable interactive terminals for build actions; stdin is closed except explicit bounded protocol input.

## Secrets, export and cleanup

Normal dependency builds have no upstream secret mounts. Exceptional approved secret actions receive broker handles at dispatch and are noncacheable by default; their values are excluded from plan/hash/logs and expire when the action/session ends. These actions cannot claim credential exclusion merely because a secret mount was used. They require separate output controls and evidence. Cleanup destroys mounts, closes leases, terminates children and removes session material on success/failure/cancel. Crash recovery reconciles abandoned process leases before deletion.

Validate every output path with no-follow filesystem operations to prevent validation/use races. Enforce per-action file-count/byte limits supplied by descriptor or policy; initial defaults 100,000 files and 10 GiB, overridable before execution. Decompression limits apply to acquired inputs as well as outputs. Reject credential-bearing config files in source contexts when known; no content scanner is represented as a proof against arbitrary exfiltration. Secret detection failure blocks export and deletes or quarantines affected staged cache entries according to retention policy.

## Verification and capability gates

EXEC-01–07 plus adversarial home/registry access, symlink races, descendant processes, socket mounts, IPv4/IPv6/DNS egress, alternate package endpoints, secret-canary observation, disk/memory exhaustion, output collisions and cancellation. Run probes on Linux and both virtualized host transports before claiming cross-host support. First implementation can fail unsupported capabilities explicitly; silently weakened isolation is never an acceptable first slice. The precise runtime library/transport is a gated prototype choice, with this contract as its pass/fail test.
