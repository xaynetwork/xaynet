#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status.
set -e
# Print the command we're executing
set -x

echo "Starting $1 participants in parallel"
for ((i = 1; i <= $1; i++))
do
    run-participant \
        --data-directory data \
        --coordinator-url http://localhost:8081 \
        --write-performance-metrics "perf_${i}" \
        &
done
