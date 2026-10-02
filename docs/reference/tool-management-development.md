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
oyzu install --frozen
oyzu install --frozen --offline
oyzu which node
oyzu env --json
oyzu exec -- node --version
oyzu exec -- node -e 'console.log(process.env.APP_MODE)'
```

The shell quoting in the last example is suitable for Bash and PowerShell.
`-C DIRECTORY`, explicit root and profile selection use the existing configuration
resolver. Initial installation supports the workspace root and exactly `tools.node`.
The Node selector is interpreted by mise against the actual Node catalog. Existing
locks retain the exact version unless an update is explicitly requested.

After editing the Node requirement in TOML, run:

```sh
oyzu install --update node
oyzu exec -- node --version
```

Bare `--update` and `--update core:node` select the same configured Node root.
The command resolves through mise, prints the previous and proposed exact version,
installs the result and then commits the lock using the existing compare-and-swap
transaction. It never edits TOML. Ordinary install rejects changed requirements
with `TOOL_LOCK_STALE` and an update remedy. `--update` conflicts with `--frozen`
and `--offline`; unsupported tool names fail. A failed update leaves the previous
lock intact. An unchanged resolution preserves lock bytes.

This first update path supports a single Node selection at the workspace root for
the selected profile and host. Locks containing additional environments or target
platforms are refused rather
than dropping their selections. Other projects sharing the store keep their own
locks and versions. Multi-tool and scoped updates remain to be implemented.

The default store is `.oyzu/tools` relative to the invocation directory. For two
projects sharing installations, pass the same absolute `--store PATH` to both
`install`, `which` and `exec` (before `--` for exec). Commit `oyzu.lock`; keep the store out
of version control. Each project selects its own locked version when executed.

Installation requests metadata and the archive through Oyzu's existing HTTP
fetcher, currently with the public `https://nodejs.org/dist/` route. It checks the
archive against Node's declared SHA-256, derives the layout from mise, stages and
publishes the installation, then commits the lock through the existing lock-edit
transaction. The lock explicitly records digest-only verification; it does not
claim a verified publisher signature. An existing incompatible lock fails rather
than being silently replaced. A failed installation does not publish a new lock.

`install --frozen` requires an existing compatible lock and never resolves a new
version. Both ordinary and frozen installs reuse an already installed selection
after verifying its receipt and contents; this path needs no network and leaves
the lock unchanged. If the installation is absent, Oyzu first verifies the cached
archive against the locked digest and size, then restores the installed tree.
Only a missing archive requires acquisition from the public route; invalid cached
content fails instead of silently downloading a replacement.

`install --offline` forbids metadata and artifact networking. It requires an
existing compatible lock and either a verified installation or its cached archive.
Combine it with `--frozen` to explicitly prohibit lock edits. Missing locks or
archives fail with an online-install remedy. Run ordinary `install` online to
create the initial lock and acquire the required bytes.

`which node` prints the absolute executable path selected by `exec`. It uses the
same frozen configuration/lock matching and verified installation lookup, makes
no network request and does not install or launch anything. A missing installation
or mismatched configuration fails instead of returning a system PATH executable.

Execution requires an existing matching lock and installation. It reuses frozen
selection and a verified installation lease, launches the native Node executable
directly, passes arguments without a command shell, applies configured environment
values and prepends the installed binary directory to PATH. The lease is held
until the direct child exits. Its exit status is returned. Exec performs no implicit
installation and its metadata-facts lookup is local. Initial installation and
acquisition of uncached content require the network.

## Environment inspection and shell application

`oyzu env` and `oyzu env --json` inspect the existing frozen selection and report
the executable and environment variable names. All environment values, including
PATH, are redacted. Missing or stale selections fail without installing anything.
The environment composition is shared with `exec`: literal TOML values plus the
selected binary directory prepended to inherited PATH. An already leading selected
directory is not prepended again. Exec rejects `--json` so application stdout is
not mixed with status output.

To explicitly print assignments with their actual values, select a shell:

```sh
# Bash
eval "$(oyzu env --shell bash)"
node --version
```

For Zsh use `--shell zsh`. For PowerShell 7 use:

```powershell
oyzu env --shell pwsh | Out-String | Invoke-Expression
node --version
```

`--shell` conflicts with `--json`. Rendering uses the pinned mise library's
environment diff and shell assignment quoting in the isolated same-image child.
It performs no network or installation. Values remain literal, including quotes,
dollar signs and shell metacharacters; they are not interpreted as config code.
Rendering requires UTF-8 environment values. Pass the same `--store` used for
installation when using a shared store.

This is a one-time application to the current shell. It does not install prompt
hooks, automatically switch on directory changes or restore old values. Use the
activation commands below when testing automatic transitions instead.
The lookup/rendering lease ends when the command returns; this does not establish
an active-shell retention lease. Those lifecycle capabilities remain unfinished.

## Development shell activation

From an installed project, start a session in Bash or Zsh:

```sh
eval "$(oyzu activate bash)" # use zsh for Zsh
node --version
cd ../another-installed-project
node --version
oyzu deactivate
```

For a shared store, pass `--store /absolute/store` to `activate`. Without that
option each hook looks in `.oyzu/tools` under its current directory. Root/profile
options from activation are retained for the session. The PowerShell equivalent
is `oyzu activate pwsh | Out-String | Invoke-Expression`. The original PowerShell
activation/switching/deactivation scenario passed on Windows at `83049fa` in run
`37050580618`; native acceptance with the newly added shims remains pending.

The pinned mise library renders the prompt/directory hooks and deactivation
script. Oyzu adapts the generated namespace before inserting the real frontend
path through mise's literal quoting. The hooks call this same Oyzu binary, never
a separate mise executable. Automatic command-not-found installation is disabled.
Each hook uses frozen local selection, with no network, installation or project
task execution. Changing directories applies the new project's locked Node and
literal TOML environment. Unchanged selection emits no environment changes.

Oyzu records changed variable values and PATH in a temporary session file under
the OS temporary directory's `oyzu-shell-sessions` directory; the environment
carries its random token. Deactivation removes that file and emits cleanup.
Scalars are restored only when they still equal the value Oyzu applied, preserving
user edits. An unchanged PATH is restored exactly. A still-leading owned insertion
can also be removed while preserving later user additions; arbitrary PATH
reordering and ambiguous duplicate ownership are not fully implemented yet.

A missing/stale/uninstalled selection clears the preceding project's delta,
sets `OYZU_TOOL_STATUS=unavailable` and emits a diagnostic once per changed failure.
A usable selection sets the status to `ready`. The versioned Oyzu shim stays at
the front of PATH while the session is active. Its `node` invocation fails on a
missing selection instead of falling back to a system Node. Deactivation removes
the session's PATH insertion and restores the previous environment.

### Native executable shims

Install and activation provision `node` (`node.exe` on Windows) as a hardlink to
the actual frontend image, with a byte-identical copy when hardlinking is unavailable.
Shims live under `STORE/shims/RELEASE_DIGEST/shared` or `.../cwd`; their adjacent
`oyzu-shim.json` identifies the release, command and store mode, never a project
or locked version. Invocation validates the image/manifest and then uses the
current working directory, configuration, lock and verified installation lease.
Arguments and exit status go through the same native execution path as `exec`.
There is no command-shell wrapper and no implicit download or PATH fallback.

Activation chooses a fixed shared store when `--store` is supplied. Otherwise
the shim resolves `.oyzu/tools` in each invocation directory. Active root/profile
options come from the session; outside a session default configuration selection
applies. Prompt hooks do not provision shims. Re-run install or start a new
activation to provision a new frontend release; existing versioned images are
retained for active shells. Shim pruning and broader command sets remain open.
Deactivate sessions created before this shim support before reactivating them.

The generated `oyzu` shell function evaluates deactivation output. Invoking the
absolute binary with `deactivate` only prints it. No profile file is modified.
Starting another activation in an already active session is rejected; independent
and nested shell lifecycle qualification remains open. Hooks currently verify
installed contents on every call, which can be slow in debug builds. Active-shell
retention, abandoned-session recovery, fully reversible PATH alignment and profile
installation remain unfinished. This is a controlled development integration.

The real Linux Bash/Zsh acceptance passed with networking disabled: activation,
automatic `cd` switching between Node 22.15.0 and 22.14.0, unchanged-hook output,
missing-selection cleanup, user-edited scalar preservation and deactivation.
The subsequent native-shim replay also passed on Linux Bash/Zsh, including literal
arguments and exit status. A direct offline probe put a real Node later on PATH
and confirmed that a missing selection failed instead of invoking it. Native
macOS activation and Windows shim results remain pending. Run it against the
retained two-project workspace:

```sh
python tooling/test-tool-activation.py --cli PATH_TO_FEATURE_ENABLED_OYZU --workspace PATH --shell bash --shell zsh
```

Linux Bash and Zsh application passed with networking disabled on 2026-10-02:
the shell launched the same Node executable as `exec`, literal metacharacters
were preserved, inspection values were redacted and the lock was unchanged.
PowerShell environment application passed on Windows at `3eefcc2` in CI run
`37049209850`; native macOS environment application remains pending. Reproduce
against a retained acceptance workspace with Python 3.11+:

```sh
python tooling/test-tool-environment.py --cli PATH_TO_FEATURE_ENABLED_OYZU --workspace PATH --shell bash --shell zsh
```

Use `--shell pwsh` to require the real PowerShell interpreter. Requested shells
must be installed; the runner does not skip a missing interpreter.

## Scope and acceptance evidence

Host target mapping currently covers Linux amd64 GNU, Windows amd64 MSVC and
macOS arm64. On 2026-10-02, Linux amd64 under Rust 1.95 passed both versions,
shared-store project switching, frozen/repeat installs, which/exec agreement,
arguments, cwd, TOML environment, unchanged locks and exit status. The same
projects then passed replay under Docker `--network none`. A separately mounted
store also passed installation/publication. Removing the installed trees while
retaining their real archives then passed `install --frozen --offline` restoration
and execution for both versions with networking disabled; an empty cache failed
as expected and locks remained byte-identical. The original install/exec scenario
at revision `73bb33d` passed on macOS arm64 and Windows amd64 in CI run
`37045134221`. Frozen/which/restoration then passed on macOS at `01e81be` in
run `37047681781` and Windows at `1acb16f` in run `37048375166`.

Run the real user acceptance scenario with Python 3.11+ and public Node access:

```sh
python tooling/test-tool-management.py --cli PATH_TO_FEATURE_ENABLED_OYZU
```

The runner creates two projects selecting Node 22.15.0 and 22.14.0, installs into
one store, checks frozen version selection, literal arguments, cwd, TOML
environment and exit-code propagation, and revisits both projects. These are
actual downloads and executions, not mocked backend or store behavior.

Use `--workspace PATH` to retain these projects, then rerun with the same path and
`--offline-check` in an environment with network access disabled. The replay
checks frozen and ordinary install reuse, `which`/`exec` executable agreement,
unchanged locks and both project versions. `--offline-check` itself does not
disable networking; the surrounding environment must enforce that condition.
Add `--restore-cached` to move the retained installed trees to
`WORKSPACE/installs-before-restore` and restore them through
`install --frozen --offline`, then verify actual execution and empty-cache errors.
Use this restoration check once per retained workspace; the backup is preserved.

With online access and a retained workspace, exercise explicit updates with:

```sh
python tooling/test-tool-updates.py --cli PATH_TO_FEATURE_ENABLED_OYZU --workspace PATH
```

The runner changes the first project's requirement from 22.15.0 to 22.14.0,
checks stale-selection rejection, updates and executes the new version, then
restores 22.15.0 through an explicit update. It checks that the second project is
unchanged and a repeated update preserves lock bytes. On failure it retains the
workspace at the failing step for diagnosis.
This complete update scenario passed on Linux amd64 with Rust 1.95 on 2026-10-02
and Windows amd64 at `1acb16f` in CI run `37048375166`; native macOS update
acceptance remains pending.

Remaining functional work includes other tools, aliases/native constraints,
multi-tool and scoped updates, configured corporate
transport, managed selection, npm entrypoints, shims, activation and build handoff.
Managed configuration is explicitly rejected by this standalone proof. Child
stdio currently carries internal metadata; authenticated worker IPC, full process
tree cleanup and further hardening are deferred. Transitive notices/SBOM and
distribution approval remain separate from the authorized development import.
