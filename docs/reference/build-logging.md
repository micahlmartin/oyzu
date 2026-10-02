# Build logging

`oyzu build` presents a pipeline in the terminal: phases, parallel targets, commands, live output and an evidence-based final receipt. Presentation never changes the task graph, execution policy or required checks.

```text
oyzu build
oyzu build api web worker
oyzu build --output plain
oyzu build --output interactive
oyzu build --plan
oyzu build --json > result.json 2> events.jsonl
```

## Presentation selection

`--output auto` is the default. With interactive stdin/stdout/stderr, a non-dumb terminal and no detected CI environment it opens a persistent dashboard. CI and redirected streams use append-only text. `--output plain` explicitly disables the dashboard; `--output interactive` requires terminal stdin/stderr and allows inspecting the dashboard even in CI. `--json` takes precedence over either option: stdout contains the final manifest/plan JSON and stderr contains JSON-line events only. No project configuration is needed.

The dashboard uses Crossterm's Windows/macOS/Linux terminal support. Auto mode falls back to text if terminal setup fails; explicit interactive selection reports a setup error. Windows GNU builds using an older bundled GNU linker can encounter import-table incompatibility with terminal dependencies; the local proof uses the provisioned LLVM MinGW linker explicitly. Standard CI compilation remains registered on all three hosts; see [implementation status](../implementation-status.md) for actual verification.

## Persistent terminal view

The build header, elapsed wall time, phase indicators, target overview and controls stay in place. Targets show their current task/state and completed task count; a bounded target area follows task selection for larger builds. The view uses the actual planned tasks, including hooks and builder-specific steps, rather than a fixed set of pipeline columns. Version identities appear when planning supplies them; they do not imply release eligibility.

Up to three independent active-task panels show recent output, depending on terminal height. A task that has exited successfully may show `collecting` until the scheduler has validated its results. Actual scheduler concurrency and its current bounded-batch admission behavior are unchanged; this view does not promise newly asynchronous scheduling.

| Key | Action |
| --- | --- |
| Up / Down | Select previous/next planned task and show its details |
| Enter | Expand the selected task's command, dependency/reason and output |
| a | Show active tasks in separate panels |
| l | Show complete multiplexed log history |
| Page Up / Page Down | Scroll the selected/all log view |
| Left / Right | Pan long log/command lines horizontally |
| f / End | Follow new output again |
| q | Hide the dashboard and continue the build with plain logs |
| Ctrl+C | Restore the terminal and interrupt the CLI with exit 130 |

A failed command or final action failure selects its task automatically. Successful process exit does not establish action success: reports/artifacts can fail validation later. Blocked tasks expose their prerequisite reason. The interface does not require input to finish.

Terminal resizing recalculates panel sizes. Very small terminals show a resize hint. Output is clipped to display cells rather than wrapping into the pinned header; horizontal panning reveals long lines. Interactive scrollback uses a temporary disk spool with in-memory line offsets, retaining early output without holding every output byte in memory. It is removed when the view closes. Raw build logs and the event journal remain the durable record.

The alternate screen restores the previous terminal at normal completion or error, then prints the permanent receipt. `q` restores the screen immediately without cancelling work. Ctrl+C retains the existing abrupt-interruption semantics; graceful process/container-tree cancellation is not added by this presentation change.

## CI and redirected output

The plain view prints phase boundaries, a target/builder/task plan, `START`, `PASS`, `FAIL` and `BLOCKED` records, exact commands with working directories, and labeled stdout/stderr as they arrive. Every ten seconds it reports active tasks and finished counts when execution remains unfinished. Native processes retain their own buffering; Oyzu cannot show bytes before the process writes them.

Concurrent scopes share a synchronized sink so lines do not interleave at the byte level. Within-stream ordering is retained. Observation order between independent tasks or stdout/stderr is not a global causal order. Command arguments use JSON escaping for readability; the structured `argv` array is authoritative, not a portable shell command string. Control characters are escaped. Text redirected to a file contains no dashboard cursor controls.

Detected provider names include GitHub Actions, GitLab CI, Azure Pipelines, Buildkite, Jenkins and TeamCity, with generic CI fallback. These are unverified presentation hints, never credentials or release authority.

In GitHub Actions, sequential preflight, dependency and planning sections use collapsible groups. Groups close before parallel execution; individual concurrent tasks do not open overlapping groups. On completion, Oyzu appends an HTML-escaped receipt to `GITHUB_STEP_SUMMARY` if supplied. Failure to write the summary warns without changing build status. JSON mode still uses only JSON events on stderr, with no group commands. Local artifact paths are not presented as download links: artifact uploading, additional provider-native summaries and reliable source-location annotations are not implemented here.

## Final receipt and retained evidence

The receipt lists each target's outcome, completed/total tasks, summed **action time**, collected test totals, per-report coverage with its metric, artifacts, reports, diagnostics and manifest/log paths. Overall elapsed time is wall time; action durations overlap and must not be added to infer build elapsed time. Missing reports do not become zero failures or 100% coverage. Coverage is not averaged across unrelated targets/metrics. Test counts describe collected reports, not inferred absence of other tests. Failed and blocked task details remain visible. Planned artifacts are labeled as planned; only collected artifacts appear as build outputs.

Normal builds retain `dist/logs/events.jsonl`, numbered action stdout/stderr files, `manifest.json` and `plan.json`. Preparation output is included in events. The journal starts once the bundle transaction exists; earlier setup failures can appear only on the console. The final console build outcome follows publication of the bundle; the journal is closed before publication. `--plan` retains its no-published-bundle behavior. History follows the [bundle lifecycle](build-bundles.md).

Event records have `schemaVersion`, `sequence`, `elapsedMs`, `scope` and a tagged `event`: `progress`, `phase`, `plan`, `task`, `command`, `output` or `finished`. Phase/task events carry lifecycle facts. The plan event contains only presentation inventory (target ID/builder, action ID/target/dependencies and artifact versions), never action environments. Output includes `stream`, `text` and `continued`; completion includes `status`, optional `exit_code` and optional `duration_ms`. Consumers should tolerate new event types. This experimental observation format remains separate from deterministic plan identity and release evidence.

The logger adds no network calls. Environment maps are not dumped. Project commands/output can deliberately print sensitive values; arbitrary-output secret detection is not promised. Upstream broker credentials remain outside executor arguments/environment under the existing acquisition contract. Native raw-log size and process-time limits remain in force. Log sink failures do not override native process results; durable remote delivery is separate work.

## Verification

`tooling/test-build-logging.py --cli <compiled-cli> --evidence-dir <new-directory>` runs two real Rust targets, verifies live parallel streams, four snapshot artifacts, report collection, JSON failure/blocked events and bundle inspection. The Rust image must already be provisioned.

For real terminal acceptance, install `tooling/terminal-requirements.txt`, then run `tooling/test-build-terminal.py --cli <compiled-cli> --evidence-dir <new-directory>`. This uses ConPTY on Windows and a pseudo-terminal on Unix to exercise the actual CLI, independent active panels, navigation, resizing, failure focus, hide/restore behavior and resulting bundles. Screen transcripts and representative text frames are retained. Unit tests cover the event projection, full spool history, control escaping and evidence-based receipts. Registration is not passing evidence; see [implementation status](../implementation-status.md).
