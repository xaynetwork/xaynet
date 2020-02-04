#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status
set -e
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

set -x
isort --settings-path=.isort.cfg -rc setup.py xain_fl tests
black setup.py xain_fl tests
