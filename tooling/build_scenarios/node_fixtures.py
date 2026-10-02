"""Use the native formatter while authoring generated positive test inputs."""
import subprocess


def format_sources(root, *paths):
    # Deliberate fixture setup before source snapshots, never an invoke/build hook.
    # Negative formatting scenarios must retain their invalid input unchanged.
    subprocess.run(['node', str(root/'tooling/images/node-quality/node_modules/prettier/bin/prettier.cjs'), '--write', *map(str, paths)], check=True, capture_output=True, text=True)
