"""Authored producer/image matrices using actual provisioned ARM toolchains."""
import hashlib
import json
import shutil
import struct

from .node_fixtures import format_sources


def verify_platform_execution(root, base, invoke, validate, source_files, verified):
    from .docker import image_contents

    for scenario, producer, expected_kind in [
        ('container-variants', 'api', 'file'),
        ('materialize-directory', 'frontend', 'directory'),
    ]:
        project = base / f'platform-execution-{scenario}'
        shutil.copytree(root / 'examples/builds' / scenario / 'project', project,
                        ignore=shutil.ignore_patterns('node_modules', 'dist'))
        if producer == 'frontend':
            format_sources(root, project / producer)
        before = source_files(project)
        tasks = invoke(project, 'run', 'list', '--json')
        assert all(f'{producer}:{stage}' in tasks for stage in ['build', 'test', 'lint', 'format-check'])
        # Emulated compiler preparation can be slow. This is only the harness
        # deadline; product action/acquisition time limits remain enforced.
        invoke(project, 'build', 'image', timeout=2400)
        manifest = validate(project / 'dist')
        assert manifest['status'] == 'succeeded' and source_files(project) == before
        plan = json.loads((project / 'dist/plan.json').read_text())
        selected = plan['extensions']['oyzu.dev/selection']
        assert selected['requested'] == ['image']
        assert len(set(selected['variants'][producer]) & set(selected['selected'])) == 2
        assert len(manifest['artifacts']) == 5
        images = {}
        for platform in ['linux/amd64', 'linux/arm64']:
            target, = [t for t in manifest['targets']
                       if t['id'] in selected['variants'][producer] and t['variant']['platform'] == platform]
            image_target, = [t for t in manifest['targets']
                             if t['id'] in selected['variants']['image'] and t['variant']['platform'] == platform]
            artifact, = [a for a in manifest['artifacts'] if a['target'] == target['id']]
            image, = [a for a in manifest['artifacts'] if a['target'] == image_target['id']]
            assert artifact['kind'] == expected_kind and '-dev.g' in artifact['version']
            expected_platform = dict(zip(['os', 'arch'], platform.split('/')))
            assert target['platform'] == expected_platform
            for stage in ['build', 'test', 'lint', 'format-check', 'package']:
                identity = f"{target['id']}:{stage}"
                assert next(a for a in manifest['actions'] if a['id'] == identity)['status'] == 'succeeded'
                action = next(a for a in plan['actions'] if a['id'] == identity)
                assert action['executionPlatform'] == action['targetPlatform'] == expected_platform
            assert any(r['target'] == target['id'] and r['kind'] == 'test'
                       and r['summary']['passed'] > 0 for r in manifest['reports'])
            assert any(r['target'] == target['id'] and r['kind'] == 'coverage'
                       and r['summary']['covered'] > 0 for r in manifest['reports'])
            dependency = json.loads((project / f"dist/dependencies/{target['id']}.json").read_text())
            assert dependency['manager']['platform'] == expected_platform
            digest, config, contents = image_contents(project / 'dist' / image['path'])
            assert digest == image['ociDigest']
            assert f"{config['os']}/{config['architecture']}" == platform
            receipt, = json.loads((project / f"dist/inputs/{image_target['id']}.json").read_text())['inputs']
            assert receipt['digest'] == artifact['digest']
            if producer == 'api':
                binary = (project / 'dist' / artifact['path']).read_bytes()
                assert binary[:4] == b'\x7fELF' and binary[5] == 1
                assert struct.unpack_from('<H', binary, 18)[0] == (62 if platform.endswith('amd64') else 183)
                assert contents['server'] == binary
                assert hashlib.sha256(binary).hexdigest() == artifact['digest'].removeprefix('sha256:')
            else:
                assert 'site/index.html' in contents and 'site/dist/index.html' not in contents
                for entry in artifact['entries']:
                    if entry['kind'] == 'file':
                        assert contents[f"site/{entry['path']}"] == (project / 'dist' / artifact['path'] / entry['path']).read_bytes()
            images[platform] = digest
        index, = [a for a in manifest['artifacts'] if a['kind'] == 'oci-index']
        inspected = invoke(project, 'inspect', str(project / 'dist' / index['path']))
        assert {f"{p['os']}/{p['architecture']}" for p in inspected['platforms']} == set(images)
        invoke(project, 'inspect', 'dist')
        repeated = invoke(project, 'build', 'image', timeout=2400)
        assert repeated['planDigest'] == manifest['planDigest']
        assert {a['id']: a['digest'] for a in repeated['artifacts']} == {a['id']: a['digest'] for a in manifest['artifacts']}
        verified.append(f'{scenario}: authored two-platform graph, native-language tests/coverage and quality under matching amd64/ARM toolchains, versioned producer outputs, verified Docker materialization, complete OCI index and repeatability; remaining policy/ABI/cache cases are not covered')
