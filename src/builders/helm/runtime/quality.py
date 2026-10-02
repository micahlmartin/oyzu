"""Native formatting of chart metadata, values and suites; templates remain untouched."""
import argparse
import os
from pathlib import Path
import stat
import subprocess
import tempfile

CONFIGS = ('.yamlfmt', '.yamlfmt.yaml', '.yamlfmt.yml', 'yamlfmt.yaml', 'yamlfmt.yml')
SKIP = {'.git', '.oyzu', '.oyzu-build', 'dist', 'node_modules', 'templates', '__snapshot__'}


def files(root):
    selected, charts = [], set()
    count = 0
    def failed(error):
        raise error
    for directory, dirs, names in os.walk(root, followlinks=False, onerror=failed):
        path = Path(directory)
        dirs[:] = sorted(d for d in dirs if d not in SKIP and not d.startswith('.'))
        for name in dirs + names:
            count += 1
            item = path/name
            if count > 100000 or item.is_symlink() or bool(getattr(item.lstat(), 'st_file_attributes', 0) & 0x400):
                raise ValueError('Helm formatting inputs exceed limits or contain links')
        if 'Chart.yaml' in names:
            charts.add(path)
        if any(name in CONFIGS for name in names):
            raise ValueError('Custom yamlfmt configuration needs an explicit format/format-check task override; implicit chart formatting currently uses fixed native defaults')
        in_chart_data = any(parent in path.parents and path.relative_to(parent).parts[0] in ('tests', 'crds') for parent in charts)
        if path in charts or in_chart_data:
            for name in sorted(names):
                if name.endswith(('.yaml', '.yml')):
                    item = path/name
                    if not stat.S_ISREG(item.lstat().st_mode):
                        raise ValueError('Helm formatting requires regular YAML files')
                    selected.append(item)
    return sorted(selected)


def run(mode, root, executable='yamlfmt'):
    root = root.resolve(strict=True)
    inputs = files(root)
    if not inputs:
        raise ValueError('No chart YAML inputs were selected for formatting')
    # An explicit private configuration avoids parent/home configuration and fixes
    # LF across hosts. Native defaults own YAML representation and indentation.
    with tempfile.TemporaryDirectory(prefix='oyzu-helm-format-') as temporary:
        config = Path(temporary)/'yamlfmt.yaml'
        config.write_text('line_ending: lf\nformatter:\n  type: basic\n  eof_newline: true\n')
        version = subprocess.run([executable, '-version'], capture_output=True, check=True, timeout=30)
        print(version.stdout.decode('utf-8').strip(), flush=True)
        changed = False
        total = 0
        for path in inputs:
            with path.open('rb') as stream:
                original = stream.read(4*1024*1024+1)
            total += len(original)
            if len(original) > 4*1024*1024 or total > 64*1024*1024:
                raise ValueError('Helm formatting input exceeds byte limits')
            result = subprocess.run([executable, '-conf', str(config), '-in'], input=original,
                                    capture_output=True, timeout=30)
            if result.returncode:
                raise ValueError(f'{path.relative_to(root)}: {result.stderr.decode("utf-8", errors="replace")}')
            if original != result.stdout:
                changed = True
                print(f'Formatting differs: {path.relative_to(root).as_posix()}', flush=True)
                if mode == 'format':
                    if path.is_symlink() or path.read_bytes() != original:
                        raise ValueError('Helm source changed during formatting')
                    path.write_bytes(result.stdout)
        print(f'Helm YAML {mode}: {len(inputs)} files; templates, packaged charts and snapshots excluded', flush=True)
        return int(changed and mode == 'format-check')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=['format', 'format-check'])
    parser.add_argument('root', type=Path)
    args = parser.parse_args()
    raise SystemExit(run(args.mode, args.root))
