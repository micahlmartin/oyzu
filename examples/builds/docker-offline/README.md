# EX-026: Multi-stage image requiring no external base

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Build a data-only OCI image without network or downloaded base layers.
- scratch is Docker's empty base, not an unpinned remote tag.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
docker build --network=none --tag oyzu-example-data:local .
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **external-add:** Use the negative Dockerfile. Expected: Reject undeclared external acquisition before a hermetic action.
- **image-aliases:** [variants/image-aliases](variants/image-aliases/Dockerfile) uses a provisioned image through short and qualified references in `FROM`, external `COPY` and an image-backed `RUN` mount. Identical captured inputs share one native context; conflicting identities fail. Copied content enters the output, while the temporary mount does not.
- **provisioned-base:** Use [variants/provisioned-base](variants/provisioned-base/Dockerfile) with `alpine:3.22` already present in the executor's Docker image store. Preparation captures that image by immutable identity; the isolated build consumes its OCI store offline and emits a snapshot image. A missing base fails preflight without a pull. See [Docker image inputs](../../../docs/reference/docker-images.md) for scope and limitations.
- **quality-gates:** Introduce noncanonical spacing or a relative WORKDIR. Native format/lint checks fail and block final artifacts without changing source.
- **ignored-quality-config:** Exclude target-local Hadolint/EditorConfig files with `.dockerignore`. Checks must still use them while image layers omit them. See the [Dockerfile quality reference](../../../docs/reference/docker-quality.md) for native defaults and provisioning.

## Contract and limitations

Acceptance criteria: EXEC-01, BUILDER-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.
