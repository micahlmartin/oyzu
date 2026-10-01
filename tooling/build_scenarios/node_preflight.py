"""Native toolchain compatibility must be decided before build actions start."""
import json
import shutil
from jsonschema import Draft202012Validator


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'node-declared-manager'
    shutil.copytree(root / 'examples/builds/node-managers/npm', project)
    before = source_files(project)
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and manifest['artifacts']
    assert before == source_files(project)
    record = json.loads((project / 'dist/dependencies/project.json').read_text())
    schema = json.loads((root / 'docs/contracts/v1alpha1/dependencies.schema.json').read_text())
    Draft202012Validator(schema).validate(record)
    assert record['manager']['version'] == '11.11.0' and record['packages'] == []
    assert record['extensions']['oyzu.dev/npm']['nodeVersion'].startswith('22.')
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] > 0
    invoke(project, 'inspect', 'dist')
    package_file = project / 'package.json'
    package = json.loads(package_file.read_text())
    package['packageManager'] = 'npm@0.0.0'
    package_file.write_text(json.dumps(package))
    invoke(project, 'build', success=False)
    mismatch = validate(project / 'dist')
    assert not mismatch['actions'] and not mismatch['artifacts']
    assert 'does not match provisioned' in mismatch['diagnostics'][0]['message']
    package['packageManager'] = 'npm@11.11.0'
    package['engines']['node'] = '>=999'
    package_file.write_text(json.dumps(package))
    invoke(project, 'build', success=False)
    engine = validate(project / 'dist')
    assert not engine['actions'] and not engine['artifacts']
    assert 'EBADENGINE' in engine['diagnostics'][0]['message']
    verified.append('EX-019 npm: matching provisioned manager, native engine checks, dependency-free preflight evidence, snapshot artifact and preflight rejection of incompatible versions')
