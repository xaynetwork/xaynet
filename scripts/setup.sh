#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

python -m pip install -U pip==19.3.1
python -m pip install -U setuptools==41.6.0
python -m pip install -e .[dev]
