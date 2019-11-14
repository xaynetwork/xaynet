#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

rm -rf .mypy_cache
rm -rf .pytest_cache
find . -type d -name __pycache__ -exec rm -r {} \+
