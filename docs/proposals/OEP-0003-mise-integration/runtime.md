# Tool runtime, commands and shell contract

Normative draft companion to [OEP-0003](README.md). Names below are proposed Oyzu
interfaces, not claims about current public mise APIs.

## Ownership and process topology

Keep the first implementation in the existing Rust crate, with private modules:

| Module | Responsibility |
| --- | --- |
| `src/tools/mod.rs` | Facade and immutable request/result types |
| `src/tools/selection.rs` | Effective-config projection, exact closure selection and policy admission |
| `src/tools/lock.rs` | Strict format-2 parsing, identity and transactional editing |
| `src/tools/store/{mod,receipt,transaction,lease}.rs` | Content store, verified receipts, publication, recovery and prune |
| `src/tools/mise/{mod,context,backend,environment}.rs` | Only production modules importing mise types; translation to upstream APIs |
| `src/tools/worker.rs` | Bounded same-binary worker protocol and immutable initialization |
| `src/tools/launch/{mod,unix,windows}.rs` | Generic launch descriptor and process-tree lifecycle |
| `src/tools/shell/{mod,state,profile}.rs` | Upstream-rendered hook integration, reversible state and opt-in profile edits |
| `src/tools/shims.rs` | Native shim generation, identity and command dispatch |
| `src/broker/` and `src/executor/` | Shared credential/transport and sandbox capability enforcement |
| Existing `src/builders/<ecosystem>/` | Native manifest requirements, package-manager semantics and typed script launch descriptors behind the crate-private Builder contract |

Do not move native npm behavior into the engine, add manager switches to shared
orchestration, expose speculative plugin APIs or create a global helpers module.
Mise backend translation is tool-management code; native project/task/package
semantics stay in the appropriate builder. Extend the Builder contract with
typed tool requirements and native launch metadata where needed.

The CLI resolves effective configuration, validates locks, checks protected
management state, obtains authorization, holds installation leases and supervises
execution. It never hands a project path to mise for independent discovery.
The agent owns credentials, policy and broker sessions; it does not initialize
mutable mise globals for multiple workspaces.

An internal `oyzu __tool-worker` instance links mise and serves exactly one
operation/context, then exits. Local pure environment/metadata operations use a
restricted child environment; network-capable acquisition runs inside the
qualified executor. A worker process is not itself a sandbox. The supervisor
derives the executable from its verified running image/release installation,
never PATH, project configuration or `MISE_*`. On Unix, re-exec the validated
absolute executable identity; on Windows, validate the opened image file against
the release manifest before `CreateProcessW`. A concurrent upgrade either retains
the old release directory or forces a clean retry.

## Facade and initialization

The crate-private facade exposes these operations with owned records:

| Operation | Input | Output / permitted effects |
| --- | --- | --- |
| `resolve` | Effective tool requests, target platforms, catalog snapshot, mode | Candidate lock change and explanation; authorized metadata I/O only |
| `prepare` | Exact locked closure, authorization, executor capabilities | Verified committed installation handles; no lock edits |
| `environment` | Verified selection, admitted literal env, prior shell state | Typed delta and selection status; no downloads, tasks or native code execution |
| `executable` | Verified selection, command name | Typed launch descriptor; no PATH guessing for a declared tool |
| `inspect` | Selection/store query | Redacted status, origins and reasons; no execution |

Worker initialization order is mandatory: validate envelope and capabilities;
construct scratch/home/cache paths; construct a minimal environment; initialize
upstream settings once; install explicit frontend/broker callbacks; initialize
backend registry from the pinned admission manifest; construct discovery-free
configuration; inject exact ToolRequest/ToolVersion/Toolset values in memory.
Do not call the upstream default discovery constructor. Disable automatic lock
writes, downloads on command lookup, self-update, task hooks, config templates,
interactive prompts and undeclared plugins.

Production removes every `OYZU_SPIKE_*` behavior, including the toy policy flag and
environment-variable artifact mapping. The fork supplies a private embedding
entrypoint with explicit options instead. Unknown embedding options fail closed.
Neither child environment variables nor project input can install a callback or
change a route. Preserve normal child application variables only after the
trusted selection phase; acquisition workers never receive the developer's full
environment or upstream credentials.

## Worker protocol and lifecycle

Use an inherited private duplex pipe/socket, not worker stdin/stdout, so tool
streams remain separate. Unix uses a socketpair with close-on-exec and explicitly
passed descriptor; Windows uses handles restricted to the spawned process handle
list. The worker refuses operation without the channel. Local transport is not a
public plugin ABI and not a bearer capability written into the environment.

Frames are a four-byte unsigned big-endian length followed by UTF-8 JSON. Limit
each control frame to 8 MiB, total control bytes to 32 MiB, nesting to 32 and a
single request plus terminal result. Reject duplicate keys, nonfinite values,
unknown fields and unknown protocol/operation values. Artifact bytes use broker
streams/CAS, never JSON frames. Cancel is the only additional request. A 10-second
handshake deadline and a supervisor-owned operation deadline bound hangs; defaults
are 30 seconds for local selection, 120 seconds for metadata and 15 minutes for
installation. Administrative caps may shorten deadlines; increasing them is a
host preference within policy, never project syntax.

Envelope fields are `protocol = "oyzu.tool-worker/1"`, `request_id` (UUID),
`operation` (`resolve`, `prepare`, `environment`, `executable`),
`context_digest`, `backend_release_digest`, `target_platform`, `capabilities`
(sorted unique IDs) and a typed `payload`. The payload carries normalized requests
or an exact locked closure, scratch/CAS handles and broker session handles as
appropriate. It never carries credentials or arbitrary shell snippets. An
environment request contains literal admitted values and validated state only;
a prepare request contains no arbitrary command field.

Response fields are the same protocol/request/context identifiers, `status`
(`ok`, `error`, `cancelled`) and exactly one typed `result` or `diagnostic`.
Progress events are bounded/redacted diagnostic-channel records, not stdout
shell fragments. Unrecognized responses, mismatched identity, pipe loss, timeout
or panic fail the operation. The supervisor verifies output containment, digests
and receipts; worker success alone cannot authorize publication or selection.

## Commands and mutation rules

All new commands use existing `-C/--directory` and `--json` conventions. A normal
project command resolves the workspace root under OEP-0002; it does not ascend
through an unrelated repository. `--profile` uses OEP-0002 profile selection.

| Command | Exact behavior |
| --- | --- |
| `oyzu install` | Missing lock: resolve current scope/profile and host platform, install, then atomically write lock. Existing compatible lock: install exact closure without changing resolution |
| `oyzu install --update [TOOL...]` | Explicitly resolve selected roots and affected dependencies; preserve unaffected selections; emit diff, install, then CAS-replace lock |
| `oyzu lock [--update] [--platform PLATFORM]... [--all-scopes]` | Metadata/verification-based resolution only, no tool execution or installed tree required; existing selections remain exact unless update requested; default current scope/profile and host |
| `oyzu install --frozen` | Require a compatible existing lock/platform; acquire missing exact bytes; never edit config/lock |
| `oyzu exec -- COMMAND [ARGS...]` | Frozen local selection and current eligibility; missing install fails with remedy, no implicit acquisition; inherit admitted development env and supervise command |
| `oyzu activate bash\|zsh\|pwsh` | Emit initialization script using the absolute running frontend; do not modify a profile |
| `oyzu deactivate SHELL` | Emit reversible cleanup for this shell session |
| `oyzu env [--json]` | Explain computed environment/selection without mutation; redact values by default |
| `oyzu which COMMAND` | Return the verified launch target and owning tool; does not execute it |
| `oyzu tools list/search/versions` | List installed/current status locally; search/versions uses explicit authorized catalog access, supports `--offline` for existing catalog only |
| `oyzu tools prune [--dry-run]` | Remove only unleased, unreferenced committed entries; print candidates and retained reasons; no lock edits |
| `oyzu shell install SHELL --profile-path PATH` / `remove` | Explicit opt-in bounded profile block editing; validate selected shell and ownership |

`install` never edits `[tools]`; adding a requirement remains a TOML/config editing
action. `--update` and `--frozen` conflict. `--offline` forbids all metadata and
artifact networking and requires complete local bytes and valid authority.
Locking a new platform is an explicit lock operation, not a side effect of exec,
activation, or build. A mismatched existing request fails ordinary install with
`TOOL_LOCK_STALE` and an explicit update remedy. CLI update arguments are canonical
IDs or unambiguous configured aliases; raw URLs and arbitrary backend options fail.

For `exec`, a declared executable resolves only through the selected closure.
A bare unknown name is an error rather than an ambient globally installed tool.
An explicit absolute or relative executable path is allowed for a requested
development command after workspace/trust/policy checks; it does not satisfy a
locked requirement. No implicit shell parsing, pipelines or redirection is added.
Users explicitly invoke a shell when they want its language. Build tasks continue
through Builder launch contracts and the hermetic executor, not development exec.

Successful administrative commands exit 0; validation/selection/policy errors
exit 2; install/runtime infrastructure errors exit 1; cancellation exits 130.
Exec preserves the application's exit code (including the native Windows DWORD
through the platform-specific exit API). Unix signal termination is forwarded to
the supervisor so the invoking shell observes signal termination, with the usual
shell `128 + signal` representation. Diagnostics use stable codes; stderr never
contains secrets. `--json` for exec is rejected rather than mixing JSON with
application stdout; structured status belongs to inspection commands.

## Environment and shell state

Initial supported shells are Bash 3.2+ on macOS, Bash 5.2+ on Linux, Zsh 5.9+ on
both Unix hosts and PowerShell 7.4+ on all three hosts where installed. Qualify
the minimum and current pinned CI version. Fish, Nushell, Windows PowerShell 5.1
and cmd.exe activation are explicitly unsupported in this release. Native exec
and executable shims do not require a supported interactive shell.

Use upstream shell rendering and diff logic through the facade; fork only where
tests demonstrate a gap. Never concatenate raw config values into executable
shell text. Environment values are literal under OEP-0002; secrets require its
explicit consumer/trust contract. No shell expansion or new configuration language
is introduced. Exported broker credentials are prohibited.

Each shell gets a random session ID and private local state file containing:
schema version, workspace/scope/profile, config/lock/selection digests, changed
variable before/after values (distinguish absent from empty), PATH before/after
ordered lists, owned insertion positions and sequence number. Files use owner-only
permissions and contain no registry secrets. The environment carries only the
session token, not serialized executable code. A session cannot reference paths
outside the state directory or load another user's file. Hook updates use
compare-and-replace on the sequence; they do not share state across terminals.

On transition, remove the prior delta before applying the next. For a scalar,
restore its prior value only if the current value equals the last applied value;
otherwise retain the user's edit. PATH rollback uses recorded insertion positions
and an order-preserving alignment against the last applied list: match from left
to right, retain unmatched current entries, remove only matched owned entries,
then restore original occurrences. Equal duplicate entries are matched by
occurrence, not deduplicated. If user edits make ownership ambiguous, retain the
ambiguous entry and emit a once-per-transition diagnostic; do not delete a
possibly user-owned path. Windows comparison uses native path/case rules while
preserving original spelling and order. Unchanged transitions emit nothing.

Observe cwd identity, config/lock content digests and receipt/policy generations.
Never rely on mtime alone. Reuse an agent/local selection snapshot only when those
identities still match. Prompt hooks do no network, installation, project task or
foreign executable execution. An unavailable target clears prior project-owned
selection, exports a visible unavailable status, and returns syntactically valid
cleanup; do not leave the preceding project's Node active. Shim lookup for the
declared missing command then fails explicitly. Emit a diagnostic once per state
change, not on every redraw. Interactive development activation is convenience,
not a sandbox restricting commands manually run outside Oyzu.

Profile installation accepts a user-selected file, creates a backup, preserves
newline/encoding and unrelated bytes, inserts exactly one versioned marked block,
and atomically replaces only if the original digest still matches. Refuse a
symlink/reparse redirection or a block edited outside the generated grammar;
report the conflict instead of overwriting it. Remove only a matching owned block.
No package installer silently modifies profiles.

## Launch descriptors, shims and process control

A launch descriptor contains `tool_key`, `installation_key`, `kind` (`native` or
`interpreter`), executable path relative to a leased installation, optional
interpreter installation key, fixed prefix arguments, working-directory policy
and required environment. Paths and interpreters must resolve inside verified
receipts. An interpreted package binary (for example a Node script) invokes the
locked Node executable with the script path as an argv element. Parse neither a
generated `.cmd` file nor an arbitrary shebang into shell code. Ecosystem adapters
must produce typed interpreter metadata; unknown wrappers fail admission.

Shims are hardlinks to the delivered `oyzu` image when possible; otherwise copy
that identical image to a versioned private shim directory and verify its release
digest. Windows names end in `.exe`; do not generate `.cmd` shims. Unix may use
validated symlinks to a retained release image. A manifest maps shim basename to
command name and release identity, never a frozen project/version. It is
owner-protected and project-independent. Every invocation resolves its current
cwd/config/lock and current authorization anew. Detect shim invocation before
normal CLI parsing; dispatch from validated executable basename/manifest, not a
forged environment variable. Duplicate command names in selected tools fail
`TOOL_COMMAND_AMBIGUOUS`; no registration-order winner.

Windows uses `CreateProcessW` with an explicit application path and a tested CRT
argv encoder for native/typed-interpreter descriptors. Preserve empty arguments,
quotes, Unicode, backslashes and `&|<>^()%!` without cmd.exe. The supervisor creates
a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, creates the child suspended,
assigns it before resume, disallows breakaway, and retains the job handle. Nested
job assignment failure is a capability error before execution, not a fallback to
`Command::status`. Console control is forwarded where supported; after a
five-second cancellation grace terminate the Job Object. Killing the supervisor
closes the final job handle and terminates descendants. Detached services are
outside this command contract and cannot escape under a "background" flag.

On Unix retain a supervisor and create a child process group; forward INT/TERM/HUP,
reap children, allow five seconds then kill the group. Keep terminal foreground
ownership correct for interactive jobs, restoring it on exit. SIGKILL of the
Unix supervisor cannot reliably promise descendant cleanup on all hosts; the
generic development contract makes no such promise. Managed build/acquisition
workers use the executor's stronger container/job lifetime. The experiment's
Unix exec-replacement behavior is not used to pretend supervision has no costs.

The supervisor holds installation leases through child-tree completion. Shell
activation holds a renewable session reference so pruning cannot remove the
active PATH directory. Crashed sessions expire only after process identity
(including start time) no longer exists; PID reuse is not ownership. Read-only
copies into hermetic workers hold separate action leases.
