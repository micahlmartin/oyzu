# Native Vite configuration overlay

Start from the sibling [Vite frontend/image variant](../vite/) and copy `vite.config.mjs` into its `frontend/` directory. Keep its package manifest, lockfile, build targets and Dockerfile. No additional Oyzu configuration is required.

The native configuration chooses `web/production` and emits a plugin-owned asset. The configured builder must record the resolved output, stage its contents under the deterministic versioned primary artifact, retain native tests/coverage and quality gates, and materialize the primary artifact into the image. The generated directory is excluded from implicit quality checks. Static discovery must never execute this configuration.

This is a design-contract variation with a native host probe and a registered Docker acceptance case. Check implementation status for verified hosts and captured-build results; it does not establish matrix or SSR support.
