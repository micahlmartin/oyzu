# EX-050: Materialize a directory into platform-specific image contexts

The frontend's native build produces a directory containing index.html. Configuration describes where its image consumer needs that artifact:

```yaml
frontend:
  uses: node/app
  path: frontend
image:
  uses: docker/image
  matrix:
    platform: [linux/amd64, linux/arm64]
  materialize:
    - from: frontend
      to: site
```

Run `oyzu build`. Each context contains site/index.html. The [Dockerfile](project/Dockerfile) uses COPY site/ /site/. It never references frontend/dist, and the result is not nested under site/dist.

This is a data-only image, not a web server. The authored build writes fixed static HTML. Reuse across variants requires established platform independence and equivalent inputs; arbitrary Node scripts or native addons do not get that assumption.

Image variants retain their own platform identities even if their content digest is identical. See [expected-materialization.json](expected-materialization.json).

Negative cases cover overlapping paths, escaping symlinks, ignored explicit inputs, and a producer becoming platform-dependent. Supporting commands are `node --test` and `node build.mjs` from project/frontend. Native success does not validate Oyzu materialization.

Acceptance: PLAN-07, PLAN-08, BUILDER-07. See [scenario.json](scenario.json) and the [shared contract](../../MATERIALIZATION.md). Implementation remains pending.

The engine now has [directory integrity and copying support](../../../docs/reference/directory-artifacts.md). This does not complete this example: native frontend output selection, evidence of platform independence and the two-platform image matrix still need integration and actual build acceptance. The authored project and its expected outcomes remain unchanged.

The [Vite variant](variants/vite/build.yaml) exercises a native frontend build and one same-platform image consumer. Its package script supplies the native output convention without additional Oyzu configuration. The [Node application reference](../../../docs/reference/node-applications.md) documents its directory artifact, browser lint defaults, test/coverage gates and supported customization. This variant supplements the original contract; passing it does not establish arbitrary-script inference or either cross-platform matrix requirement.
