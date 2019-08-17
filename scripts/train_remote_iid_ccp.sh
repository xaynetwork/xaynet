#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $DIR/../

export BENCHMARK_GROUP=non_IID_to_IID

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

# IID
./scripts/train_remote.sh --benchmark_name fashion_mnist_100p_IID_balanced
