"""Real bounded concurrency and private output roots through the compiled CLI."""
import json
import shutil


def verify(root, base, invoke, validate, source_files, verified):
    project = base / 'configuration-concurrency'
    project.mkdir()
    for target in ['alpha', 'beta']:
        shutil.copytree(root / 'examples/builds/node-package/project', project / target)
    (project / 'build.yaml').write_text(
        'alpha: {uses: node/package, path: alpha}\n'
        'beta: {uses: node/package, path: beta}\n')
    script = """const fs = require('node:fs');
const target = process.argv[1];
if (JSON.stringify(fs.readdirSync('/out')) !== JSON.stringify([target])) process.exit(1);
const start = Date.now();
setTimeout(() => console.log('OYZU_CONCURRENCY ' + JSON.stringify({start, end: Date.now()})), 4000);
"""
    for jobs in [1, 2]:
        config = f'[build]\njobs={jobs}\n'
        for target in ['alpha', 'beta']:
            config += f'[tasks."{target}:pre_test"]\nargv = {json.dumps(["node", "-e", script, target])}\n'
        (project / 'oyzu.toml').write_text(config)
        before = source_files(project)
        invoke(project, 'build')
        bundle = project / 'dist'
        manifest = validate(bundle)
        assert manifest['status'] == 'succeeded'
        assert before == source_files(project)
        plan = json.loads((bundle / manifest['planPath']).read_text())
        assert plan['extensions']['oyzu.dev/execution']['jobs'] == jobs
        intervals = []
        for index, action in enumerate(plan['actions']):
            if action['id'] in ['alpha:pre_test', 'beta:pre_test']:
                lines = (bundle / 'logs' / f'{index:04}.stdout').read_text().splitlines()
                intervals.append(json.loads(next(line.removeprefix('OYZU_CONCURRENCY ')
                    for line in lines if line.startswith('OYZU_CONCURRENCY '))))
        assert len(intervals) == 2
        overlap = max(x['start'] for x in intervals) < min(x['end'] for x in intervals)
        assert overlap == (jobs == 2), intervals
    verified.append('build.jobs 1/2 controls actual action overlap with private target output roots')
    # A qualified prerequisite runs once in its owner's workspace. Ordering
    # alone must not expose the owner's mutated files to the consumer.
    proof = "const fs=require('node:fs'); if(fs.existsSync('proof.txt'))process.exit(1); fs.writeFileSync('proof.txt','beta'); console.log('OYZU_PROOF_ONCE');"
    post = "if(require('node:fs').readFileSync('proof.txt','utf8')!=='beta')process.exit(1);"
    consumer = "if(require('node:fs').existsSync('../beta/proof.txt'))process.exit(1);"
    config = '[build]\njobs=2\n'
    config += '[tasks."beta:proof"]\nargv='+json.dumps(['node','-e',proof])+'\n'
    config += '[tasks."beta:post_proof"]\nargv='+json.dumps(['node','-e',post])+'\n'
    for target in ['alpha','beta']:
        config += f'[tasks."{target}:pre_test"]\ndepends_on=["beta:proof"]\nargv='+json.dumps(['node','-e',consumer if target=='alpha' else post])+'\n'
    (project/'oyzu.toml').write_text(config)
    before = source_files(project)
    invoke(project,'build')
    manifest = validate(project/'dist')
    assert manifest['status']=='succeeded' and source_files(project)==before
    plan = json.loads((project/'dist/plan.json').read_text())
    proofs = [a for a in plan['actions'] if a['id']=='beta:proof']
    assert len(proofs)==1 and proofs[0]['target']=='beta' and proofs[0]['cwd']=='beta'
    assert proofs[0]['tools']==['beta']
    for target in ['alpha','beta']:
        action = next(a for a in plan['actions'] if a['id']==f'{target}:pre_test')
        assert 'beta:post_proof' in action['dependsOn']
    assert len(manifest['artifacts'])==2
    assert len([r for r in manifest['reports'] if r['kind']=='test'])==2
    invoke(project,'inspect','dist')
    # A failing producer post-hook blocks both consumers and packages.
    (project/'oyzu.toml').write_text(config.replace(
        'argv='+json.dumps(['node','-e',post]),
        'argv='+json.dumps(['node','-e','process.exit(7)']), 1))
    invoke(project,'build',success=False)
    failed = validate(project/'dist')
    assert not failed['artifacts']
    for target in ['alpha','beta']:
        assert next(a for a in failed['actions'] if a['id']==f'{target}:pre_test')['status']=='blocked'
    verified.append('Cross-target task prerequisites: one owned execution, private workspaces, post-hook ordering, per-target reports/artifacts and failed-hook gating')
