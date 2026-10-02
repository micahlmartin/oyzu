"""Bounded downloads for explicit CI tool provisioning, not product acquisition."""
import http.client
import time
import urllib.error
import urllib.request


def download(url, limit):
    """Retry transient transport failures; callers still verify pinned digests.

    Each attempt has a 60-second socket timeout and a bounded response body.
    At most three attempts are made; this is not a total wall-clock deadline.
    No partial bytes are returned or reused after a failed attempt.
    """
    for attempt in range(3):
        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                expected = response.headers.get('Content-Length')
                body = response.read(limit + 1)
            if len(body) > limit:
                raise ValueError('Provisioned asset exceeds its download limit')
            if expected is not None and len(body) < int(expected):
                raise http.client.IncompleteRead(body, int(expected) - len(body))
            return body
        except urllib.error.HTTPError as error:
            error.close()
            if error.code not in (429, 500, 502, 503, 504) or attempt == 2:
                raise
        except (urllib.error.URLError, TimeoutError, ConnectionError,
                http.client.IncompleteRead):
            if attempt == 2:
                raise
        time.sleep(2 ** attempt)
