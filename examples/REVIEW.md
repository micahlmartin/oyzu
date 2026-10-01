# Reviewing the example contract

Review project files first, then compare the README's expected behavior. Judge the developer-facing shape before internal implementation details.

## Core decisions to validate

- Does a conventional native project need any Oyzu configuration? If so, is the missing information actually undiscoverable?
- Does the minimal Python app hint select sensible artifacts and an entrypoint without mandatory test/output sections?
- Are `api:test`, same-name overrides, and `api:pre_test`/`api:post_test` understandable? Should success-only post hooks remain the rule?
- Do directory-local tools/env and ignored overrides have clear ownership and restoration behavior?
- Beyond the agreed platform matrix, which runtime compatibility axes require explicit configuration?
- Do the fixed-platform, multi-platform, named-output, and directory examples fully capture the agreed materialization behavior?
- Can the chart consume the image digest without editing source values or describing deployment?
- Are absent tests, unsupported coverage, failed tests, and missing required reports visibly different?
- Do local-origin artifacts remain ineligible for production after cache reuse, upload, or later signing?
- Does managed logout/unavailability clearly stop access without falling back to public sources?

## Candidate syntax under discussion

The image target's `platform` or `matrix.platform` requirement propagates to its runtime artifact producers. This example contract is agreed. Other axes, such as the Node runtime matrix, remain proposals. Matrices express finite values, not loops or a programming language.

Cross-artifact file/directory examples now use `materialize` entries with `from`, optional `artifact`, and `to`. The reference implies its dependency and places compatible outputs in the consumer's isolated workspace. See the [agreed contract](MATERIALIZATION.md). Chart image-digest value binding is separate and still open.

Generated-source and affected-build examples provide native scripts/imports and expected dependency edges. They intentionally avoid requiring users to duplicate the native graph.

## Capturing changes

Change the example and its expectation together. Reference the relevant OEP when the behavior affects a contract. Do not turn an example-only preference into a mandatory field across every builder. Accepted examples will later become executable acceptance tests; their current value is making the proposed experience concrete.
