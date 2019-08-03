#!/bin/bash

# This scripts expects an argument which will
# then be used for namespacing the results
# Ideally its the same as image tag from train_remote.sh

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

S3_BUCKET="s3://autofl-training"

aws s3 sync $S3_BUCKET/results ./results
