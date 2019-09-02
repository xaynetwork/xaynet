import os
import random
import string

import boto3
import pytest
from absl import flags

FLAGS = flags.FLAGS


s3 = boto3.resource("s3")
session = boto3.Session()
client = session.client("s3")


def random_string(length: int):
    return "".join(random.sample(string.ascii_lowercase, length))


@pytest.fixture
def populated_output_dir(tmpdir):
    """Creates an output_dir with 10 files in it"""
    tmpdir = str(tmpdir)
    FLAGS(["test", f"--output_dir={tmpdir}"])

    for i in range(10):
        fname = os.path.join(tmpdir, f"file_{i}.txt")

        # create a new file and open it for writing
        with open(fname, "x") as f:
            f.write("To write or not to write?")
            f.close()

    return tmpdir


@pytest.fixture
def populated_results_dir(tmpdir):
    """results_dir containing one file"""
    tmpdir = str(tmpdir)
    FLAGS(["test", f"--results_dir={tmpdir}"])

    # Create a subdirectory to make the case more complex
    dname = os.path.join(tmpdir, "some_dir")
    os.mkdir(dname)

    fname = os.path.join(dname, "some_file.txt")

    # create a new file and open it for writing
    with open(fname, "x") as f:
        f.write("To write or not to write?")
        f.close()

    return tmpdir


@pytest.yield_fixture
def populated_S3_bucket():
    bucket = FLAGS.S3_bucket

    rnd_str = random_string(5)
    files = ["file_1.txt", "file_2.txt"]

    s3_keys = []

    for fname in files:
        key = "integration_test/populated_S3_bucket_fixture_" + f"{rnd_str}/{fname}"
        s3_keys.append(key)

        client.put_object(Bucket=bucket, Key=key, Body=b"foobar")

    yield s3_keys

    # Clean up after test
    client.delete_objects(
        Bucket=bucket, Delete={"Objects": [{"Key": key} for key in s3_keys]}
    )
