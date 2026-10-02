"""Collect pytest results and coverage for the installed build artifact."""
import argparse
import configparser
from importlib.metadata import distribution
import os
from pathlib import Path, PurePosixPath
import tomllib


def native_configuration(root):
    """Preserve native settings while overriding the artifact source selection."""
    explicit = os.environ.get('COVERAGE_RCFILE')
    names = [explicit] if explicit else ['.coveragerc', '.coveragerc.toml', 'setup.cfg', 'tox.ini', 'pyproject.toml']
    for name in names:
        path = root / name
        if not path.is_file():
            if explicit:
                raise ValueError('Configured coverage file does not exist')
            continue
        result = configparser.ConfigParser(interpolation=None)
        if path.suffix == '.toml':
            data = tomllib.loads(path.read_text())
            data = data.get('tool', {}).get('coverage', {}) if path.name == 'pyproject.toml' or 'tool' in data else data
            for section, options in data.items():
                if not isinstance(options, dict):
                    raise ValueError('Invalid native coverage section: ' + section)
                result[section] = {}
                for key, value in options.items():
                    if isinstance(value, list):
                        value = '\n'.join(str(item) for item in value)
                    elif isinstance(value, bool):
                        value = str(value).lower()
                    result[section][key] = str(value)
        else:
            original = configparser.ConfigParser(interpolation=None)
            original.read(path, encoding='utf-8')
            prefix = 'coverage:' if path.name in {'setup.cfg', 'tox.ini'} else ''
            for section in original.sections():
                if section.startswith(prefix):
                    result[section[len(prefix):]] = dict(original[section])
        if result.sections() or explicit or path.name == '.coveragerc':
            return result
    return configparser.ConfigParser(interpolation=None)


def installed_sources(name):
    """Read wheel RECORD entries without importing application code before tracing."""
    package = distribution(name)
    modules = set()
    files = set()
    for entry in package.files or []:
        relative = PurePosixPath(str(entry).replace('\\', '/'))
        if relative.is_absolute() or '..' in relative.parts or relative.suffix != '.py':
            continue
        first = relative.parts[0]
        module = first[:-3] if len(relative.parts) == 1 else first
        if not module.isidentifier():
            continue
        path = Path(package.locate_file(entry)).resolve()
        if path.is_file():
            modules.add(module)
            files.add(str(path))
    if not files:
        raise ValueError('Installed distribution has no measurable Python source')
    return sorted(modules), sorted(files)


def coverage_configuration(name, destination, sources=None):
    modules, files = installed_sources(name) if sources is None else sources
    config = native_configuration(Path.cwd())
    for section in ['run', 'report']:
        if not config.has_section(section):
            config.add_section(section)
    # source_pkgs explicitly means importable modules, even when the checkout has
    # a directory with the same spelling. It also supports single-file modules.
    config['run'].pop('source', None)
    config['run'].pop('source_dirs', None)
    config['run']['source_pkgs'] = '\n'.join(modules)
    if 'include' not in config['report']:
        config['report']['include'] = '\n'.join(files)
    with destination.open('w', encoding='utf-8') as output:
        config.write(output)


def run_tests(name, junit, coverage, extra=(), *, sources=None):
    import pytest
    config = Path('.oyzu-build/coverage.ini').resolve()
    config.parent.mkdir(parents=True, exist_ok=True)
    coverage_configuration(name, config, sources)
    return pytest.main([
        '--import-mode=importlib', '--junitxml=' + str(junit),
        '--cov', '--cov-config=' + str(config), '--cov-report=xml:' + str(coverage),
        *extra,
    ])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--distribution', required=True)
    parser.add_argument('--junit', required=True)
    parser.add_argument('--coverage', required=True)
    args, extra = parser.parse_known_args()
    raise SystemExit(run_tests(args.distribution, args.junit, args.coverage, extra))
