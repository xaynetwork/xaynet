FROM python:3.6.8

WORKDIR /opt/ml/project

# Create output directory as its expected
RUN mkdir output

# First copy scripts and setup.py to install dependencies
# and avoid reinstalling dependencies when only changing the code
COPY setup.py setup.py

# Mostly taken from setup.py but split into
# required, dev and cpu to trigger image layer
# updates less often
RUN pip install -U pip==19.2.3
RUN pip install -U setuptools==41.2.0

# Now install required_installs as well as dev and cpu
RUN pip install -e .[cpu,dev]

COPY scripts scripts
COPY xain xain

# Rerun to install scripts
RUN ./scripts/setup.sh
