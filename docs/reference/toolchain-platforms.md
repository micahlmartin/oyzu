# Provisioned toolchain platform selection

Captured builds can select a matching provisioned execution image for an explicit or inferred artifact platform. Builders declare their execution requirement through an internal contract. Language builders require matching OS/architecture; the Docker assembly builder can use a different worker for operations that do not execute target code. This adds no project configuration or tool installation.

## Selection and provisioning

After matrix expansion, Oyzu resolves the builder's usual runtime-specific image and inspects its immutable identity and platform. If its platform satisfies the execution requirement, that image is used. Otherwise, for a default tagged image, Oyzu inspects one platform sibling named `<default-reference>-<os>-<arch>`. The sibling must actually have the requested platform; a matching tag is insufficient. Both the default and any needed sibling must already be available in the selected Docker daemon. There is no pull, image build or emulator installation in this selection path.

Examples of currently provisioned CI siblings:

| Default image | Linux ARM sibling |
| --- | --- |
| `oyzu-toolchain/go:1.24-mod0.25.0` | `oyzu-toolchain/go:1.24-mod0.25.0-linux-arm64` |
| `oyzu-toolchain/node:npm11.11.0-node22` | `oyzu-toolchain/node:npm11.11.0-node22-linux-arm64` |

The same suffix rule follows an exact Node runtime selection. Other ecosystem images are eligible for the common resolution rule but their platform-specific native integrations still need provisioning and verification. The current executor accepts Linux images only.

For example, after explicitly provisioning Docker and any required ARM execution support:

```text
docker build -f tooling/images/go.Dockerfile -t oyzu-toolchain/go:1.24-mod0.25.0 .
docker build --platform linux/arm64 -f tooling/images/go.Dockerfile -t oyzu-toolchain/go:1.24-mod0.25.0-linux-arm64 .
oyzu build image
oyzu inspect dist
```

The authored [Go container matrix](../../examples/builds/container-variants/project/build.yaml) requests two image platforms; materialization infers two matching Go producers. Each executes its language build, test and quality commands in its selected toolchain. Docker can then assemble both scratch images on its existing worker, and the engine creates their complete [OCI index](oci-indices.md).

`--image manager=reference` is an explicit override for all selected variants using that manager. Oyzu does not rewrite it or try siblings. An override with the wrong platform fails before preparation; one image generally cannot satisfy both sides of a native platform matrix. Omitted platforms retain existing defaults. Development `oyzu run` commands continue to use host tools and do not select container images.

## Evidence, failures and limits

Plans and dependency records bind the actual selected image identity and execution platform independently of the artifact target and CLI host. Go preparation additionally verifies native Go OS/architecture against the image. Successful image inspection alone does not establish that commands can execute: preparation/actions must run successfully under the existing sandbox and timeout limits. Unsupported execution or a missing/mislabeled sibling fails instead of silently using the wrong architecture. Provision the required image/execution support or correct an explicit override before rebuilding.

These records do not attest physical native hardware versus emulation, discover remote worker pools, or authorize production publication. The Docker host must supply any emulation; Oyzu does not infer permission to install it. [Docker documents](https://docs.docker.com/build/building/multi-platform/) native nodes, emulation and cross-compilation as different strategies. Executing target-language tests in an ARM image is required here; simply cross-compiling an ARM binary does not replace those tests. Dockerfiles with foreign-platform RUN remain separately restricted by [Docker admission](docker-images.md#artifact-target-and-worker-platform).

Toolchain selection itself uses only the local daemon. Existing native dependency acquisition and offline build replay retain their own rules. Source snapshots, private target workspaces, report gates and failure-bundle retention are unchanged. Cross-platform ABI/base compatibility, automatic platform-independent output reuse, policy-controlled worker pools and comprehensive platform coverage remain unfinished.

## Verification

Rust tests exercise matching/default selection, one verified sibling, explicit override preservation, missing/mislabeled siblings and the distinction between language execution and Docker assembly. The Linux Docker CI job separately provisions ARM emulation through a pinned setup action and builds matching Go/Node toolchain images, checking actual Go and Node architecture before invoking Oyzu.

The captured suite now runs the authored EX-027 Go and EX-050 frontend platform graphs, requiring native-language JUnit/coverage and quality gates for both architectures, Go ELF architecture checks, exact materialized file/directory bytes, complete indices and repeatable plans/artifacts. Node fixture formatting is explicit setup before the immutable source check; builds do not rewrite source. These new native CI results remain pending. This does not claim all cases in either scenario, especially managed policy, ABI and cache requirements, have passed.
