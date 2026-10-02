"""Pinned setuptools compatibility integration, executed only in an offline worker."""
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import sysconfig


def distribution(commands):
    import setuptools  # Activate its distutils compatibility implementation.
    from distutils.core import run_setup
    if not Path('setup.py').is_file():
        raise ValueError('Legacy metadata requires setup.py')
    # Match native setup.py import semantics, inside the private worker only.
    sys.path.insert(0, str(Path.cwd()))
    result = run_setup('setup.py', script_args=commands, stop_after='commandline')
    if result.setup_requires:
        raise ValueError('Dynamic setup_requires needs declared build dependency acquisition')
    return result


def compiler_facts(dist):
    if not dist.has_ext_modules():
        return None
    if sys.platform != 'linux':
        raise ValueError('Native legacy compiler integration currently requires the Linux worker')
    include = Path(sysconfig.get_path('include'))
    if not (include/'Python.h').is_file():
        raise ValueError('Declared Python headers are unavailable')
    command = shlex.split(os.environ.get('CC') or sysconfig.get_config_var('CC') or '')
    if not command or not shutil.which(command[0]):
        raise ValueError('Declared C compiler is unavailable')
    result = subprocess.run([*command, '--version'], capture_output=True, text=True, check=True, timeout=15)
    return {'command':command, 'version':result.stdout.splitlines()[0], 'include':str(include), 'soabi':sysconfig.get_config_var('SOABI')}


def facts(dist):
    from packaging.version import Version
    wheel = dist.get_command_obj('bdist_wheel')
    wheel.ensure_finalized()
    egg = dist.get_command_obj('egg_info')
    egg.ensure_finalized()
    return {
        'name':re.sub(r'[-_.]+', '_', dist.get_name()).lower(),
        'version':str(Version(egg.egg_version)),
        'wheelTag':'-'.join(wheel.get_tag()),
        'compiler':compiler_facts(dist),
        'requirements':list(dist.install_requires or []),
        'extensions':[extension.name for extension in dist.ext_modules or []],
    }


def commands():
    source = os.environ['OYZU_SOURCE_DIGEST']
    if not re.fullmatch(r'sha256:[0-9a-f]{64}', source):
        raise ValueError('Invalid captured source identity')
    return ['egg_info', '--tag-build=.dev0+g'+source[7:19], '--no-date']


def metadata(destination):
    dist = distribution(commands())
    value = facts(dist)
    # Dynamic runtime requirements must be captured before an offline build can
    # use them. This compatibility profile does not grant metadata network access.
    if value['requirements']:
        raise ValueError('Legacy runtime requirements need metadata-driven acquisition')
    Path(destination).write_text(json.dumps(value, sort_keys=True)+'\n', encoding='utf-8')


def build(expected):
    recorded = json.loads(Path(expected).read_text())
    dist = distribution(commands()+['bdist_wheel', '--dist-dir=.oyzu-build/dist', 'sdist', '--dist-dir=.oyzu-build/dist'])
    observed = facts(dist)
    if observed != recorded or observed['version'] != os.environ['OYZU_VERSION']:
        raise ValueError('Legacy metadata/toolchain differs from the captured build plan')
    dist.run_commands()
    expected_names = {
        f"{recorded['name']}-{recorded['version']}-{recorded['wheelTag']}.whl",
        f"{recorded['name']}-{recorded['version']}.tar.gz",
    }
    if {p.name for p in Path('.oyzu-build/dist').iterdir()} != expected_names:
        raise ValueError('Legacy output identity differs from the captured build plan')


if __name__ == '__main__':
    if sys.argv[1]=='metadata':
        metadata(sys.argv[2])
    elif sys.argv[1]=='build':
        build(sys.argv[2])
    else:
        raise ValueError('Unknown legacy operation')
