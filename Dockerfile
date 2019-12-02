FROM python:3.6.8-slim

WORKDIR /opt/app

# Create output directory as its expected
RUN mkdir output

# Upgrade pip and setuptools
RUN python -m pip install -U pip==19.3.1 setuptools==41.6.0

# First copy scripts and setup.py to install dependencies
# and avoid reinstalling dependencies when only changing the code
COPY setup.py setup.py

# These files are needed for the setup.py to work
COPY xain_fl/__version__.py xain_fl/__version__.py
COPY README.md README.md

# Install only install_requires
RUN python setup.py egg_info && \
    LN=$(awk '/tensorflow/{ print NR; exit }' xain_fl.egg-info/requires.txt) && \
    IR=$(head -n $LN xain_fl.egg-info/requires.txt | awk '{gsub(/\[.+\]/,"");}1') && \
    python -m pip install $IR

COPY xain_fl xain_fl
COPY protobuf protobuf

RUN python -m pip install .
