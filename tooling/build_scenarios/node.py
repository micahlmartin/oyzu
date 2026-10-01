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
