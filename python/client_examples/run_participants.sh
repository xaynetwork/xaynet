#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

echo "Starting $2 participants in parallel"
for ((i = 1; i <= $2; i++))
do
    python $1 &
done
