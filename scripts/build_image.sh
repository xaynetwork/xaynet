#!/bin/sh

# Exit immediately if a command exits with a non-zero status.
set -e

DOCKERFILE="ops/docker/Dockerfile"
# IMAGE_TAG=$(date +%s)
IMAGE_TAG="autofl"

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

docker build -f $DOCKERFILE -t $IMAGE_TAG .
