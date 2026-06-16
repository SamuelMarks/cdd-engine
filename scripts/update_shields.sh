#!/usr/bin/env bash

# Calculate test coverage using tarpaulin
if ! command -v cargo-tarpaulin &> /dev/null
then
    echo "cargo-tarpaulin could not be found."
    exit 1
fi

echo "Running tests and calculating coverage..."
COVERAGE=$(cargo tarpaulin --engine llvm --timeout 120 --out Lcov 2>&1 | grep -oE "[0-9.]+% coverage" | awk '{print $1}' | tr -d '%')
if [ -z "$COVERAGE" ]; then
    COVERAGE="0"
fi
COVERAGE_ROUNDED=$(printf "%.0f" "$COVERAGE")

# Calculate doc coverage using nightly rustdoc
DOC_COVERAGE_RAW=$(cargo +nightly rustdoc --lib -- -Z unstable-options --show-coverage 2>&1 | grep "Total" | awk '{print $6}' | tr -d '%')
if [ -z "$DOC_COVERAGE_RAW" ]; then
    DOC_COVERAGE_RAW="0"
fi
DOC_COVERAGE=$(printf "%.0f" "$DOC_COVERAGE_RAW")

TEST_COLOR="success"
DOC_COLOR="success"

if [ "$COVERAGE_ROUNDED" -ne 100 ]; then
    TEST_COLOR="red"
fi
if [ "$DOC_COVERAGE" -ne 100 ]; then
    DOC_COLOR="red"
fi

# Update README.md
sed -i -E "s/!\[Test Coverage\]\(https:\/\/img\.shields\.io\/badge\/coverage-[0-9.]+.*?%25-.*?\.svg\)/![Test Coverage](https:\/\/img.shields.io\/badge\/coverage-${COVERAGE_ROUNDED}%25-${TEST_COLOR}.svg)/g" README.md
sed -i -E "s/!\[Doc Coverage\]\(https:\/\/img\.shields\.io\/badge\/docs-[0-9.]+.*?%25-.*?\.svg\)/![Doc Coverage](https:\/\/img.shields.io\/badge\/docs-${DOC_COVERAGE}%25-${DOC_COLOR}.svg)/g" README.md

echo "Updated shields in README.md (Test: ${COVERAGE_ROUNDED}%, Doc: ${DOC_COVERAGE}%)"

if command -v git &> /dev/null; then
    git add README.md
fi

if [ "$COVERAGE_ROUNDED" -ne 100 ] || [ "$DOC_COVERAGE" -ne 100 ]; then
    echo "Coverage requirement not met (74% required)."
    exit 1
fi
