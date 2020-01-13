#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

set -x
isort --indent=4 -rc setup.py xain_fl tests
black --line-length 100 setup.py xain_fl tests
