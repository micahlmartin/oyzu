"""Native nextest, coverage and doctest outcomes retain independent failures."""
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import tomllib
import uuid


def execute(config, profile, coverage, doctest, packages, extra):
    test_status = subprocess.run(['cargo', 'llvm-cov', 'nextest', '--no-report', '--workspace',
                                  '--locked', '--offline', '--config-file', str(config),
                                  '--profile', profile, *extra]).returncode
    report_status = subprocess.run(['cargo', 'llvm-cov', 'report', '--locked', '--offline',
                                    '--cobertura', '--output-path', coverage]).returncode
    doctest_status = 0
    if packages:
        doctest_status = runpy.run_path(str(Path(__file__).with_name('rust-doctest.py')))['run'](
            packages, Path(doctest))
    return test_status or doctest_status or report_status


def host(config, junit, coverage, doctest, packages, extra):
    # Explicit native prerequisites prevent llvm-cov from offering tool downloads.
    suffix = '.exe' if os.name == 'nt' else ''
    lib = Path(subprocess.check_output(['rustc', '--print', 'target-libdir'], text=True).strip())
    for key, tool in [('LLVM_COV', 'llvm-cov'), ('LLVM_PROFDATA', 'llvm-profdata')]:
        location = os.environ.get(key) or str(lib.parent/'bin'/(tool+suffix))
        if not shutil.which(location):
            raise RuntimeError('Provision llvm-tools-preview before Rust tests: missing '+tool)
        os.environ[key] = location
    with tempfile.TemporaryDirectory(prefix='oyzu-rust-test-') as temporary:
        root = Path(temporary)
        os.environ['CARGO_TARGET_DIR'] = str(root/'target')
        os.environ['CARGO_LLVM_COV_TARGET_DIR'] = str(root/'coverage-target')
        # A private native profile inherits all project default settings. Only
        # report destinations/applicability are supplied here, without a TOML
        # reimplementation or editing the user's configuration.
        profile = 'oyzu-host-' + uuid.uuid4().hex
        native = tomllib.loads(config)
        skipped = native.get('profile', {}).get('default', {}).get('junit', {}).get('report-skipped', 'ignored')
        overlay = config + f'\n[profile.{profile}.junit]\npath = {json.dumps(junit, ensure_ascii=False)}\nreport-skipped = {json.dumps(skipped)}\n'
        configuration = root/'nextest.toml'
        configuration.write_text(overlay, encoding='utf-8', newline='\n')
        return execute(configuration, profile, coverage, doctest, packages, extra)


if __name__ == '__main__':
    mode, config, junit, coverage, doctest, encoded, *extra = sys.argv[1:]
    packages = json.loads(encoded)
    if mode == 'host':
        status = host(config, junit, coverage, doctest, packages, extra)
    elif mode == 'captured':
        status = execute(config, 'default', coverage, doctest, packages, extra)
    else:
        raise ValueError('Unknown Rust test execution mode')
    raise SystemExit(status)
