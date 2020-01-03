#!/bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

# sort import
isort --check-only --indent=4 -rc setup.py conftest.py xain_fl && echo "===> isort says: well done <===" &&

# format code
black --check setup.py conftest.py xain_fl && echo "===> black says: well done <===" &&

# lint
pylint --rcfile=pylint.ini xain_fl && echo "===> pylint says: well done <===" &&

# type checks
mypy xain_fl && echo "===> mypy says: well done <===" &&

# documentation checks
(cd docs/ && SPHINXOPTS="-W" make docs) && echo "===> sphinx-build says: well done <===" &&

# tests
pytest -v && echo "===> pytest/unmarked says: well done <===" &&

echo "All went well"
