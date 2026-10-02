"""Native npm inputs shared by host protocol and captured Docker acceptance."""
import json

IMAGE = 'oyzu-toolchain/node:npm11.11.0-node22'
DOCKERFILE = f'''FROM {IMAGE}
WORKDIR /app
COPY package.json package-lock.json ./
RUN --mount=type=bind,from=dependencies,target=/dependencies \\
    for archive in /dependencies/*.tgz; do npm cache add "$archive" --offline --cache /tmp/npm-cache --ignore-scripts; done \\
    && npm ci --offline --cache /tmp/npm-cache --ignore-scripts --omit=dev --no-audit --no-fund \\
    && rm -rf /tmp/npm-cache
RUN node -e "if (!require('is-odd')(3)) process.exit(1)"
CMD ["node", "-e", "console.log(require('is-odd')(3))"]
'''


def create(project):
    project.mkdir()
    (project / 'package.json').write_text(json.dumps({
        'name': 'oyzu-npm-context', 'version': '1.0.0', 'private': True,
        'packageManager': 'npm@11.11.0', 'engines': {'node': '>=22'},
        'dependencies': {'is-odd': '3.0.1'},
        'devDependencies': {'picocolors': '1.1.1'},
        'scripts': {'preinstall': 'node -e "process.exit(91)"'},
    }, indent=2) + '\n', encoding='utf-8')


def lock_arguments(cache):
    # Fixture provisioning only; acquisition and replay must stay offline except
    # for the product's separately authorized broker requests.
    return ['install', '--package-lock-only', '--ignore-scripts', '--no-audit',
            '--no-fund', '--registry=https://registry.npmjs.org', '--cache', str(cache)]
