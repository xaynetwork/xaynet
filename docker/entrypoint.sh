#!/usr/bin/env sh

set -Eeuxo pipefail
# set -o xtrace

if [ $# -eq 0 ]; then
    exec coordinator --config ${CONFIG_FILE}
else
    exec coordinator "$@"
fi
