#!/bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

# sort import
isort --check-only --indent=4 -rc setup.py conftest.py xain && echo "===> isort says: well done <===" &&

# format code
black --check --exclude "xain/grpc/.*_pb2.*" setup.py conftest.py xain && echo "===> black says: well done <===" &&

# check format of proto files
clang-format protobuf/xain/grpc/*.proto | diff protobuf/xain/grpc/*.proto - && echo "===> clang-format says: well done <===" &&

# lint
pylint --rcfile=pylint.ini xain && echo "===> pylint says: well done <===" &&

# type checks
mypy --ignore-missing-imports xain && echo "===> mypy says: well done <===" &&

# tests
pytest -v && echo "===> pytest/unmarked says: well done <===" &&

echo "All went well"
