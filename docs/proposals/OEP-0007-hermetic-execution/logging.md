# Build progress and multiplexed logs

This implementation companion remains part of draft OEP-0007. The requested outcome is a complete readable build experience: the user can see current phases, queued/running/completed actions, commands, native output, failures and resulting artifact paths without opening a JSON manifest to understand the build.

The first proof is `tooling/test-build-logging.py` running the compiled CLI on two real Rust targets with parallel hooks. Acceptance requires output from both targets while execution is active, distinguishable stdout/stderr, exact command arrays, four collected snapshot artifacts and an inspected bundle. A second invocation deliberately fails one hook and requires the original failure, blocked dependent packaging and independently parseable JSON events/result. This proves the connected lifecycle, executor, renderer and retained evidence together.

An invocation-owned logger is passed explicitly through build, preparation and execution contracts. Its shared synchronized sink orders observations; scoped clones label actions. It owns rendering/event retention, not builder semantics or scheduling. The executor streams retained output files while preserving existing process and sandbox behavior. Lifecycle owners emit phase and outcome facts; the CLI chooses presentation and renders final evidence.

The [current reference](../../reference/build-logging.md) defines the supported commands and event format. Default output becomes readable text; scripts explicitly request `--json`. This is a deliberate interface change from the previous unconditional manifest dump.

Remote log delivery, interactive terminal dashboards, process-tree tracing and additional performance/stress qualification are later work. They do not precede proving this complete local flow. Existing credential boundaries, raw-log limits and failure gates remain required.
