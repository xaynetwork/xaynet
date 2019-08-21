#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

read -p "Enter a group id which will be append to IID_nonIID_" GROUP_ID

# Will be used in image_tag.py
# unfortunatly the whole thing is a bit hacky
export BENCHMARK_GROUP="IID_nonIID_$GROUP_ID"

echo "Running benchmark with group id => $BENCHMARK_GROUP"

# IID
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_IID_balanced

# non-IID
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_01cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_02cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_03cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_04cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_05cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_06cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_07cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_08cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_09cpp
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_10cpp
