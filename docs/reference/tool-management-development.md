# Node tool-management development integration

This opt-in implementation connects standalone Node installation and execution
through Oyzu configuration, the maintained mise Rust library, format-2 locks and
Oyzu's installation store. It is under active functional acceptance testing;
it is not a distribution-qualified release. The maintainer authorized development
import and reserved hardening for future goals.

Build with Rust 1.95 or newer:

```sh
cargo build --locked --features mise-integration --bin oyzu
```

The dependency is pinned to oyzuai/mise revision
`9290bcac695c8ff8a56760ccebd785d5062b459c`; defaults are disabled and the selected
features are `rustls` and `vendored-lua`. A fresh instance of the Oyzu executable
owns mise's process-global state. No separate mise executable is used. The default
Oyzu build does not expose these commands yet.

## First user flow

Create `oyzu.toml` at the workspace root:

```toml
[tools]
node = "22.15.0"

[env]
APP_MODE = "development"
```

Then run the feature-enabled binary:

```sh
oyzu install
oyzu exec -- node --version
oyzu exec -- node -e 'console.log(process.env.APP_MODE)'
```

The shell quoting in the last example is suitable for Bash and PowerShell.
`-C DIRECTORY`, explicit root and profile selection use the existing configuration
resolver. Initial installation supports the workspace root and exactly `tools.node`.
The Node selector is interpreted by mise against the actual Node catalog. Existing
locks retain the exact version; this first implementation has no update command.

The default store is `.oyzu/tools` relative to the invocation directory. For two
projects sharing installations, pass the same absolute `--store PATH` to both
`install` and `exec` (before `--` for exec). Commit `oyzu.lock`; keep the store out
of version control. Each project selects its own locked version when executed.

Installation requests metadata and the archive through Oyzu's existing HTTP
fetcher, currently with the public `https://nodejs.org/dist/` route. It checks the
archive against Node's declared SHA-256, derives the layout from mise, stages and
publishes the installation, then commits the lock through the existing lock-edit
transaction. The lock explicitly records digest-only verification; it does not
claim a verified publisher signature. An existing incompatible lock fails rather
than being silently replaced. A failed installation does not publish a new lock.

Execution requires an existing matching lock and installation. It reuses frozen
selection and a verified installation lease, launches the native Node executable
directly, passes arguments without a command shell, applies configured environment
values and prepends the installed binary directory to PATH. The lease is held
until the direct child exits. Its exit status is returned. Exec performs no implicit
installation and its metadata-facts lookup is local; initial install requires the
network, including repeat installs in this first implementation.

## Scope and acceptance evidence

Host target mapping currently covers Linux amd64 GNU, Windows amd64 MSVC and
macOS arm64. Mapping is not native acceptance evidence: the first integrated build
and test run passed on Linux amd64 under Rust 1.95 on 2026-10-02. The real acceptance runner passed both versions, shared-store project switching, arguments, cwd, TOML environment, unchanged locks during exec and exit status. Windows/macOS integrated acceptance remains pending.

Run the real user acceptance scenario with Python 3.11+ and public Node access:

```sh
python tooling/test-tool-management.py --cli PATH_TO_FEATURE_ENABLED_OYZU
```

The runner creates two projects selecting Node 22.15.0 and 22.14.0, installs into
one store, checks frozen version selection, literal arguments, cwd, TOML
environment and exit-code propagation, and revisits both projects. These are
actual downloads and executions, not mocked backend or store behavior.

Remaining functional work includes other tools, aliases/native constraints,
multi-tool and scoped updates, offline cached install, configured corporate
transport, managed selection, npm entrypoints, shims, activation and build handoff.
Managed configuration is explicitly rejected by this standalone proof. Child
stdio currently carries internal metadata; authenticated worker IPC, full process
tree cleanup and further hardening are deferred. Transitive notices/SBOM and
distribution approval remain separate from the authorized development import.
