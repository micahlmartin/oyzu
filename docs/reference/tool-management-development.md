# Core tool-management development integration

This opt-in implementation connects Node, Go and Rust installation and execution
through Oyzu configuration, the maintained mise Rust library, format-2 locks and
Oyzu's installation store. It is under active functional acceptance testing;
it is not a distribution-qualified release. The maintainer authorized development
import and reserved hardening for future goals.

Build with Rust 1.95 or newer:

```sh
cargo build --locked --features mise-integration --bin oyzu
```

The dependency is pinned to oyzuai/mise revision
`1da2a9fa009ada755cbcc96e5d944fe1cd61072c`; defaults are disabled and the selected
features are `rustls` and `vendored-lua`. A fresh instance of the Oyzu executable
owns mise's process-global state. No separate mise executable is used. The default
Oyzu build does not expose these commands yet.

The Rust-enabled fork pin changes the backend identity recorded in development
locks. Projects locked with the earlier pin must explicitly update their selected
tools online before frozen execution with this build; retain the previous frontend
when replaying an unchanged old lock. This is a development compatibility change.

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
resolver. Installation supports Node, Go and Rust selections at the workspace root.
The Node selector is interpreted by mise against the actual Node catalog. Existing
locks retain the exact version unless an update is explicitly requested.

Tool names are resolved through the maintained fork's pinned registry. For the
currently admitted backends, `node` and `"core:node"` identify the same tool, as
do `go` and `"core:go"`, and `rust` and `"core:rust"`. Quote canonical TOML keys containing a colon. Changing
only between these names preserves the normalized request identity and existing
lock; it does not require installation or relocking. Defining both names for one
tool fails with `TOOL_ALIAS_AMBIGUOUS`, even if their versions agree. Unknown
registry names fail with `TOOL_ALIAS_UNSUPPORTED`. This does not enable additional
backends or make alias names executable commands: use `node`, `go`, `cargo`,
`rustc` or `rustdoc` for exec.
Selective updates accept the same registry names. Administrative `tools.allowed`
values are compared through that same alias map; this setting remains admin-only.
The real acceptance runner is
`python tooling/test-tool-aliases.py --cli PATH --workspace NODE_ACCEPTANCE_PATH`;
it reuses the two-version Node workspace above without changing its projects.
Add `--update` for online canonical-name resolution. On a disposable root Linux
host only, `--disposable-linux-host` temporarily creates an administrative policy
to check both allowed-name spellings and denial; it refuses an existing policy
and removes its own file afterwards. Linux offline replay with networking disabled
and the separate online update passed on 2026-10-02. Native CI remains pending.

After editing the Node requirement in TOML, run:

```sh
oyzu install --update node
oyzu exec -- node --version
```

For a Node-only project, bare `--update` and `--update core:node` select the same root.
The command resolves through mise, prints the previous and proposed exact version,
installs the result and then commits the lock using the existing compare-and-swap
transaction. It never edits TOML. Ordinary install rejects changed requirements
with `TOOL_LOCK_STALE` and an update remedy. `--update` conflicts with `--frozen`
and `--offline`; unsupported tool names fail. A failed update leaves the previous
lock intact. An unchanged resolution preserves lock bytes.

This update path supports the configured Node/Go roots at the workspace root for
the selected profile and host. Locks containing additional environments or target
platforms are refused rather
than dropping their selections. Other projects sharing the store keep their own
locks and versions. Updates across multiple scopes/profiles/platforms remain to
be implemented.

The default store is `.oyzu/tools` relative to the invocation directory. The
feature-enabled CLI resolves `-C` to the physical directory before deriving
relative paths, so projects reached through directory symlinks (including macOS
temporary paths under `/var`) use the same physical default store. This does not
permit symlinks inside the installation store or a symlinked profile file.
For two
projects sharing installations, pass the same absolute `--store PATH` to both
`install`, `which` and `exec` (before `--` for exec). Commit `oyzu.lock`; keep the store out
of version control. Each project selects its own locked version when executed.

Installation requests metadata and the archive through Oyzu's existing HTTP
host fetcher, using `https://nodejs.org/dist/` unless an administrative connector
route applies (see below). It checks the
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
Only a missing archive requires acquisition from the selected route; invalid cached
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

`exec` also accepts an explicit absolute or relative executable path. It resolves
relative paths against `-C`/the invocation directory and applies the same frozen
Node selection, configuration eligibility and environment. For example:

```sh
oyzu exec -- ./bin/development-runner "argument with spaces"
oyzu exec -- /usr/bin/python3 scripts/check.py
```

On Windows, use a full executable path such as
`oyzu exec -- C:\Python313\python.exe scripts\check.py`. Drive-relative paths
such as `C:python.exe` are rejected as ambiguous. A program started this way can
find the selected Node at the front of PATH; the explicit program itself is a
requested development command, not a verified locked tool. This is ordinary
development execution, not the hermetic build executor. Invoke an interpreter
explicitly for scripts that need one. No implicit shell parsing is added.

Bare unknown commands still fail even if installed globally. Missing paths and
directories fail without changing the lock or acquiring tools. `which` remains
limited to the declared `node` command. Managed execution and process-tree/signal
supervision remain unfinished. Exercise the actual explicit-path flow against
the retained two-project acceptance workspace with:

```sh
python tooling/test-tool-exec.py --cli PATH_TO_FEATURE_ENABLED_OYZU --workspace PATH
```

Linux amd64 passed this scenario with container networking disabled on 2026-10-02.
Native Windows/macOS explicit-path execution acceptance remains pending in CI.

## Go installation and execution

For a Go project, configure:

```toml
[tools]
go = "1.24.13"
```

```sh
oyzu install
oyzu which go
oyzu exec -- go version
oyzu exec -- go run ./main.go
oyzu install --frozen --offline
```

Mise supplies version selection and target archive facts. Its official Go catalog
adapter checks the release filename, catalog size/hash and checksum sidecar through
Oyzu's host transport. Oyzu verifies the acquired bytes, publishes the installation
and retains the existing format-2 lock and frozen reuse/restoration contracts.
The lock records `go-releases` and digest-only verification, not publisher signatures.
No Go executable is needed to install the prebuilt release.
The Windows Go ZIP adapter uses the 800:1 per-file expansion allowance already
exercised by real Go archive qualification; its highly compressible compiler test
fixtures exceed the Node adapter's 200:1 allowance. Other archive limits are
unchanged. A lock authored with the earlier Windows Go layout needs an explicit
`install --update go`; frozen commands never silently change its layout identity.

`exec` and shell activation prepend the selected installation's `bin` directory,
set `GOROOT` to its payload and default `GOTOOLCHAIN` to `local` so normal execution
uses the locked compiler without automatic toolchain acquisition. An explicit TOML
environment setting can override `GOTOOLCHAIN` in this standalone development mode.
Ordinary Go module/package acquisition remains native Go behavior; selecting a
compiler is not yet managed module routing or hermetic build integration.

`install --update go`, `--update core:go` and bare `--update` explicitly resolve
the configured Go request again. Stale ordinary requests fail and frozen/offline
commands do not resolve new versions. Node and Go projects can share an explicit
store; a project can also declare both tools as described below. Native `go.mod` constraint
discovery, `gofmt` launch descriptors and builder handoff remain
unfinished. Supported target tuples remain Linux amd64 GNU, Windows amd64 and
macOS arm64; native qualification is recorded separately below.

Activation prepares native `node` and `go` shims, each with its own manifest. A shim
resolves the currently configured tool and refuses a command absent from that
selection. Existing retained frontend/shim versions are unchanged; deactivate and
reactivate to use the new frontend. `env --json` identifies the selected canonical
tool and continues to redact values.

Go accepts administrative `tools` routes scoped to `go`, `core:go` or `*`, with the
same explicit host binding file as Node. A Go proxy base must serve `index.json`
containing the official all-release Go catalog, plus archive filenames and their
`.sha256` sidecars at that base. Without a configured route, the host fetches
`https://go.dev/dl/?mode=json&include=all` and `https://dl.google.com/go/` artifacts.
There is no public fallback from a configured route. Real corporate Go proxy
acceptance remains pending; managed mode is still unavailable.

The real Go runner installs 1.24.13, checks frozen selection and GOROOT, and compiles
and runs a program using only the standard library, with literal arguments and
TOML environment. It can also exercise a native shell shim:

```sh
python tooling/test-tool-go.py --cli PATH_TO_FEATURE_ENABLED_OYZU --workspace PATH --shell bash
```

Retain the workspace and rerun with `--offline-check --restore-cached` under
externally disabled networking to move installations aside and restore from cached
archives. Use restoration once per workspace; the previous tree is preserved at
`installs-before-restore`. The script's offline option does not itself disable
networking. Use `--shell pwsh` on Windows or `--shell zsh` where available.

Linux amd64 passed real acquisition, frozen reuse, version/GOROOT checks, actual
compiler execution and Bash shim activation on 2026-10-02. Windows/macOS Go
acceptance remains pending in CI. This does not qualify native module constraints
or corporate Go proxy behavior. Mixed-project evidence is separate below.
The same Linux workspace then passed restoration from cached bytes and real
compiler execution with container networking disabled, preserving lock bytes.

## Mixed projects and selective updates

A project can request both tools in the same TOML file:

```toml
[tools]
node = "22.15.0"
go = "1.24.13"
```

```sh
oyzu install
oyzu which node
oyzu which go
oyzu exec -- node --version
oyzu exec -- go version
oyzu install --update node
```

Installation resolves the configured roots, prepares missing installations and
publishes one format-2 lock only after the complete selection is available. Existing
content-addressed installations can be reused across single-tool and mixed projects.
Failure leaves the previous lock intact; acquired cache entries may remain for reuse.

`--update node` or `--update core:node` resolves only Node and retains Go's exact
record, version and artifact identity. The equivalent Go names preserve Node.
Multiple update names select those roots; bare `--update` selects all configured
roots. Adding a tool requires an update that includes it. Removing a tool from
TOML requires bare `--update`. Changing an unselected request fails with
`TOOL_LOCK_STALE`; include that tool in the update or restore its requirement.
The command never edits TOML. Unchanged requests/records retain their existing
text through the lock-edit contract.

Exec and shell activation acquire one verified lease for the complete selection.
Both bin directories are prepended in deterministic configuration-key order, and
each backend adds its environment. A Node process can therefore launch the selected
Go, and vice versa. An incomplete or stale environment fails as a whole; execution
does not silently use one valid tool while replacing another from ambient PATH.
Shell selection identity includes every selected executable, so changing either
locked tool causes the next hook to recompute the environment.

`env --json` now includes `tools` (canonical IDs to executables) and `executables`
(command names to executables). The existing `tool` and `executable` fields remain
for single-tool projects; mixed projects use the maps. Environment values remain
redacted. `which` accepts either selected command and still returns its path.

The real runner installs both tools, changes only Node, verifies the complete Go
record is unchanged, and runs Node with a Go child under the composed environment:

```sh
python tooling/test-tool-mixed.py --cli PATH_TO_FEATURE_ENABLED_OYZU --workspace PATH
```

Use `--store PATH` to reuse an existing shared store. Retain the workspace and
repeat with `--offline-check` under externally disabled networking to verify
frozen reuse and both tools without acquisition. Scoped/profile/multi-platform
updates and native manifest constraints remain unsupported.
Add `--shell bash`, `--shell zsh` or `--shell pwsh` to exercise both commands
through an activated shell. Linux frozen replay, Node-to-Go child execution,
redacted inspection and mixed Bash activation passed with networking disabled
on 2026-10-02. Native Windows/macOS mixed-project acceptance remains pending.

## Standalone proxy acquisition

Administrative `registries.routes` can select an opaque connector for Node.
The existing protected `admin-settings.json` contract accepts this entry inside
its `settings` object:

```json
"registries.routes": {
  "locked": true,
  "value": [{"protocol": "tools", "scope": "core:node", "connectorId": "corp-node"}]
}
```

Routes remain administrative-only; they cannot appear in project TOML. Exact
`node` or `core:node` scopes take precedence over `*`; specifying both exact
aliases is ambiguous and fails. Other protocols do not route Node acquisition.
`tools.allowed` is enforced before selecting or installing Node. Non-public
`tools.catalogs` remain unsupported and fail without public fallback.

Supply a host-owned TOML binding file explicitly to `install`:

```toml
format = 1
[connectors.corp-node]
base_url = "https://proxy.example/node/"
authorization_env = "CORPORATE_NODE_AUTHORIZATION"
```

```sh
oyzu install --connector-bindings /absolute/path/to/host-bindings.toml
```

Relative binding paths are resolved from the invocation directory (including
`-C`). This file is not automatically discovered in a project. `base_url` must
end in `/` and serve the Node distribution layout: `index.json`, versioned
`SHASUMS256.txt` and archives. HTTPS is required except for loopback HTTP used by
local proxies. `authorization_env` is optional and names a host environment
variable containing the complete Authorization header value. Set that variable
through your host credential provisioning; do not put credentials in project
TOML, binding files or command arguments.

Both mise metadata requests and archive acquisition use the host broker and the
same binding. The mise child receives response bytes, not connector credentials
or endpoints. Existing broker redirect admission applies. This is an explicit
standalone development binding, not an agent-issued managed connector grant or
an OS network containment claim. Managed mode is still rejected.

Missing bindings, unavailable credential variables or denied requests fail;
there is no retry against the public source. Correct the binding or authorization
and rerun installation. Failed acquisition does not commit a new lock. Locks
retain the logical `node-releases` source and content identity without transport
endpoints or credentials. Frozen reuse and restoration from cached archives load
no bindings and require no proxy credentials. A missing cached archive in offline
mode fails without contacting the proxy.

The real proxy acceptance runner forwards actual public Node metadata and archive
bytes through an authenticated loopback proxy, installs and executes Node, checks
missing/denied binding failures, then reuses the frozen installation offline.
Run it only as root on a disposable Linux host; it temporarily provisions
`/etc/oyzu/admin-settings.json`, refuses to overwrite an existing policy and removes
its policy afterward:

```sh
python tooling/test-tool-proxy.py --cli PATH_TO_FEATURE_ENABLED_OYZU --disposable-linux-host
```

This scenario passed on Linux amd64 on 2026-10-02. Native Windows/macOS proxy
acceptance remains pending; the shared implementation has not yet been qualified
against a production corporate proxy.

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

### Install activation in a shell profile

Profile editing is explicit; `activate` alone never edits a file. Choose the
profile your shell loads, then run:

```sh
oyzu shell install bash --profile-path ~/.bashrc
oyzu shell remove bash --profile-path ~/.bashrc
```

Use `zsh` with your `.zshrc`, or PowerShell:

```powershell
oyzu shell install pwsh --profile-path $PROFILE
oyzu shell remove pwsh --profile-path $PROFILE
```

The required path resolves relative to `-C`/the invocation directory. Oyzu creates
missing parent directories and files, appends one marked block, and preserves
surrounding UTF-8 content and existing permissions. The block invokes the absolute
running Oyzu frontend with shell-specific quoting. Keep that frontend available;
rerunning installation after moving/upgrading it replaces an intact block. Repeated
installation with the same frontend is a no-op. `--json` reports `changed`, `shell`
and `profile_path` instead of human-readable status.

The block uses ordinary activation defaults: the current project's `.oyzu/tools`
store and configuration at shell startup. Install the project's locked tools before
opening a new shell. It does not embed a project, profile selection, credentials or
shared store path. Shell profile editing itself performs no networking or activation.
To use an explicit shared store or custom activation options, keep a separately
maintained initialization command outside the managed block.

Installation/removal refuses modified, incomplete, duplicate or differently owned
blocks. Preserve any manual customization elsewhere and restore/reconcile the
managed block before retrying. Symlink profiles require the explicit real target;
non-UTF-8 profiles require conversion before use. Removal deletes only the intact
block, including its inserted separator, preserving surrounding content byte for
byte. A newly created profile remains as an empty file after removal. Removing the
block affects future loads; run `oyzu deactivate` to end the current active session.

The real acceptance runner installs Node, loads each managed profile in its shell,
executes the locked Node through its shim, deactivates, and checks idempotence,
edited-block preservation and byte-exact removal:

```sh
python tooling/test-shell-profile.py --cli PATH_TO_FEATURE_ENABLED_OYZU --shell bash --shell zsh
```

Use `--shell pwsh` for PowerShell. The runner requires public Node access for its
fresh installation and never edits the user's actual shell profile.
Linux Bash and Zsh passed this complete flow on 2026-10-02. Native macOS and
PowerShell profile acceptance remain pending in the three-host CI workflow.

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
retention, abandoned-session recovery and fully reversible PATH alignment remain
unfinished. Profile installation/removal is explicit as described above. This is
a controlled development integration.

The real Linux Bash/Zsh acceptance passed with networking disabled: activation,
automatic `cd` switching between Node 22.15.0 and 22.14.0, unchanged-hook output,
missing-selection cleanup, user-edited scalar preservation and deactivation.
The subsequent native-shim replay also passed on Linux Bash/Zsh, including literal
arguments and exit status. A direct offline probe put a real Node later on PATH
and confirmed that a missing selection failed instead of invoking it. Native
macOS and Windows native shim acceptance passed at `e335d70` in CI run
`37051685318`, including their native activation/switching flows. Run it against the
retained two-project workspace:

```sh
python tooling/test-tool-activation.py --cli PATH_TO_FEATURE_ENABLED_OYZU --workspace PATH --shell bash --shell zsh
```

Linux Bash and Zsh application passed with networking disabled on 2026-10-02:
the shell launched the same Node executable as `exec`, literal metacharacters
were preserved, inspection values were redacted and the lock was unchanged.
PowerShell environment application passed on Windows at `3eefcc2` in CI run
`37049209850`; macOS Bash/Zsh environment application passed at `e335d70` in
run `37051685318`. Reproduce
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
and Windows amd64 at `1acb16f` in CI run `37048375166`. Native macOS update
acceptance passed at `e335d70` in run `37051685318`.

The complete Linux mixed runner also passed from a fresh project on 2026-10-02: initial installation, Node-only update preserving the exact Go record, rejection of a changed unselected requirement, frozen reuse and cross-tool execution. This extends the offline replay evidence above; native mixed CI remains pending.

Remaining functional work includes other tools, native constraints,
scoped/profile/multi-platform updates, managed connector grants and selection, npm
entrypoints, full shim/shell lifecycle qualification and build handoff.
Managed configuration is explicitly rejected by this standalone proof. Child
stdio currently carries internal metadata; authenticated worker IPC, full process
tree cleanup and further hardening are deferred. Transitive notices/SBOM and
distribution approval remain separate from the authorized development import.

Native checkpoint: at `1c2bd16`, CI run `37056475928` passed Windows PowerShell
profile installation/removal and explicit-path exec on both Windows and macOS.
Its Linux job passed Go installation, compilation, cached restore and shell
execution. Windows Go and macOS profile failures were corrected subsequently;
the corrected native results remain pending. See the consolidated
[review checkpoint](../implementation-status.md#oep-0003-review-checkpoint-2026-10-02)
for the remaining functional scope and evidence boundaries.


## Rust installation through mise

The Rust integration is being verified through the same `install`, `which`,
`exec`, environment and shell contracts as Node/Go. It requires a feature-enabled
Oyzu frontend and an exact stable version declared in Oyzu TOML:

```toml
[tools]
rust = "1.95.0"
```

```sh
oyzu install rust
oyzu which cargo
oyzu exec -- rustc --version
oyzu exec -- cargo build
oyzu install rust --frozen --offline
```

An explicit install name must already be configured. It validates the name and
installs the complete configured project selection, retaining one environment
lock rather than creating a separate Rust installation workflow. Bare `install`
continues to install all configured tools. The Rust backend currently accepts
exact three-part stable versions and uses the upstream minimal profile: Cargo,
rustc and rustdoc. Components, extra targets, rolling channels and native
rust-toolchain file discovery remain subsequent functional work.

Mise performs the rustup installation in private homes. Oyzu packages the completed
compiler sysroot into its existing cached archive/store format, retaining bundled
notices; the receipt identifies the published Cargo/rustc/rustdoc commands. The
lock's digest-only evidence covers that generated snapshot, not a claim of signed
publisher verification or a captured bootstrap download closure. Frozen cached
restoration does not invoke rustup. If both installation and archive are absent,
restore online through explicit update; do not silently regenerate a different
archive for an existing digest.

Rustup subprocesses use normal public downloads in this initial integration.
Managed mode and configured Rust connector routes are rejected until complete
routing is connected; direct fallback from an enforced route is not supported.
Cargo package/Git acquisition remains ordinary Cargo behavior. Mutable Cargo
cache/configuration is separate from the committed compiler payload. Execution
sets RUSTC and RUSTDOC to selected native binaries and prepends their bin directory;
no ambient rustup installation supplies the compiler. A compatible system linker
and SDK are prerequisites, particularly MSVC build tools on Windows and command
line tools on macOS. Installing Rust does not install those system prerequisites.

Acceptance is exercised by `tooling/test-tool-rust.py`. At `9ddf621`, Linux amd64
passed real installation with ambient Rust removed from PATH, store-owned
Cargo/rustc/rustdoc version checks, dependency-free compilation and program
execution, frozen reuse with unchanged lock bytes, and building the actual Oyzu
checkout's default frontend using the installed compiler. The last check reused
the provisioned Cargo dependency cache; it does not establish dependency capture
or an isolated Oyzu build. A subsequent run with container networking disabled
restored the removed installation from its cached snapshot and compiled the sample
project again. Native Windows/macOS Rust qualification remains pending.
