# EX-012: A Python HTTP API with minimal builder intent

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- The sole explicit Oyzu hint is api: uses: python/app.
- Use the declared console entrypoint; build/test the app and plan a pinned-runtime container when supported.
- No mandatory test or output section.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


The build YAML uses draft OEP syntax and contains only target intent/relationships. It is not a stable schema.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
python -m unittest discover -s tests
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **ambiguous-entrypoint:** Add a second equally plausible app entrypoint. Expected: Ask for a focused choice; do not guess the service.

## Contract and limitations

The implemented baseline now emits a snapshot console application `.pyz`, wheel and source distribution, with archive-source JUnit/coverage and quality gates. See [Python applications](../../../docs/reference/python-applications.md) for prerequisites and pure-Python limits. Multiple declared scripts fail with an ambiguity diagnostic; an explicit selection override and optional container assembly remain pending. Revision-specific acceptance evidence is tracked in implementation status rather than inferred from this example's presence.

Acceptance criteria: PLAN-01, BUILDER-04, BUILDER-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

## Detailed specification candidate

[Container candidate](variants/container.build.yaml) demonstrates the optional v1alpha1 packaging hint specified by OEP-0018. It is separate from the minimal baseline, and entrypoint inference still requires sufficient native evidence.
