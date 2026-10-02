"""Native Biome gates must control actual snapshot artifact collection."""
import json
import shutil
import subprocess
import tarfile


def verify(root, base, invoke, validate, source_files, verified):
    project = base/'node-biome'
    shutil.copytree(root/'examples/builds/node-package/project', project)
    config = {'linter':{'rules':{'recommended':False,'suspicious':{'noDebugger':'error'}}}}
    (project/'biome.jsonc').write_text('// Native Biome configuration\n'+json.dumps(config)+'\n')
    # Author only the source-code inputs with the native formatter before capture.
    sources = [str(path) for path in project.rglob('*') if path.suffix in ['.js','.mjs','.cjs']]
    subprocess.run(['node',str(root/'tooling/images/node-quality/node_modules/@biomejs/biome/bin/biome'),'format','--write',*sources],cwd=project,check=True,capture_output=True)
    before = source_files(project)
    invoke(project,'build')
    manifest = validate(project/'dist')
    assert source_files(project) == before and manifest['status'] == 'succeeded'
    assert len(manifest['artifacts']) == 1
    for stage in ['lint','format-check']:
        assert next(a for a in manifest['actions'] if a['id'] == f'project:{stage}')['status'] == 'succeeded'
    tests = next(r for r in manifest['reports'] if r['kind'] == 'test')
    coverage = next(r for r in manifest['reports'] if r['kind'] == 'coverage')
    assert tests['summary']['passed'] > 0 and coverage['summary']['covered'] > 0
    artifact = manifest['artifacts'][0]
    with tarfile.open(project/'dist'/artifact['path']) as archive:
        package = json.load(archive.extractfile('package/package.json'))
        assert '-dev.g' in package['version']
    bad = project/'quality.js'
    for source, stage in [('debugger;\n','lint'),('export const value=42;\n','format-check')]:
        bad.write_text(source,newline='\n')
        before = source_files(project)
        invoke(project,'build',success=False)
        failed = validate(project/'dist')
        assert source_files(project) == before and not failed['artifacts']
        assert next(a for a in failed['actions'] if a['id'] == f'project:{stage}')['status'] == 'failed'
    verified.append('Node Biome: native snapshot package with JUnit/coverage and read-only lint/format failures blocking artifact collection')
