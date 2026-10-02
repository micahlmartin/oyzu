"""Install Rust through Oyzu/mise, then run the published compiler and Cargo."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cli', type=Path, required=True)
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--offline-check', action='store_true')
    parser.add_argument('--restore-cached', action='store_true')
    parser.add_argument('--build-oyzu', type=Path)
    args = parser.parse_args()
    cli = args.cli.resolve(strict=True)
    root = args.workspace.resolve()
    project = root / 'project'
    store = root / 'store'
    if not args.offline_check:
        project.mkdir(parents=True)
        (project / 'oyzu.toml').write_text('[tools]\nrust = "1.95.0"\n', encoding='utf-8')
        (project / 'Cargo.toml').write_text('[package]\nname="rust-proof"\nversion="0.1.0"\nedition="2021"\n', encoding='utf-8')
        (project / 'src').mkdir()
        (project / 'src/main.rs').write_text('fn main() { println!("oyzu-rust:{}", std::env::args().nth(1).unwrap()); }\n', encoding='utf-8')
    environment = dict(os.environ)
    environment["CARGO_TARGET_DIR"] = str(root / "program-target")
    for name in ['RUSTC', 'RUSTDOC', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN']:
        environment.pop(name, None)

    suffix = '.exe' if os.name == 'nt' else ''
    environment['PATH'] = os.pathsep.join(directory for directory in environment.get('PATH', '').split(os.pathsep)
                                         if not any((Path(directory) / (name + suffix)).exists() for name in ['cargo', 'rustc', 'rustup']))
    assert shutil.which('cargo', path=environment['PATH']) is None

    def run(arguments):
        print('oyzu ' + ' '.join(arguments), flush=True)
        result = subprocess.run([str(cli), '-C', str(project)] + arguments, env=environment,
                                capture_output=True, text=True, timeout=1800)
        assert result.returncode == 0, (arguments, result.stdout, result.stderr)
        return result.stdout.strip()

    if args.restore_cached:
        assert args.offline_check
        (store / 'installs').rename(root / 'installs-before-restore')
    install = ['install', 'rust', '--store', str(store)]
    lock_before = (project / 'oyzu.lock').read_bytes() if args.offline_check else None
    run(install + (['--frozen', '--offline'] if args.offline_check else []))
    lock = (project / 'oyzu.lock').read_bytes()
    if lock_before is not None:
        assert lock == lock_before
    execute = ['exec', '--store', str(store), '--']
    for name in ['cargo', 'rustc', 'rustdoc']:
        executable = Path(run(['which', '--store', str(store), name]))
        assert executable.is_file() and store in executable.parents
        version = run(execute + [name, '--version'])
        assert version.startswith(name + ' 1.95.0'), version
    run(execute + ['cargo', 'build', '--offline'])
    assert run(execute + ['cargo', 'run', '--offline', '--quiet', '--', 'a b']) == 'oyzu-rust:a b'
    run(install + ['--frozen', '--offline'])
    if args.build_oyzu:
        run(execute + ['cargo', 'build', '--locked', '--manifest-path', str(args.build_oyzu.resolve() / 'Cargo.toml'), '--target-dir', str(root / 'oyzu-build')])
        assert (root / 'oyzu-build/debug' / ('oyzu.exe' if os.name == 'nt' else 'oyzu')).is_file()
    assert (project / 'oyzu.lock').read_bytes() == lock
    print(json.dumps({'verified': ['Rust installed through normal Oyzu command and mise backend',
                                  'published cargo/rustc/rustdoc report 1.95.0',
                                  'real Cargo compilation and program execution', 'unchanged frozen lock'] +
                     (['Oyzu compiled by installed Cargo'] if args.build_oyzu else [])}, indent=2))


if __name__ == '__main__':
    main()
