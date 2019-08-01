#!/bin/sh

# Exit immediately if a command exits with a non-zero status.
set -e

# IMAGE_TAG="autofl:$(date +%s)"
IMAGE_TAG="autofl:latest"

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

docker run --rm -it $IMAGE_TAG $@
