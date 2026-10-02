"""Verified Go artifact consumption and separate mutable target workspaces."""
import json
import shutil
from .node_fixtures import format_sources


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'artifact-materialization'
    project.mkdir()
    shutil.copytree(root / 'examples/builds/go-app/project', project / 'producer')
    shutil.copytree(root / 'examples/builds/node-package/project', project / 'consumer')
    configuration = '''consumer:
  uses: node/package
  path: consumer
  materialize:
    - from: producer
      to: bin/server
producer:
  uses: go/app
  path: producer
'''
    (project / 'build.yaml').write_text(configuration)
    (project / 'oyzu.toml').write_text('''[tasks."producer:post_test"]
argv = ["sh", "-c", "printf private > ../producer-marker"]
[tasks."producer:verify"]
argv = ["go", "version"]
[tasks."consumer:pre_test"]
argv = ["node", "--check", "build.mjs"]
depends_on = ["producer:verify"]
''')
    (project / 'consumer/materialization.test.mjs').write_text('''import test from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {existsSync, writeFileSync} from 'node:fs';
test('consume the tested binary without sharing producer writes', () => {
  assert.equal(execFileSync('./bin/server', {encoding:'utf8'}).trim(), 'Hello, Oyzu!');
  assert.equal(existsSync('../producer-marker'), false);
  writeFileSync('bin/server', 'consumer-owned mutation');
});
''')
    format_sources(root, project/'consumer/materialization.test.mjs')
    before = source_files(project)
    invoke(project, 'build', 'consumer')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and len(manifest['artifacts']) == 2
    assert source_files(project) == before
    plan = json.loads((project/'dist/plan.json').read_text())
    assert plan['extensions']['oyzu.dev/selection']['requested']==['consumer']
    assert plan['extensions']['oyzu.dev/selection']['selected']==['consumer','producer']
    verification = next(a for a in plan['actions'] if a['id']=='producer:verify')
    assert verification['target']=='producer' and verification['tools']==['producer']
    assert verification['cwd']=='producer' and verification['argv']==['go','version']
    assert next(a for a in manifest['actions'] if a['id']=='producer:verify')['status']=='succeeded'
    producer = next(a for a in manifest['artifacts'] if a['target'] == 'producer')
    evidence = next(e for e in manifest['evidence'] if e['kind'] == 'artifact-materialization')
    receipt = json.loads((project / 'dist' / evidence['path']).read_text())['inputs'][0]
    assert receipt['digest'] == producer['digest'] and receipt['artifact'] == producer['id']
    assert receipt['path'] == 'consumer/bin/server'
    consumers = [a for a in manifest['actions'] if a['target'] == 'consumer']
    assert all(evidence['id'] in a['producerEvidence'] for a in consumers)
    invoke(project, 'inspect', 'dist')
    # Source destinations cannot silently replace checked-in files.
    (project / 'consumer/bin').mkdir()
    (project / 'consumer/bin/server').write_text('source-owned data')
    invoke(project, 'build', success=False)
    collision = validate(project / 'dist')
    assert not collision['actions'] and not collision['artifacts']
    assert any('collides with captured source' in d['message'] for d in collision['diagnostics'])
    # The data edge participates in cycle detection before execution.
    (project / 'build.yaml').write_text(configuration.replace('from: producer', 'from: consumer'))
    invoke(project, 'build', success=False)
    cycle = validate(project / 'dist')
    assert not cycle['actions'] and any('cycle' in d['message'] for d in cycle['diagnostics'])
    verified.append('Materialization: symbolic ordering, executable Go artifact, verified digest receipt, private consumer copy, cross-language task prerequisite uses its Go owner, source collision and cycle rejection')
