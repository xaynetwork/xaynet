#!/bin/bash

# clang-format does not provide a way to check the files.
# This function iterates over a list of files and checks each one of them
# for formatting errors
clang_format() {
    local_ret=0

    for f in ./protobuf/xain_fl/cproto/*.proto
    do
        echo "Processing $f"
        clang-format -style="{Language: Proto, BasedOnStyle: Google}" $f | diff $f -

        if [ $? -ne 0 ] ; then
            local local_ret=1
        fi

    done
    return $local_ret
}

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

# sort import
isort --check-only --indent=4 -rc setup.py conftest.py xain_fl && echo "===> isort says: well done <===" &&

# format code
black --check --exclude "xain_fl/cproto/.*_pb2.*" setup.py conftest.py xain_fl && echo "===> black says: well done <===" &&

# lint
pylint --rcfile=pylint.ini xain_fl && echo "===> pylint says: well done <===" &&

# type checks
mypy xain_fl && echo "===> mypy says: well done <===" &&

# documentation checks
(cd docs/ && SPHINXOPTS="-W" make docs) && echo "===> sphinx-build says: well done <===" &&

# tests
pytest -v && echo "===> pytest/unmarked says: well done <===" &&

echo "All went well"
