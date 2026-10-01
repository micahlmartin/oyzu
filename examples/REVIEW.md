# Reviewing the example contract

Review project files first, then compare the README's expected behavior. Judge the developer-facing shape before internal implementation details.

## Core decisions to validate

- Does a conventional native project need any Oyzu configuration? If so, is the missing information actually undiscoverable?
- Does the minimal Python app hint select sensible artifacts and an entrypoint without mandatory test/output sections?
- Are `api:test`, same-name overrides, and `api:pre_test`/`api:post_test` understandable? Should success-only post hooks remain the rule?
- Do directory-local tools/env and ignored overrides have clear ownership and restoration behavior?
- Should target-local matrix axes be the proposed YAML shape, or can a builder infer the desired compatibility set?
- Does an image depending on an application imply a conventional binary/context mapping? What minimal binding should be needed when that convention does not hold?
- Can the chart consume the image digest without editing source values or describing deployment?
- Are absent tests, unsupported coverage, failed tests, and missing required reports visibly different?
- Do local-origin artifacts remain ineligible for production after cache reuse, upload, or later signing?
- Does managed logout/unavailability clearly stop access without falling back to public sources?

## Candidate syntax under discussion

The compatibility matrix and container variants include a small `matrix` map under their target. This is a review proposal, not accepted syntax. It expresses finite values, not loops or a programming language.

Cross-artifact examples currently declare `depends_on` and record intended bindings alongside the project. We should decide whether builder conventions are enough before adding binding fields to build.yaml.

Generated-source and affected-build examples provide native scripts/imports and expected dependency edges. They intentionally avoid requiring users to duplicate the native graph.

## Capturing changes

Change the example and its expectation together. Reference the relevant OEP when the behavior affects a contract. Do not turn an example-only preference into a mandatory field across every builder. Accepted examples will later become executable acceptance tests; their current value is making the proposed experience concrete.
