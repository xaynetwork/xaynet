#!/usr/bin/env sh

set -o errexit
set -o pipefail
set -o nounset
# set -o xtrace

if [ $# -eq 0 ]; then
    exec coordinator -f ${CONFIG_FILE}
else
    exec coordinator "$@"
fi
