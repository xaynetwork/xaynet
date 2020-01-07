#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

set -x
rm -rf .mypy_cache
rm -rf .pytest_cache
rm -rf __pycache__
find xain_fl tests -type d -name __pycache__ -exec rm -r {} \+
rm -rf docs/_code_reference_*
rm -rf docs/_build
