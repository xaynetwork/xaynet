#!/bin/sh

# Exit immediately if a command exits with a non-zero status.
set -e

# Do this early to avoid building if
# credentials are not even setup
export AWS_PROFILE=xain-autofl
$(aws ecr get-login --no-include-email)

ECR_REPO="693828385217.dkr.ecr.eu-central-1.amazonaws.com/autofl"
DOCKERFILE="ops/docker/Dockerfile"
IMAGE_NAME="autofl"
IMAGE_TAG=$(python -c "import time; print(int(time.time() / 60))")

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../


docker build \
    -f $DOCKERFILE \
    -t $IMAGE_NAME:latest \
    -t $IMAGE_NAME:$IMAGE_TAG \
    -t $ECR_REPO:$IMAGE_TAG \
    .

docker push $ECR_REPO
