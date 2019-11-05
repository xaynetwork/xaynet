#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

isort --indent=4 -rc setup.py conftest.py benchmarks xain examples
black --exclude "xain/grpc/.*_pb2.*" setup.py conftest.py benchmarks xain examples
clang-format -style="{Language: Proto, BasedOnStyle: Google}" -i  protobuf/xain/grpc/*.proto
