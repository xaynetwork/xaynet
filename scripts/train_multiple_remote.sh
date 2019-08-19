#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

# Usage of this script
# Example:
# ./scripts/train_multiple_remote.sh my_goup fashion_mnist_100p_03cpp fashion_mnist_100p_04cpp

for var in "$@"
do
    if [ $1 = "$var" ]
    then
        export BENCHMARK_GROUP=$1
    else
        ./scripts/train_remote.sh --benchmark_name $var
    fi
done
