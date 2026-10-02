# Build logging

`oyzu build` now defaults to readable live progress and a concise final summary. It shows discovery/preflight, source capture, toolchain and dependency preparation, planning, queued actions, running commands, native stdout/stderr, outcomes and output paths. It does not change scheduling or the required build checks.

```text
oyzu build
oyzu build api web
oyzu build --plan
oyzu build --json > result.json 2> events.jsonl
```

Live output goes to stderr; the final human summary or JSON result goes to stdout. `--json` is the explicit machine interface: stdout retains the existing final manifest/plan JSON, while build progress is emitted as JSON lines on stderr. Scripts that previously parsed the default build output must add `--json`. Other commands retain their existing output behavior.

## Plain-text view

The following is illustrative; elapsed times and interleaving depend on execution:

```text
[00:00.010] [preflight] Discovering builders and validating inputs
[00:01.200] [api:test] RUNNING
[00:01.201] [api:test] $ cargo test --locked --offline  (cwd: /workspace/.)
[00:01.230] [web:build] stdout | Building application
[00:01.250] [api:test] stderr | Running tests
[00:03.500] [api:test] SUCCEEDED (exit 0) [2.30s]
```

Every native output line retains its target/action scope and stdout/stderr identity. Parallel scopes share a synchronized sink, so complete rendered lines remain separate. Ordering within each stream is preserved; observation order across independent processes or stdout/stderr is not a causal ordering guarantee. Elapsed time and a monotonically increasing sequence identify emitted events.

Commands display their argument boundaries and working directory. Quoted arguments use JSON string escaping for readability; the displayed command is not a portable shell script. The event's `argv` array is authoritative. Engine-injected environment values are not dumped. Output is formatted as append-only plain text, works without a terminal and does not use cursor movement or ANSI styling. Control characters are escaped in the rendered view.

The executor follows its retained files during execution. Quiet commands emit a running message every ten seconds. Native tools retain their own buffering: Oyzu cannot display bytes before a process writes them. Final unterminated lines are flushed at exit; very long pending lines are emitted in marked fragments. Raw action logs preserve the original bytes.

## Outcomes and retained evidence

Command completion and action completion are distinct: successful process exit can still be followed by a failed report or artifact check. Failed commands remain visible; dependent stages show `BLOCKED` with their reason. The final summary lists action statuses/durations, collected artifacts/reports, diagnostics and the manifest/log directory. It never presents a planned artifact as a collected result.

Normal builds retain `dist/logs/events.jsonl` alongside the existing numbered action stdout/stderr files, `manifest.json` and `plan.json`. Events also retain native preparation output that was previously only available in temporary files. The journal begins once the bundle transaction exists; earlier configuration failures can only appear on the console. `--plan` retains its existing no-bundle behavior, so its progress is console-only. History follows the existing [bundle lifecycle](build-bundles.md).

Event records have `schemaVersion`, `sequence`, `elapsedMs`, `scope` and a typed `event`: `progress`, `command`, `output` or `finished`. Output includes `stream`, `text` and `continued`; completion includes `status`, optional `exit_code` and optional `duration_ms`. This is an experimental observational format, separate from deterministic plan identities and release evidence. Log integrity is not a claim of release authority.

The logger adds no network calls. Command arguments and project output may contain values deliberately printed by project code; no arbitrary-output secret detection is promised. Upstream broker credentials remain outside executor arguments/environment under the existing acquisition contract. Existing raw-log size and execution-time limits still apply. Sink write failures do not change native execution outcomes; durable remote log delivery and terminal dashboards are outside this implementation.

## Verification

`tooling/test-build-logging.py --cli <compiled-cli> --evidence-dir <new-directory>` provisions no tools. With the documented Rust image already present, it builds two real Rust targets concurrently, checks live stdout/stderr before process completion, preserves final partial lines, verifies snapshot artifacts and bundle inspection, then checks a deliberately failed hook, blocked packaging and JSON output. Rust tests also cover concurrent event sequencing and split UTF-8 output. See [implementation status](../implementation-status.md) for completed runs; registration is not a passing result.
