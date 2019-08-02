FROM python:3.7.4-buster

WORKDIR /opt/ml/project

# First copy scripts and setup.py to install dependencies
# and avoid reinstalling dependencies when only changing the code
COPY setup.py setup.py
COPY scripts/setup.sh scripts/setup.sh

RUN ./scripts/setup.sh

COPY scripts scripts
COPY autofl autofl

# Rerun to install scripts
RUN ./scripts/setup.sh
