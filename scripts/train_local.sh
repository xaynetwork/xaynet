#!/bin/sh

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

IMAGE_NAME="autofl"
IMAGE_TAG=$(python -c "import time; print(int(time.time() / 60))")
IMAGE_FULLNAME=$IMAGE_NAME:$IMAGE_TAG

docker build -t $IMAGE_FULLNAME .
docker run --rm -it $IMAGE_FULLNAME train
