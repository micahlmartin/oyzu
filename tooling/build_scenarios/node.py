"""Native Node task replacement evidence through the compiled CLI."""
import shutil


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
