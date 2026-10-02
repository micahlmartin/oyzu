"""pnpm captured workspace acceptance through the compiled CLI."""
from .workspace_packages import verify_workspace


def verify(root, base, invoke, validate, source_files, verified):
    verify_workspace('pnpm', 'pnpm', root, base, invoke, validate, source_files, verified)
