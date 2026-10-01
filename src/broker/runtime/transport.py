"""Credential-free client for the engine's private, scoped acquisition channel."""
import json
from pathlib import Path
import time
import uuid


def fetch(url, root=Path('/broker')):
    request_id = uuid.uuid4().hex
    temporary = root / (request_id + '.pending')
    temporary.write_text(json.dumps({'url': url}))
    temporary.rename(root / (request_id + '.request'))
    response = root / (request_id + '.response')
    deadline = time.monotonic() + 55
    while not response.exists():
        if time.monotonic() > deadline:
            raise TimeoutError('scoped acquisition broker did not respond')
        time.sleep(.01)
    info = json.loads(response.read_text())
    body_path = root / (request_id + '.body')
    body = body_path.read_bytes()
    response.unlink()
    body_path.unlink()
    return info, body
