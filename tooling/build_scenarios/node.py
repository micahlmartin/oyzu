"""Native Node task replacement evidence through the compiled CLI."""
import shutil
import json
from jsonschema import Draft202012Validator


def verify_overrides(root, base, invoke, validate, source_files, verified):
    project = base / "node-overrides"
    shutil.copytree(root / "examples/tasks/overrides-and-hooks/project", project)
    before = source_files(project)
    invoke(project, "build")
    manifest = validate(project / "dist")
    assert manifest["status"] == "succeeded" and manifest["artifacts"]
    assert before == source_files(project)
    assert {r["kind"] for r in manifest["reports"]} == {"test", "coverage"}
    assert all(r["status"] == "collected" for r in manifest["reports"])
    ids = [a["id"] for a in manifest["actions"]]
    assert ids.index("api:pre_test") < ids.index("api:test") < ids.index("api:post_test")
    invoke(project, "inspect", "dist")

    # A custom native runner keeps its body. It receives exact required report
    # destinations; real Node reporters, rather than fixture XML, supply evidence.
    (project / "api/custom-tests.mjs").write_text("""import {spawnSync} from 'node:child_process';
const result=spawnSync(process.execPath, [
  '--test', '--experimental-test-coverage', '--test-reporter=junit',
  `--test-reporter-destination=${process.env.OYZU_TEST_REPORT}`,
  '--test-reporter=lcov',
  `--test-reporter-destination=${process.env.OYZU_COVERAGE_REPORT}`,
], {stdio:'inherit'});
process.exitCode=result.status ?? 1;
""")
    config = project / "oyzu.toml"
    config.write_text(config.read_text().replace('run = "node --test"', 'argv = ["node", "custom-tests.mjs"]'))
    before = source_files(project)
    invoke(project, "build")
    manifest = validate(project / "dist")
    assert before == source_files(project)
    assert {r["kind"] for r in manifest["reports"]} == {"test", "coverage"}
    assert next(r for r in manifest["reports"] if r["kind"] == "test")["summary"]["passed"] > 0
    assert next(r for r in manifest["reports"] if r["kind"] == "coverage")["summary"]["covered"] > 0

    # A zero exit code alone cannot satisfy the inherited builder contract.
    # Each build has fresh report storage, so prior evidence cannot mask this.
    (project / "api/custom-tests.mjs").write_text("console.log('no reports');\n")
    invoke(project, "build", success=False)
    manifest = validate(project / "dist")
    assert manifest["status"] == "failed" and not manifest["artifacts"]
    assert len(manifest["reports"]) == 2
    assert all(r["status"] == "invalid" and "path" not in r for r in manifest["reports"])
    assert next(a for a in manifest["actions"] if a["id"] == "api:test")["status"] == "failed"
    assert next(a for a in manifest["actions"] if a["id"] == "api:package")["status"] == "blocked"
    invoke(project, "inspect", "dist")
    verified.append("Node overrides: automatic native reporting, custom runner destinations, missing evidence blocks artifacts")
    verify_declared_reports(root, base, invoke, validate, source_files, verified)
    verify_defaults(root, base, invoke, validate, source_files, verified)
    verify_dependencies(root, base, invoke, validate, source_files, verified)


def verify_dependencies(root, base, invoke, validate, source_files, verified):
    project = base / 'node-registry-dependencies'
    shutil.copytree(root / 'tooling/fixtures/npm-registry', project)
    # Real tests execute inside the build sandbox after dependency preparation.
    (project / 'test/isolation.test.mjs').write_text("""import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
test('execution has no broker or external network', async () => {
  assert.equal(fs.existsSync('/broker'), false);
  await assert.rejects(new Promise((resolve,reject) => {
    const socket=net.connect({host:'1.1.1.1',port:443});
    socket.once('connect',()=>{socket.destroy();resolve();});
    socket.once('error',reject);
    socket.setTimeout(2000,()=>{socket.destroy();reject(new Error('timed out'));});
  }));
});
""")
    before = source_files(project)
    first_plan = invoke(project, 'build', '--plan')
    assert first_plan == invoke(project, 'build', '--plan'), 'capture destabilized identical plans'
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    assert len(manifest['artifacts']) == 1
    artifact = manifest['artifacts'][0]
    assert '-dev.g' in artifact['version']
    assert next(r for r in manifest['reports'] if r['kind'] == 'test')['summary']['passed'] == 2
    assert next(r for r in manifest['reports'] if r['kind'] == 'coverage')['summary']['covered'] > 0
    dependency = json.loads((project / 'dist/dependencies/project.json').read_text())
    schema = json.loads((root / 'docs/contracts/v1alpha1/dependencies.schema.json').read_text())
    Draft202012Validator(schema).validate(dependency)
    assert dependency['manager']['id'] == 'npm'
    assert [(p['name'],p['version'],p['sourceId']) for p in dependency['packages']] == [('is-number','7.0.0','npm-public')]
    assert dependency['packages'][0]['verification'] == 'digest-only'
    invoke(project, 'inspect', 'dist')
    invoke(project, 'build')
    again = validate(project / 'dist')
    assert again['planDigest'] == manifest['planDigest']
    assert again['artifacts'][0]['digest'] == artifact['digest']
    lock_path = project / 'package-lock.json'
    original = json.loads(lock_path.read_text())
    lock = json.loads(lock_path.read_text())
    entry = lock['packages']['node_modules/is-number']
    entry['integrity'] = 'sha512-' + 'A' * 86 + '=='
    lock_path.write_text(json.dumps(lock))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'integrity mismatch' in failed['diagnostics'][0]['message']
    entry['integrity'] = original['packages']['node_modules/is-number']['integrity']
    entry['resolved'] = 'https://unapproved.invalid/package.tgz'
    lock_path.write_text(json.dumps(lock))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'denied or failed' in failed['diagnostics'][0]['message']
    lock_path.write_text(json.dumps(original))
    package_path = project / 'package.json'
    package = json.loads(package_path.read_text())
    package['dependencies']['is-number'] = '6.0.0'
    package_path.write_text(json.dumps(package))
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['actions'] and not failed['artifacts']
    assert 'npm ci failed' in failed['diagnostics'][0]['message']
    verified.append('Node registry acquisition: deterministic lock-bound tarballs, native offline install/tests, verified snapshot package, bad SRI/stale lock/unapproved source failures')


def verify_defaults(root, base, invoke, validate, source_files, verified):
    project = base / 'node-default-tests'
    shutil.copytree(root / 'examples/builds/node-package/project', project)
    package_file = project / 'package.json'
    package = json.loads(package_file.read_text())
    del package['scripts']['test']
    package_file.write_text(json.dumps(package, indent=2) + '\n')
    before = source_files(project)
    listing = invoke(project, 'run', 'list', '--json')
    assert listing['project:test']['argv'] == ['node', '--test']
    assert listing['project:test']['build_stage']
    invoke(project, 'build')
    manifest = validate(project / 'dist')
    assert manifest['status'] == 'succeeded' and source_files(project) == before
    assert manifest['targets'][0]['extensions']['oyzu.dev/discovery']['test-framework']['selected'] == 'node-test'
    tests = next(r for r in manifest['reports'] if r['kind'] == 'test')
    coverage = next(r for r in manifest['reports'] if r['kind'] == 'coverage')
    assert tests['summary']['passed'] == 2 and coverage['summary']['covered'] > 0
    sources = [line[3:] for line in (project / 'dist' / coverage['path']).read_text().splitlines() if line.startswith('SF:')]
    assert sources and any(p.endswith('src/greeting.mjs') for p in sources)
    assert all('/test/' not in p and '/tests/' not in p and '.test.' not in p for p in sources)
    invoke(project, 'inspect', 'dist')
    # A known configuration wins over an unused dependency convention, but an
    # unimplemented integration must not silently run a different framework.
    (project / 'vitest.config.ts').write_text("throw new Error('discovery must not execute this config');\n")
    invoke(project, 'build', success=False)
    unsupported = validate(project / 'dist')
    assert not unsupported['actions'] and not unsupported['artifacts']
    assert 'vitest' in unsupported['diagnostics'][0]['message']
    (project / 'vitest.config.ts').unlink()
    (project / 'test/failing.test.mjs').write_text("import test from 'node:test'; test('intentional failure',()=>{throw new Error('expected')});\n")
    invoke(project, 'build', success=False)
    failed = validate(project / 'dist')
    assert not failed['artifacts']
    assert next(r for r in failed['reports'] if r['kind'] == 'test')['summary']['failed'] == 1
    assert all(r['status'] == 'collected' for r in failed['reports'])
    # No tests must remain a callable task and retain real empty native reports.
    shutil.rmtree(project / 'test')
    invoke(project, 'build')
    empty = validate(project / 'dist')
    assert next(r for r in empty['reports'] if r['kind'] == 'test')['summary']['total'] == 0
    assert next(r for r in empty['reports'] if r['kind'] == 'coverage')['summary']['total'] == 0
    # A custom script is preserved and must meet the same report obligations.
    package['scripts']['test'] = 'node custom-harness.mjs'
    package_file.write_text(json.dumps(package, indent=2) + '\n')
    (project / 'custom-harness.mjs').write_text("console.log('zero exit without test evidence');\n")
    invoke(project, 'build', success=False)
    missing = validate(project / 'dist')
    assert not missing['artifacts'] and len(missing['reports']) == 2
    assert all(r['status'] == 'invalid' for r in missing['reports'])
    action = next(a for a in missing['actions'] if a['id'] == 'project:test')
    assert action['status'] == 'failed'
    verified.append('Node framework detectors: no-config native tests, JUnit/application coverage, real empty reports, failed tests, unsupported framework admission and custom-script evidence obligations')


def verify_declared_reports(root, base, invoke, validate, source_files, verified):
    example = root / "examples/tasks/overrides-and-hooks"
    project = base / "node-declared-reports"
    shutil.copytree(example / "project", project)
    config = project / "oyzu.toml"
    declaration = (example / "variants/reports.oyzu.toml").read_text()
    config.write_text(declaration)
    before = source_files(project)
    invoke(project, "build")
    manifest = validate(project / "dist")
    assert source_files(project) == before
    assert manifest["status"] == "succeeded" and manifest["artifacts"]
    assert {r["kind"] for r in manifest["reports"]} == {"test", "coverage"}
    assert next(r for r in manifest["reports"] if r["kind"] == "test")["summary"]["passed"] == 3

    # Failed native tests still deliver their report; post is skipped, while
    # collection runs at that boundary and retains both JUnit and coverage.
    hooks = '\n[tasks."api:post_test"]\nargv=["node","hook.mjs","post"]\n'
    config.write_text('[env]\nFAILURE_POINT="main"\n' + declaration + hooks)
    invoke(project, "build", success=False)
    manifest = validate(project / "dist")
    assert not manifest["artifacts"]
    assert next(a for a in manifest["actions"] if a["id"] == "api:post_test")["status"] == "blocked"
    assert all(r["status"] == "collected" for r in manifest["reports"])
    assert next(r for r in manifest["reports"] if r["kind"] == "test")["summary"]["failed"] == 1

    # Only the post-hook creates the declared final location. No collection may
    # occur between the main process and that hook.
    runner = project / "api/scripts/custom-tests.mjs"
    original_runner = runner.read_text()
    runner.write_text(original_runner.replace('const junit = process.env.OYZU_TEST_REPORT ?? "reports/tests.xml";',
                                             'const junit = process.env.OYZU_TEST_REPORT + ".pending";'))
    finalizer = project / "api/finalize.mjs"
    finalizer.write_text("import {renameSync} from 'node:fs';\nrenameSync(process.env.OYZU_TEST_REPORT+'.pending',process.env.OYZU_TEST_REPORT);\n")
    config.write_text(declaration + '\n[tasks."api:post_test"]\nargv=["node","finalize.mjs"]\n')
    before = source_files(project)
    invoke(project, "build")
    manifest = validate(project / "dist")
    assert source_files(project) == before
    assert all(r["status"] == "collected" for r in manifest["reports"])
    assert next(a for a in manifest["actions"] if a["id"] == "api:post_test")["status"] == "succeeded"

    finalizer.write_text(finalizer.read_text() + "process.exitCode=4;\n")
    invoke(project, "build", success=False)
    manifest = validate(project / "dist")
    assert not manifest["artifacts"]
    assert all(r["status"] == "collected" for r in manifest["reports"])
    assert next(a for a in manifest["actions"] if a["id"] == "api:post_test")["exitCode"] == 4
    invoke(project, "inspect", "dist")

    # Each glob member comes from a real native test invocation. Collect all
    # members, in deterministic order, retaining malformed members as evidence.
    config.write_text(declaration.replace('reports/tests.xml', 'reports/**/*.xml'))
    runner.write_text("""import {spawnSync} from 'node:child_process';
import {mkdirSync} from 'node:fs';
mkdirSync('reports/nested',{recursive:true});
const cases=[['test/greeting.test.mjs','reports/greeting.xml'],['test/failure.test.mjs','reports/nested/failure.xml']];
for(const [file,report] of cases){
  const flags=['--test','--test-reporter=junit',`--test-reporter-destination=${report}`];
  if(file.includes('greeting')) flags.push('--experimental-test-coverage','--test-reporter=lcov',`--test-reporter-destination=${process.env.OYZU_COVERAGE_REPORT}`);
  const result=spawnSync(process.execPath,[...flags,file],{stdio:'inherit'});
  if(result.status!==0) process.exitCode=result.status ?? 1;
}
""")
    before = source_files(project)
    invoke(project, "build")
    manifest = validate(project / "dist")
    assert source_files(project) == before
    reports = [r for r in manifest["reports"] if r["kind"] == "test"]
    assert len(reports) == 2 and sum(r["summary"]["passed"] for r in reports) == 3
    assert len({r["id"] for r in reports}) == 2
    runner.write_text(runner.read_text() + "\nconst fs=await import('node:fs');fs.writeFileSync('reports/broken.xml','incomplete XML');\n")
    invoke(project, "build", success=False)
    manifest = validate(project / "dist")
    assert not manifest["artifacts"]
    invalid = [r for r in manifest["reports"] if r["status"] == "invalid"]
    assert len(invalid) == 1
    assert (project / 'dist' / invalid[0]['path']).read_text() == 'incomplete XML'
    invoke(project, "inspect", "dist")
    verified.append("Declared reports: native JUnit/coverage, captured cwd, globs, post finalization and failure evidence")
