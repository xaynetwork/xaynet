# FROM tensorflow/tensorflow:1.14.0-gpu-py3
FROM python:3.7.4-buster

WORKDIR /opt/ml/project

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update
RUN apt-get install -y awscli

# First copy scripts and setup.py to install dependencies
# and avoid reinstalling dependencies when only changing the code
COPY setup.py setup.py
COPY scripts/setup.sh scripts/setup.sh

RUN ./scripts/setup.sh

# Create output directory as its expected
RUN mkdir output

COPY scripts scripts
COPY autofl autofl

# Rerun to install scripts
RUN ./scripts/setup.sh
