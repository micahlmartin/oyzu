# Runtime matrix builds

Experimental captured builds can expand a finite Node runtime matrix for npm, pnpm and Yarn Classic. A single project definition supplies the tasks, overrides and hooks for every variant. Other language runtimes, platform matrices, cross-compilation and automatic runtime selection remain unfinished. This does not install tools.

## Configure and provision

For an application with a native build script producing `dist/`:

```yaml
app:
  uses: node/app
  matrix:
    node: ["22.14.0", "24.14.1"]
```

Use `node/package` for native package archives. `node/app` retains its existing framework/output inference; see [Node applications](node-applications.md). Runtime versions must be exact numeric `major.minor.patch` values. Ranges, aliases and multiple language axes are not accepted. Native `package.json` engines still constrain every requested runtime, and the provisioned native manager must match any declared `packageManager`. The manager is inferred from native metadata, not another matrix axis or Oyzu setting. Existing manager restrictions still apply, including the single-project [pnpm](pnpm.md) and [Yarn Classic](yarn.md) dependency profiles.

The current executor needs provisioned Linux Docker images and the existing sandbox capabilities. Default image names follow the selected manager:

| Native manager | Provisioned image for each requested Node version |
| --- | --- |
| npm 11.11.0 | `oyzu-toolchain/node:npm11.11.0-node<VERSION>` |
| pnpm 10.11.0 | `oyzu-toolchain/node:pnpm10.11.0-node<VERSION>` |
| Yarn Classic 1.22.22 | `oyzu-toolchain/node:yarn1.22.22-node<VERSION>` |

CI provisions the two Node versions above using the existing quality and manager Dockerfiles with `NODE_IMAGE` and `QUALITY_IMAGE` build arguments; see [the acceptance provisioner](../../tooling/build-scenario-tools.sh). Provisioning is explicit test infrastructure, not a download performed by a build action. A missing image fails normally; provision the image before retrying. Windows/macOS host-side checks do not establish isolated execution on those operating systems.

```text
oyzu run list
oyzu build app --plan
oyzu build app
oyzu inspect dist
```

Discovery and `run list` retain logical task names such as `app:test`. A development `oyzu run app:test` uses the active host runtime; it does not fan out or switch runtimes. The captured build expands the matrix and shows concrete variant IDs in its plan. `--plan` includes native preparation and runtime admission but executes no application build/test actions and does not replace `dist`.

## Identity and dependency behavior

Variants receive deterministic, portable target IDs derived from the logical name, axis and version, with a digest suffix when normalization is needed. Ordering the YAML matrix values differently does not change these IDs; changing the file still changes source identity. Expansion rejects case-insensitive ID collisions and bounds the expanded workspace to 256 target instances and 16,384 copied tasks. Plans are also limited to 16,384 actions.

Each variant has its own prepared dependencies, mutable source workspace, toolchain identity, actions and output paths. The target and artifact records contain `variant: {node: "<VERSION>"}`. Reports identify their concrete target and producing action; resolve that target's variant in the same manifest. Coverage from different runtimes is retained separately and is not summed.

The selection extension records logical requested IDs, concrete selected IDs, and a `variants` mapping. Selecting a logical target requests all its declared variants. Dependencies are then recomputed against the expanded graph: common runtime axes must match. A producer without a runtime axis remains a single producer. Ordering-only dependencies can wait for multiple compatible producer variants; materialization requires exactly one compatible producer and rejects ambiguity. Runtime matching does not establish platform/ABI compatibility or platform-independent output.

Bundle inspection checks that manifest target records match the frozen plan and that each retained artifact has the plan's target/variant identity. Changing a manifest's runtime label without changing its plan is rejected. This verifies internal consistency and recorded content; it does not authenticate a producer or authorize publication.

Qualified tasks and their hooks are copied per variant. In a workspace originally containing one logical target, unqualified root overrides retain precedence for build stages, while explicitly qualified references keep their separate command and hook family. The graph uses internal `root-` task aliases for those root operations, and preserves the original operation name in action records. Collisions with an existing task using a reserved alias fail explicitly. In multi-target workspaces, shared root tasks retain the existing ownership restrictions described in [build tasks](build-tasks.md).

Expansion uses the frozen build inventory and does not rewrite `build.yaml`. Discovery and structural graph validation still cover the workspace; invalid matrix relationships can prevent a selected build even when an affected target would otherwise be excluded.

## Runtime evidence and failure

All three adapters check the actual `process.versions.node` against the requested version before acquisition. npm uses its native engine validator for the root and workspace members before fetching registry packages. pnpm and Yarn validate engines during their native preparation/install operation, before application actions; registry archives may already have been captured. Oyzu does not substitute npm's engine interpretation for either manager's native rules.

Prepared metadata records the observed runtime in the manager's existing dependency extension (`oyzu.dev/npm`, `oyzu.dev/pnpm` or `oyzu.dev/yarn`). Planning requires the requested version in that evidence, and isolated installation checks that Node and the manager still match the capture. Native patches, resolutions, frozen locks and lifecycle rules retain their manager-owned behavior. A manager-level override such as `--image pnpm=...` applies to every selected target using that manager; it cannot bypass these checks and usually cannot satisfy two different runtime versions.

No application action is scheduled until all selected variants have passed preparation and planning. An unavailable runtime, mismatched image, incompatible engines or unsupported adapter therefore produces a failed bundle without application artifacts. Correct the declaration or provisioned toolchain and rebuild. Existing source and prior bundles follow the ordinary [bundle retention](build-bundles.md) rules.

Once execution starts, a failing variant retains its available reports and cannot package successfully. Independent variants may complete and retain their own snapshot artifacts. The overall build still fails. Distinct runtime builds are retained separately even if their bytes happen to match; this is not automatic publication of multiple releases or proof that the outputs are interchangeable.

## Verification and remaining scope

Rust tests cover deterministic expansion, root/qualified task and hook identity, matching dependencies, ambiguous materialization, collisions, bounds, selected producer closure, separate artifact paths and required runtime evidence for each manager. The native [npm runtime probe](../../tooling/test-node-runtime.py) and [pnpm/Yarn manager probe](../../tooling/test-node-managers.py) check actual Node admission, replay identity and native failures. CI runs them on all three task hosts after compiling the CLI.

[The captured matrix acceptance group](../../tooling/build_scenarios/node_matrix.py) requires both exact runtimes, build/test/lint/read-only-format actions, snapshot directories for EX-032, snapshot package archives for pnpm patches and Yarn resolutions, JUnit/coverage, repeatable plans/output and mismatched-runtime rejection. The npm example also checks incompatible engines before actions and a runtime-specific test failure with separate evidence. Its addition is not itself a passing result; consult [implementation status](../implementation-status.md) for observed CI evidence. Platform matrices, additional managers/languages, variant-aware caching/publication and the complete scenario catalog remain outstanding.
