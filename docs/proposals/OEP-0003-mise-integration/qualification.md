# Expanded mise qualification

Status: authorized work in progress; no new backend or enforcement claim yet.
This extends the completed [Node experiment](experiment.md) following the
maintainer request to qualify broader backends, macOS and production broker/
network enforcement. Existing passing results remain evidence for their original
scope; they do not satisfy the expanded scope.

## Hypotheses and required evidence

1. The same Oyzu TOML/lock adapter can drive multiple real backend families.
   Exercise core Node, Go, Java and Python distributions, an aqua registry tool,
   a script/plugin backend and a native package-manager backend. Record exact
   backend/plugin/distribution identities, real version output, environment and
   entrypoints, frozen locking, dependency requirements and all outbound paths.
   A failure or unsupported path must remain visible; no replacement executable
   or locally reimplemented installer counts as backend reuse.
2. Native macOS behaves correctly under the same shell/exec/lock tests. Run on
   an actual macOS host, recording OS, architecture, compiler, source/binary
   hashes and artifacts. Linux Zsh, cross-compilation and workflow YAML alone
   are insufficient. Extend platform cases to argument edge cases, working paths
   with spaces, symlinks, process termination and installation concurrency.
3. The actual public `oyzu::broker` implementation can supply backend metadata,
   archives and verification material while upstream credentials stay on the
   host. Link the production crate; do not replace its decisions with a synthetic
   broker. Controlled origin servers may provide real archives and test-only
   credential canaries. Verify source-prefix restrictions, redirect
   reauthorization, error sanitization, request/byte limits, unavailable routes,
   denied cached selections and tampered content.
4. The production executor's network boundary contains backend and child-process
   acquisition. Run the real backend inside the enforced worker with a private
   broker channel. Attempt direct IP, DNS, proxy-environment, redirect, alternate
   client and spawned-process bypasses, and attempts to read host-only canaries.
   Unsupported native enforcement must fail closed, not become an online retry.
   Distinguish the host OS from the worker OS and report each tested combination.
5. Connect these layers end to end: authorized acquisition succeeds through the
   real broker, frozen execution succeeds without acquisition, and denial cannot
   silently switch source or lock identity. Preserve standalone operation,
   Oyzu-owned files and the prohibition on a separate mise executable.

## Execution boundaries

The public repository already contains a source-scoped broker and an offline
Docker executor. Their code and tests are the implementation under qualification.
Private platform authorization is a separate service boundary; this work must not
publish private platform details or substitute a local flag for server-side
authorization evidence. Any unavailable service or host remains an explicit gap.

Use an isolated GitHub Actions workflow for native macOS and retain its run and
artifact identities. Keep experimental drivers under `tooling/mise-experiment`.
Keep ecosystem-specific behavior outside the shared production engine. Necessary
production corrections require focused regression tests and the repository's
Rust/task/documentation checks. No design is marked accepted by a passing test.

## Completion record

Pending: backend matrix and source inventory; macOS runtime evidence; real broker
integration; enforced acquisition/attack matrix; final cross-platform regression
and evidence audit. Completion requires actual outcomes for every item above,
not merely adding test scripts or CI configuration.
