#!/usr/bin/env python3
"""Inventory the pinned Node ZIP; no acquisition or execution."""
from zip_fixture import run_cli

NAME = "node-v22.14.0-win-x64.zip"
DIGEST = "55b639295920b219bb2acbcfa00f90393a2789095b7323f79475c9f34795f217"
SIZE = 34906389
PREFIX = "node-v22.14.0-win-x64/"


if __name__ == "__main__":
    run_cli(NAME, DIGEST, SIZE, PREFIX, "22.14.0")
