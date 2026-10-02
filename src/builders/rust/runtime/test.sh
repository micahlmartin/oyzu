#!/bin/sh
# Test and reporting outcomes are independent. Never replace a failing test exit
# code with a successful report-generation exit code.
cargo llvm-cov nextest --no-report --workspace --locked --offline --config-file /dependencies/nextest.toml --profile default
test_status=$?
cargo llvm-cov report --locked --offline --cobertura --output-path "$1"
report_status=$?
doctest_status=0
if [ "$#" -gt 1 ]; then
    shift
    python3 -I /oyzu/rust-doctest.py "$@"
    doctest_status=$?
fi
if [ "$test_status" -ne 0 ]; then
    exit "$test_status"
fi
if [ "$doctest_status" -ne 0 ]; then
    exit "$doctest_status"
fi
exit "$report_status"
