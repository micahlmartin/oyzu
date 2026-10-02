"""Run captured native quality tools while excluding private engine state.

Native tools retain ownership of config parsing, file selection and diagnostics.
This adapter runs only inside the prepared build environment, never discovery.
"""
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib


def main():
    tool, *arguments = sys.argv[1:]
    if tool == 'ruff':
        executable = Path(sys.executable).with_name('ruff.exe' if os.name == 'nt' else 'ruff')
        return subprocess.call([str(executable), *arguments])
    if tool == 'flake8':
        from flake8.main.application import Application
        app = Application()
        app.initialize(arguments)
        # Keep both native exclude lists, including configuration precedence.
        app.file_checker_manager.exclude += ('.oyzu-build',)
        app.options.exit_zero = False
        app.run_checks()
        app.report()
        return app.exit_code()
    if tool == 'black':
        import black
        data = tomllib.loads(Path('pyproject.toml').read_text(encoding='utf-8'))
        existing = data.get('tool', {}).get('black', {}).get('extend-exclude', '')
        # Append an alternative without changing native anchoring or regex flags.
        # Terminate verbose-mode trailing comments before the new alternative.
        if existing:
            compiled = black.re_compile_maybe_verbose(existing)
            separator = '\n' if compiled.flags & re.VERBOSE else ''
            exclusion = existing + separator + r'|/\.oyzu-build/'
        else:
            exclusion = r'/\.oyzu-build/'
        os.environ['BLACK_CACHE_DIR'] = str(Path('.oyzu-build/black-cache').resolve())
        black.main(args=[*arguments, '--extend-exclude', exclusion])
        return 0
    raise ValueError('Unsupported captured Python quality tool: ' + tool)


if __name__ == '__main__':
    sys.exit(main())
