#!/bin/bash

# This scripts expects an argument which will
# then be used for namespacing the results
# Ideally its the same as image tag from train_remote.sh

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

if [[ $# -eq 0 ]] ; then
    echo 'You have to pass a namespace as first argument'
    exit 0
fi

S3_BUCKET="s3://autofl-training"
NAMESPACE=$1

aws s3 cp --recursive --exclude ".gitkeep" ./output $S3_BUCKET/results/$NAMESPACE/
