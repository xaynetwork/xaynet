import os
import random
import string

import boto3
import pytest
from absl import flags

from . import results

FLAGS = flags.FLAGS

# Let's use Amazon S3
s3 = boto3.resource("s3")
session = boto3.Session(profile_name="xain-xain")
client = session.client("s3")


@pytest.mark.integration
def test_push_results(populated_output_dir):
    # Prepare
    bucket = FLAGS.S3_bucket
    group_name = "integration_test"
    task_name = "".join(random.sample(string.ascii_lowercase, 5))
    # List all files which where in output directory
    expected_objs = [
        os.path.join(group_name, task_name, fname)
        for fname in os.listdir(populated_output_dir)
    ]

    # Execute
    results.push(group_name=group_name, task_name=task_name)

    # Assert
    # Get list of all files which where uploaded to the bucket which contain
    # the group_name => integration_test
    all_objs = client.list_objects_v2(Bucket=bucket)
    actual_objs = [obj["Key"] for obj in all_objs["Contents"]]

    # Check if all files where uploaded
    assert set(expected_objs).issubset(set(actual_objs)), "Could not upload all files"

    # Cleanup
    response = client.delete_objects(
        Bucket=bucket, Delete={"Objects": [{"Key": key} for key in expected_objs]}
    )

    assert (
        response["ResponseMetadata"]["HTTPStatusCode"] == 200
    ), "Cleaning up the bucket failed"


@pytest.mark.integration
def test_download_results(populated_results_dir, populated_S3_bucket):
    # Prepare
    expected_files = populated_S3_bucket

    # Execute
    results.download()

    actual_files = []

    # Assert
    for root, _, files in os.walk(populated_results_dir):
        for filename in files:
            local_path = os.path.join(root, filename)
            # get relative path to results_dir
            relative_path = os.path.relpath(local_path, populated_results_dir)
            actual_files.append(relative_path)

    # As we might have files in our results directory which are not on S3
    # anymore we will only check if everything which is on S3 was actualy downloaded
    assert len(actual_files) >= len(expected_files)
    assert set(expected_files).issubset(set(actual_files))
