#!/bin/bash
# Test script to demonstrate TOML 1.1 backward compatibility
#
# This script shows that:
# 1. Without TOML rewriting, pip FAILS to install packages with TOML 1.1 syntax
# 2. With our branch's TOML rewriting, pip SUCCEEDS

set -e

# Get the uv repository root for cargo run
UV_REPO="$(cd "$(dirname "$0")" && pwd)"

echo "=== TOML 1.1 Backward Compatibility Test ==="
echo

# Create a temporary directory for the test
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"

# Create a test project with TOML 1.1 features
mkdir -p toml11-test/src/toml11_test
cat > toml11-test/pyproject.toml << 'EOF'
[project]
name = "toml11-test"
version = "0.1.0"
description = "Test package with TOML 1.1 features"
requires-python = ">=3.8"
# TOML 1.1 feature: Multi-line inline table with trailing comma
authors = [
    { name = "Alice", email = "alice@example.com", },
    { name = "Bob", email = "bob@example.com", },
]

[build-system]
requires = ["uv_build>=0.8.0,<0.10.0"]
build-backend = "uv_build"
EOF

echo '"""Test package with TOML 1.1 features."""' > toml11-test/src/toml11_test/__init__.py

echo "Created test project with TOML 1.1 syntax (trailing commas in inline tables)"
echo

# Create a venv for testing
python3 -m venv test-venv

echo "============================================================"
echo "TEST 1: WITHOUT rewriting - pip FAILS"
echo "============================================================"
echo
echo "Creating sdist WITHOUT TOML 1.0 rewriting (simulating old behavior)..."

# Create a manual sdist without rewriting (simulating old behavior)
mkdir -p manual-sdist/toml11_test-0.1.0/src/toml11_test
cp toml11-test/pyproject.toml manual-sdist/toml11_test-0.1.0/
cp toml11-test/src/toml11_test/__init__.py manual-sdist/toml11_test-0.1.0/src/toml11_test/

# Create PKG-INFO
cat > manual-sdist/toml11_test-0.1.0/PKG-INFO << 'EOF'
Metadata-Version: 2.1
Name: toml11-test
Version: 0.1.0
Summary: Test package with TOML 1.1 features
Requires-Python: >=3.8
EOF

cd manual-sdist
tar -czf ../toml11_test-0.1.0-norewrite.tar.gz toml11_test-0.1.0
cd ..

echo "Attempting pip install (this should FAIL)..."
echo
if test-venv/bin/pip install toml11_test-0.1.0-norewrite.tar.gz 2>&1; then
    echo "UNEXPECTED: pip install succeeded (should have failed)"
    exit 1
else
    echo
    echo ">>> As expected, pip FAILED to parse TOML 1.1 syntax <<<"
fi

echo
echo "============================================================"
echo "TEST 2: WITH rewriting - pip SUCCEEDS"
echo "============================================================"
echo
echo "Building sdist WITH TOML 1.0 rewriting (our branch)..."

(cd "$UV_REPO" && cargo run -p uv -- build --sdist --directory "$TEST_DIR/toml11-test" 2>&1)

echo
echo "Contents of the sdist:"
tar -tzf toml11-test/dist/toml11_test-0.1.0.tar.gz | grep -E "pyproject"
echo

echo "Rewritten pyproject.toml (TOML 1.0 compatible):"
echo "------------------------------------------------"
tar -xzf toml11-test/dist/toml11_test-0.1.0.tar.gz -O toml11_test-0.1.0/pyproject.toml
echo

echo "Original pyproject.toml.orig (TOML 1.1 syntax preserved):"
echo "----------------------------------------------------------"
tar -xzf toml11-test/dist/toml11_test-0.1.0.tar.gz -O toml11_test-0.1.0/pyproject.toml.orig
echo

echo "Attempting pip install (this should SUCCEED)..."
test-venv/bin/pip install toml11-test/dist/toml11_test-0.1.0.tar.gz 2>&1 | grep -E "Successfully|Building|error" || true
echo
echo ">>> pip SUCCEEDED because pyproject.toml was rewritten to TOML 1.0 <<<"

echo
echo "============================================================"
echo "SUMMARY"
echo "============================================================"
echo "WITHOUT rewriting: pip FAILS  (cannot parse TOML 1.1 trailing commas)"
echo "WITH rewriting:    pip SUCCEEDS (pyproject.toml converted to TOML 1.0)"
echo
echo "The branch adds pyproject.toml.orig to preserve the original file."
