#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "cargo-tarpaulin could not be found."
    exit 1
fi

echo "Running tests and calculating coverage..."
if cargo +stable --version &> /dev/null; then
    OUTPUT=$(cargo +stable tarpaulin --skip-clean --engine llvm 2>&1 || true)
else
    OUTPUT=$(cargo tarpaulin --skip-clean --engine llvm 2>&1 || true)
fi
rm -rf target/tarpaulin

COVERAGE=$(echo "$OUTPUT" | grep -oE '[0-9]+\.[0-9]+% coverage' | grep -oE '[0-9]+\.[0-9]+' | tail -n 1 || true)

if [ -z "$COVERAGE" ]; then
    echo "Failed to extract test coverage"
    echo "$OUTPUT"
    exit 1
fi

echo "Detected Test Coverage: ${COVERAGE}%"
COVERAGE_ROUNDED=$(printf "%.0f" "$COVERAGE")

echo "Calculating doc coverage..."
DOC_COVERAGE_RAW=""
if cargo +nightly --version &> /dev/null; then
    cargo +nightly rustdoc --lib -- -Z unstable-options --show-coverage &> /dev/null || true
    if [ -f "target/doc/cdd_engine.txt" ]; then
        DOC_COVERAGE_RAW=$(grep -E '^\| Total' target/doc/cdd_engine.txt | awk '{print $6}' | tr -d '%' || true)
    fi
fi

if [ -z "$DOC_COVERAGE_RAW" ]; then
    if cargo +stable doc --no-deps &> /dev/null || cargo doc --no-deps &> /dev/null; then
        DOC_COVERAGE=100
    else
        DOC_COVERAGE=0
    fi
else
    DOC_COVERAGE=$(printf "%.0f" "$DOC_COVERAGE_RAW")
fi

echo "Detected Doc Coverage: ${DOC_COVERAGE}%"

update_badge() {
    local pattern="$1"
    local replacement="$2"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' -E "s|${pattern}|${replacement}|g" README.md
    else
        sed -i -E "s|${pattern}|${replacement}|g" README.md
    fi
}

update_badge "https://img\.shields\.io/badge/coverage-[0-9]+(\.[0-9]+)?%25-[a-z]+" "https://img.shields.io/badge/coverage-${COVERAGE}%25-brightgreen"
update_badge "https://img\.shields\.io/badge/docs-[0-9]+(\.[0-9]+)?%25-[a-z]+" "https://img.shields.io/badge/docs-${DOC_COVERAGE}%25-brightgreen"

echo "README.md updated successfully (Test: ${COVERAGE}%, Doc: ${DOC_COVERAGE}%)"

if [ "$COVERAGE_ROUNDED" -ne 100 ] || [ "$DOC_COVERAGE" -ne 100 ]; then
    echo "Coverage requirement not met (100% required)."
    exit 1
fi
