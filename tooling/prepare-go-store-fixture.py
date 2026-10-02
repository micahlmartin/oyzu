#!/usr/bin/env python3
"""Inventory pinned Go ZIP; original archive/notices stay outside the checkout.

Identity comes from the independently captured official catalog and sidecar;
this is not publisher signature verification or permission to distribute.
"""
import argparse
from pathlib import Path
from zip_fixture import prepare

PINS = {
    "1.24.13": ("40b16bc8f00540a2cb02dff4de72b73e966fdd8d65f95e33d8e4080b48a2459a", 87295983),
    "1.25.0": ("89efb4f9b30812eee083cc1770fdd2913c14d301064f6454851428f9707d190b", 67418204),
}

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--version", choices=sorted(PINS), default="1.24.13")
    args = parser.parse_args()
    digest, size = PINS[args.version]
    prepare(args.archive, args.manifest, f"go{args.version}.windows-amd64.zip",
            digest, size, "go/", args.version)
