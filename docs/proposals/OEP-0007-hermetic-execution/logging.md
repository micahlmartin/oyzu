# Build progress and multiplexed logs

This implementation companion remains part of draft OEP-0007. The requested outcome is a complete readable build experience: the user can see current phases, queued/running/completed actions, commands, native output, failures and resulting artifact paths without opening a JSON manifest to understand the build.

The first proof is `tooling/test-build-logging.py` running the compiled CLI on two real Rust targets with parallel hooks. Acceptance requires output from both targets while execution is active, distinguishable stdout/stderr, exact command arrays, four collected snapshot artifacts and an inspected bundle. A second invocation deliberately fails one hook and requires the original failure, blocked dependent packaging and independently parseable JSON events/result. This proves the connected lifecycle, executor, renderer and retained evidence together.

An invocation-owned logger is passed explicitly through build, preparation and execution contracts. Its shared synchronized sink orders observations; scoped clones label actions. It owns rendering/event retention, not builder semantics or scheduling. The executor streams retained output files while preserving existing process and sandbox behavior. Lifecycle owners emit phase and outcome facts; the CLI chooses presentation and renders final evidence.

The [current reference](../../reference/build-logging.md) defines the supported commands and event format. Default output becomes readable text; scripts explicitly request `--json`. This is a deliberate interface change from the previous unconditional manifest dump.

## Pipeline presentation increment

The maintainer requested implementation of the reviewed persistent terminal overview and append-only CI timeline. This increment does not accept unrelated parts of OEP-0007 or change build scheduling.

The entry point remains `oyzu build`. Auto presentation chooses a dashboard for an interactive local terminal, plain output for CI/redirection and structured events for `--json`. `--output plain` and `--output interactive` provide explicit human presentation selection. JSON takes precedence. The dashboard has persistent phases, bounded target status, task selection, independent active-task panels, failure focus and scrollable retained output. Finishing restores the terminal and prints a permanent evidence-based receipt. CI receives a plan, labeled live commands/output, lifecycle outcomes, periodic status and the same receipt; GitHub Actions also receives sequential setup/planning groups and a job summary. CI detection grants no authority.

First proof: run the compiled CLI against the two-target Rust logging fixture in a real pseudo-terminal and in captured CI mode. Observe overlapping active panels, task selection, log scrolling, resize and terminal restoration; require real artifacts/reports and inspect the resulting bundle. Repeat with a failed hook, requiring failure focus, blocked dependents and nonzero exit. Verify JSON remains parseable and redirected output contains no terminal controls. This connects the product before later performance/stress qualification.

The event contract adds typed phase, plan and task lifecycle events. Builders do not depend on terminal APIs. The presentation projects the frozen plan and observed execution, using a temporary disk spool for interactive scrollback. Crossterm 0.29 provides terminal/input portability (MIT, Rust 1.63 minimum); unicode-width 0.2 supplies display cell measurement (MIT/Apache-2.0, compatible with the project's Rust minimum). Their licenses remain those of upstream dependencies; no upstream source is copied into Oyzu.

Remote log delivery, automatic CI artifact uploads, additional provider-specific summaries, source annotations without reliable locations, process-tree tracing and additional performance/stress qualification remain separate work. Existing credential boundaries, raw-log limits and failure gates remain required.
