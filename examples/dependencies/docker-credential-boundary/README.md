# EX-058: Custom Dockerfiles consume prepared dependencies without credentials

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `project`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved registry mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review the prepared dependency context, the normal Dockerfile and then the negative variants. Named-context binding remains a proposal; running this Dockerfile without that binding is not an approved acquisition path.

## Inputs Oyzu must prepare

- Python dependency wheels and any build dependencies, using the Python adapter.
- Resolved base image and Dockerfile frontend digests, plus an isolated named context containing dependency files only.

The Dockerfile uses standard BuildKit syntax. `dependencies` is a proposed Oyzu-supplied named context, not yet agreed configuration or implemented automatic detection. Its binding must become explicit and reviewable in the plan; no new YAML syntax is introduced here. This complements the agreed materialize contract without pretending dependencies are already exported artifacts. The engine must enforce network denial; the pip flag alone is insufficient. Ordinary COPY operations retain their normal meaning. The Python example is a concrete custom-Dockerfile illustration of the boundary shared by EX-051 through EX-057, not a Python-only feature. Remote and virtualized executors need their own scoped acquisition session; container localhost is not assumed to be the workstation agent.

## Failure and variation cases

- **arg-or-env-secret:** Use variants/Dockerfile.arg with an ephemeral synthetic credential. Expected: Reject credential delivery through ARG/ENV before export; never record its value in a plan or diagnostic.
- **copied-config:** Use variants/Dockerfile.copy with a synthetic credential-bearing .netrc in the context. Expected: Reject context capture; ignore files are convenience, not the enforcement boundary.
- **secret-copy-then-delete:** Execute variants/Dockerfile.secret-copy in an explicitly isolated negative-test harness with only a synthetic canary. Expected: Demonstrate the canary survives in an earlier layer; detect it and prevent export of images and intermediate cache. Normal builds never receive the upstream credential.
- **secret-log:** Use variants/Dockerfile.secret-log with a synthetic canary only. Expected: Treat log disclosure as a failure. Secret mounts are not a guarantee against commands printing or copying their contents.
- **target-mismatch:** Use amd64 dependency wheels for the arm64 consumer. Expected: Reject the context identity before image assembly.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-07, BUILDER-08, BUILDER-05, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.
