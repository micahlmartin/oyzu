#!/usr/bin/env python3
"""Inventory a pinned Temurin ZIP without extracting or executing it.

Preserve the original archive and its legal files outside the checkout.
The official release checksum pins fixture identity, not distribution approval.
"""
from zip_fixture import run_cli

if __name__ == "__main__":
    run_cli(
        "OpenJDK21U-jdk_x64_windows_hotspot_21.0.6_7.zip",
        "897c8eebb0f85a99ccecbd482ebae9a45d88c19d6077054f6529ebab49b6d259",
        204643847, "jdk-21.0.6+7/", "temurin-21.0.6+7",
    )
