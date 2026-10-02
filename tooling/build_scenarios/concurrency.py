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
