#!/bin/bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

cd $DIR/../

pip install -U pip==19.1.1
pip install -e .[dev] .[cpu]
