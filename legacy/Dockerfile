FROM python:3.6-slim

ENV USER="xain"
ENV HOST="0.0.0.0"
ENV PORT="50051"
ENV PATH="/home/${USER}/.local/bin:${PATH}"
ENV CONFIG_FILE="/app/xain-fl.toml"

RUN groupadd ${USER} && useradd -g ${USER} ${USER}
RUN apt update -y && apt install -y python3-dev git

WORKDIR /app

COPY setup.py .
COPY xain_fl xain_fl/
COPY README.md .

RUN pip install -v .

# Remove everything, including dot files
RUN rm -rf ..?* .[!.]* *

COPY configs/xain-fl.toml ${CONFIG_FILE}

# Drop down to a non-root user
USER ${USER}

COPY --chown=${USER}:${USER} docker/entrypoint.sh entrypoint.sh
RUN chmod +x entrypoint.sh

ENTRYPOINT ["./entrypoint.sh"]
