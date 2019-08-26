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

# This will install pytorch with CPU only support and
# avoid downloading the > 600MB GPU enabled version
RUN pip install https://download.pytorch.org/whl/cpu/torch-1.1.0-cp36-cp36m-linux_x86_64.whl \
    https://download.pytorch.org/whl/cpu/torchvision-0.3.0-cp36-cp36m-linux_x86_64.whl

# Now install required_installs as well as dev and cpu
RUN pip install -e .[cpu,dev]

COPY scripts scripts
COPY autofl autofl

# Rerun to install scripts
RUN ./scripts/setup.sh
