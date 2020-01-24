#!/usr/bin/env sh

set -o errexit
set -o pipefail
set -o nounset
# set -o xtrace

if [ $# -eq 0 ]; then
    exec coordinator --host ${HOST} --port ${PORT}
else
    exec coordinator "$@"
fi
