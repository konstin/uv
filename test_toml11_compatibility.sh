#!/bin/bash
# Test script to demonstrate TOML 1.1 backward compatibility
#
# This script shows that:
# 1. The released uv_build causes pip to fail with TOML 1.1 syntax
# 2. The branch uv_build makes it succeed by rewriting to TOML 1.0

set -e

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

echo "Created test project with TOML 1.1 features (trailing commas in inline tables)"
echo
echo "=== Test 1: Show that TOML 1.1 fails with Python's tomllib ==="
python3 -c "
import tomllib

toml_1_1_content = '''
[project]
authors = [
    { name = \"Alice\", email = \"alice@example.com\", },
]
'''

try:
    tomllib.loads(toml_1_1_content)
    print('UNEXPECTED: TOML 1.1 content should have failed')
except Exception as e:
    print(f'As expected, TOML 1.1 fails with tomllib: {type(e).__name__}')
    print(f'  Error: {e}')
"
echo

echo "=== Test 2: Build with branch uv (TOML 1.0 rewriting) ==="
UV_BIN="${UV_BIN:-/home/user/uv/target/debug/uv}"
if [ ! -f "$UV_BIN" ]; then
    echo "ERROR: uv binary not found at $UV_BIN"
    echo "Build it first with: cargo build -p uv"
    exit 1
fi

cd toml11-test
"$UV_BIN" build --sdist 2>&1 || { echo "Build failed!"; exit 1; }
echo

echo "=== Test 3: Check sdist contents ==="
tar -tzf dist/toml11_test-0.1.0.tar.gz | grep pyproject
echo

echo "=== Test 4: Show the rewritten pyproject.toml (TOML 1.0 compatible) ==="
echo "--- pyproject.toml (rewritten) ---"
tar -xzf dist/toml11_test-0.1.0.tar.gz -O toml11_test-0.1.0/pyproject.toml
echo
echo "--- pyproject.toml.orig (original) ---"
tar -xzf dist/toml11_test-0.1.0.tar.gz -O toml11_test-0.1.0/pyproject.toml.orig
echo

echo "=== Test 5: Verify pip can install the sdist ==="
cd "$TEST_DIR"
python3 -m venv pip-test-venv
pip-test-venv/bin/pip install --quiet toml11-test/dist/toml11_test-0.1.0.tar.gz 2>&1 || {
    echo "ERROR: pip install failed!"
    exit 1
}
echo "SUCCESS: pip successfully installed the package from sdist"
echo

echo "=== Test 6: Verify the package is installed ==="
pip-test-venv/bin/python -c "import toml11_test; print(f'Imported: {toml11_test}')"
echo

echo "=== Summary ==="
echo "The branch uv_build successfully:"
echo "1. Parses TOML 1.1 syntax (trailing commas in inline tables)"
echo "2. Rewrites to TOML 1.0 format (array of tables instead of inline tables)"
echo "3. Preserves the original as pyproject.toml.orig"
echo "4. Allows pip (which only supports TOML 1.0) to install the package"
