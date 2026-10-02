#!/usr/bin/env python3
"""Inventory pinned Go ZIP; original archive/notices stay outside the checkout.

Identity comes from the independently captured official catalog and sidecar;
this is not publisher signature verification or permission to distribute.
"""
from zip_fixture import run_cli

if __name__ == "__main__":
    run_cli("go1.24.13.windows-amd64.zip",
            "40b16bc8f00540a2cb02dff4de72b73e966fdd8d65f95e33d8e4080b48a2459a",
            87295983, "go/", "1.24.13")
