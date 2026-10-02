# Repository compliance tooling

The provisional repository guard runs independently of the Oyzu executable.
It requires Git and Python 3.11+; it does not compile or run dependencies or
require a platform account. Full rules, command options, outputs, review behavior
and outstanding release obligations are maintained in the
[compliance guide](../../compliance/README.md).

From the repository root, `python tooling/compliance/check.py` compares tracked
dependency inputs and notices with the factual baseline. Exit 0 means the
inventory matches, not that a license or distribution is approved. Exit 1 reports
drift or invalid records. Investigate the change before using `--write-baseline`;
new inputs must be tracked by Git. `--report PATH` writes the factual JSON report,
and `--base REF` adds a change summary against that Git baseline. All these
operations are local/offline. No file is changed without a write/report option.

`--release` intentionally exits 2 after valid inventory checks because the actual
target-specific audit and artifact/source-availability gate are not implemented.
The checker does not certify existing development CI artifacts. It cannot identify
arbitrary copied source, infer all manager graphs or satisfy attribution by itself.

The GitHub review guard needs the event payload and a read-only GitHub token in
CI. API failure denies its review check; there is no offline approval fallback.
It checks the current head against a named human reviewer from the base policy.
Server-side branch protection and merging the foundation remain deployment steps;
neither CODEOWNERS nor files on a feature branch activate protection alone.

Fourteen regression tests exercise inventory drift, notice deletion/escape,
duplicate records, unreviewed dependencies, provisional release failure and
current/stale/dismissed/unauthorized reviews. They run locally on Windows and in
Ubuntu GitHub Actions. No production Rust capability changes with this tooling.
