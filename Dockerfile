FROM python:3.6-alpine

ENV USER="xain"
ENV HOST="0.0.0.0"
ENV PORT="50051"
ENV PATH="/home/${USER}/.local/bin:${PATH}"

RUN addgroup -S ${USER} && adduser -S ${USER} -G ${USER}
RUN apk update && apk add python3-dev build-base git

WORKDIR /app

COPY setup.py .
COPY xain_fl xain_fl/
COPY README.md .

RUN pip install -v .

# Remove everything, including dot files
RUN rm -rf ..?* .[!.]* *

# Drop down to a non-root user
USER ${USER}

COPY --chown=${USER}:${USER} test_array.npy test_array.npy
COPY --chown=${USER}:${USER} docker/entrypoint.sh entrypoint.sh
RUN chmod +x entrypoint.sh

ENTRYPOINT ["./entrypoint.sh"]
