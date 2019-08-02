#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

export AWS_PROFILE=xain-autofl

ECR_REPO="693828385217.dkr.ecr.eu-central-1.amazonaws.com/autofl"
IMAGE_TAG=$(python -c "import time; print(int(time.time() / 60))")
IMAGE_FULLNAME=$ECR_REPO:$IMAGE_TAG

# Read user_data and replace latest tag with our tag
USER_DATA=`cat $DIR/ec2_user_data.txt`
USER_DATA="${USER_DATA//latest/$IMAGE_TAG}"


# possible options for CPU only
# m5.large, m5.xlarge, m5.2xlarge, m5.4xlarge,
# m5.8xlarge, m5.12xlarge, m5.16xlarge, m5.24xlarge
# But beware it gets quite expensive... up $5.52 per Hour
INSTANCE_TYPE="m5.large"

build_image() {
    docker build -t $IMAGE_FULLNAME .
}

push_image() {
    $(aws ecr get-login --no-include-email --region eu-central-1)
    docker push $IMAGE_FULLNAME
    echo "Pushed $IMAGE_FULLNAME"
}

run_image() {
    aws ec2 run-instances \
    --image-id ami-08806c999be9493f1 \
    --count 1 \
    --instance-type $INSTANCE_TYPE \
    --key-name autofl_job \
    --subnet-id subnet-1bc3c466 \
    --iam-instance-profile Name=ECRFullAccess \
    --security-group-ids sg-01ff10b690dffbaf5 sg-01207b671ffadadf5 \
    --instance-initiated-shutdown-behavior terminate \
    --user-data "$USER_DATA"
}

build_image
push_image
run_image
