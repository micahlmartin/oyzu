# Private dependencies without credentials in build outputs

Status: design contract for review. EX-051 through EX-058 extend the existing hermetic-execution and local-broker examples. They do not implement a registry, package resolver, container frontend, or policy server.

## One boundary, native adapters

The connector owns upstream authentication and allowed repositories. An executor-side agent/broker acquires directly from approved upstreams, including existing Artifactory or Nexus; the platform is not a mandatory byte relay. Package-manager adapters preserve native resolution, registry routing, lock semantics and prepared-store formats. Builders compose these adapters; application code and dependency scripts never receive upstream credentials.

Preparation may run native resolution code, including dynamic metadata discovery, in a constrained environment. Broker access is scoped to the job and approved routes. It must not become a shared unauthenticated proxy. If a local capability is needed, it is itself sensitive and ephemeral: no cache, layer, manifest or persistent configuration may retain it. Details of authenticating container/remote sessions remain open; no assumption that localhost inside a container reaches the workstation agent is permitted.

Capture the complete dependency closure, including build plugins, backend requirements, native toolchains, lifecycle assets and OS package installation inputs. Freeze content identities and target compatibility before isolated execution. Mutable tags, version constraints, repository snapshots and absent lockfiles are preparation inputs, not proof of reproducibility. Preserve native lockfiles; if a manager lacks complete locking, record the captured graph and content digests in the plan. Policy may require committed locks and reject their absence.

Execution receives credential-free captured content, minimal nonsecret configuration, and enforced network denial. Offline flags assist native tools but are not the security boundary. Unknown dynamic downloads fail with the package/task and missing-input reason; no automatic internet retry. Private cached content still requires authorization, including when a policy permits bounded offline reuse. Credentials and their rotation are not content identities or cache keys.

## Two profiles, the same projects

Standalone users select registry routes once in user configuration and authenticate through the headless agent. Managed users receive approved routes from central policy; logout, denial, or outage cannot enable direct fallback. Projects contain native package metadata, not authentication plumbing. User-level route syntax and the public synthetic policy schema are not invented by these examples.

The same native project is reviewed as a native build and as an input to container packaging. Host, execution and target platforms remain distinct. A credential-free image is not automatically production-eligible: local-origin restrictions and independent evidence requirements still apply. Publishing uses separately scoped authorization after the build; acquisition rights never imply publication rights.

## Coverage

| Examples | Native variants and additional inputs |
| --- | --- |
| [EX-051](dependencies/python-private/README.md) | uv, pip, Poetry, legacy metadata; source distributions and build requirements |
| [EX-052](dependencies/node-private/README.md) | npm, pnpm, Yarn; lifecycle scripts, native add-ons and Git sources |
| [EX-053](dependencies/java-private/README.md) | Maven, Gradle, Ant with Ivy; plugins, processors, wrappers and toolchains |
| [EX-054](dependencies/go-private/README.md) | Go modules; proxy fallbacks and private checksum routing |
| [EX-055](dependencies/rust-private/README.md) | Cargo; alternate registries, proc macros and build scripts |
| [EX-056](dependencies/helm-private/README.md) | Helm OCI and HTTP repositories; dependency acquisition versus publication |
| [EX-057](dependencies/container-os-packages/README.md) | apt and apk; signed metadata and maintainer scripts |
| [EX-058](dependencies/docker-credential-boundary/README.md) | Custom Dockerfile integration; image layers, logs and intermediate-cache leaks |

This is an extensible adapter obligation, not a claim that every package manager or arbitrary installation script already works. Additional managers must demonstrate the same boundary before support is advertised.

## Custom Dockerfiles and secret escape hatches

The positive path supplies prepared dependency files or a prepared runtime, with no upstream credential mounts. The dependency-context binding in EX-058 is proposed; ordinary producer artifact materialization still follows [MATERIALIZATION.md](MATERIALIZATION.md). The apt/apk examples intentionally expose the need for a supported preparation integration rather than silently rewriting arbitrary shell commands.

BuildKit secret mounts are available for exceptional approved secret-dependent actions; they are not the universal package installation strategy. They do not prevent the command from copying or printing the secret. Such actions require explicit scope, accurate evidence and established cache rules, and cannot claim the credential-exclusion property of the normal path. A networked preparation action is not represented as hermetic execution.

The future harness uses an ephemeral synthetic upstream canary and inspects process-visible input, full image layers, history/config, intermediate/exported caches, logs, reports and provenance. Deleting a secret in a later layer does not repair an earlier layer. Detection blocks export but does not prove arbitrary encoded exfiltration impossible; architectural isolation is the primary protection. No real credentials are checked in or needed for these examples.

## Review questions

- What is the smallest explicit binding for prepared package-manager inputs in a custom Dockerfile, without a new scripting language?
- Which manager versions and dynamic-download integrations are supported in the first slice?
- How are preparation sessions authenticated on each local/virtualized/remote executor without exposing upstream credentials to project code?
- How should generated dependency locks be proposed for review when policy requires them to be committed?

Native dependency locks, OCI digests and registry responses will be generated against a future synthetic fixture service. They are not fabricated in the authored examples. See [VERIFICATION.md](VERIFICATION.md) for the distinction between structural checks and behavior.
