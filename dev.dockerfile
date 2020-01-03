FROM python:3.6-alpine

RUN apk update && apk add python3-dev build-base

WORKDIR /app
COPY setup.py .
COPY protobuf protobuf/
COPY xain_fl xain_fl/
COPY README.md .

RUN pip install -e .[dev]

CMD ["python3", "setup.py", "--fullname"]
