"""Native lock fixtures for Python container-store acceptance, never product rules."""
IMAGES = {
    'uv': 'ghcr.io/astral-sh/uv:0.12.21-python3.12-trixie-slim',
    'poetry': 'oyzu-toolchain/poetry:2.5.1-python3.12',
}


def create(project, manager):
    project.mkdir()
    native = '''[project]
name = "oyzu-context-fixture"
version = "1.0.0"
requires-python = ">=3.12,<3.13"
dependencies = ["six==1.17.0"]
'''
    if manager == 'uv':
        native += '''[dependency-groups]
dev = ["idna==3.10"]
docs = ["packaging==24.2"]
[tool.uv]
default-groups = ["dev", "docs"]
'''
    elif manager == 'poetry':
        native += '''[tool.poetry]
package-mode = false
[tool.poetry.group.dev.dependencies]
idna = "3.10"
[tool.poetry.group.docs.dependencies]
packaging = "24.2"
'''
    else:
        raise ValueError(manager)
    (project / 'pyproject.toml').write_text(native, encoding='utf-8')


def lock_command(manager, python):
    if manager == 'uv':
        return ['uv', 'lock', '--no-python-downloads', '--no-managed-python', '--python', python]
    if manager == 'poetry':
        return ['poetry', '--no-plugins', 'lock', '--no-interaction']
    raise ValueError(manager)
