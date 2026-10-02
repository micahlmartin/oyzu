"""Collect pytest results and coverage for the installed build artifact."""
import argparse
import configparser
from importlib.metadata import distribution
import os
from pathlib import Path, PurePosixPath
import tomllib
import tempfile
import sys


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


def run_host_tests(junit, coverage, extra=()):
    """Measure the checkout with native pytest-cov and fresh private data.

    The pytest-cov controller access is owned here and exercised with the
    provisioned integration version; the core engine never depends on it.
    """
    root = Path.cwd().resolve()
    # Match python -m pytest's checkout import path when this adapter is a file
    # outside the project. The native manager still owns the interpreter.
    sys.path.insert(0, str(root))
    import pytest
    config = native_configuration(root)
    if not config.has_section('run'):
        config.add_section('run')
    if not any(config['run'].get(name) for name in ['source', 'source_pkgs', 'source_dirs']):
        config['run']['source'] = str(root)
    omitted = config['run'].get('omit', '').splitlines()
    omitted.extend(['*/.oyzu/*', '*/.venv/*', '*/__pycache__/*'])
    config['run']['omit'] = '\n'.join(omitted)

    class ApplicationCoverage:
        @pytest.hookimpl(trylast=True)
        def pytest_collection_finish(self, session):
            plugin = session.config.pluginmanager.getplugin('_cov')
            if plugin is None or plugin.cov_controller is None:
                return  # Disabled/missing coverage remains missing evidence.
            for native in [plugin.cov_controller.cov, plugin.cov_controller.combining_cov]:
                if native is None:
                    continue
                omitted = list(native.get_option('report:omit') or [])
                omitted.extend(str(item.path.resolve()) for item in session.items)
                # Native filename patterns also exclude deselected test modules.
                omitted.extend('*/' + name for name in [*session.config.getini('python_files'), 'test_*.py', '*_test.py'])
                omitted.append('*/conftest.py')
                native.set_option('report:omit', sorted(set(omitted)))

    with tempfile.TemporaryDirectory(prefix='oyzu pytest ') as temporary:
        private = Path(temporary)
        config['run']['data_file'] = str(private/'coverage.data')
        settings = private/'coverage.ini'
        with settings.open('w', encoding='utf-8') as output:
            config.write(output)
        # Required output destinations win over user output-file options. Native
        # test selectors/options remain native; no project config is modified.
        previous = os.environ.get('COVERAGE_FILE')
        os.environ['COVERAGE_FILE'] = config['run']['data_file']
        try:
            return pytest.main([*extra, '--junitxml=' + str(junit), '--cov',
                '--cov-config=' + str(settings), '--cov-report=xml:' + str(coverage)],
                plugins=[ApplicationCoverage()])
        finally:
            if previous is None:
                os.environ.pop('COVERAGE_FILE', None)
            else:
                os.environ['COVERAGE_FILE'] = previous


if __name__ == '__main__':
    parser = argparse.ArgumentParser(allow_abbrev=False)
    parser.add_argument('--distribution')
    parser.add_argument('--host', action='store_true')
    parser.add_argument('--junit', required=True)
    parser.add_argument('--coverage', required=True)
    args, extra = parser.parse_known_args()
    if extra[:1] == ['--']:
        extra = extra[1:]
    if args.host:
        raise SystemExit(run_host_tests(args.junit, args.coverage, extra))
    if not args.distribution:
        parser.error('--distribution is required for installed-artifact tests')
    raise SystemExit(run_tests(args.distribution, args.junit, args.coverage, extra))
