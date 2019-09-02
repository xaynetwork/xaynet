import glob
import os

import boto3
from absl import flags

FLAGS = flags.FLAGS

# Let's use Amazon S3
s3 = boto3.resource("s3")
session = boto3.Session()
client = session.client("s3")


def listdir_recursive(dname: str):
    """Lists all files found in {dname} with relative path

    Args:
        dname (str): Absolute path to directory

    Returns:
        List[str]: List of all files with relative path to dname
    """
    return [
        os.path.relpath(fpath, dname)
        for fpath in glob.glob(f"{dname}/**", recursive=True)
        if os.path.isfile(fpath)
    ]


def push(group_name: str, task_name: str):
    """
    Method will push everything in output directory to the S3_BUCKET on AWS S3

    Args:
    @param group_name
    @param task_name
    @param bucket
    """
    # Use output_dir which was passed via absl flags and defaults to
    # "git_root/output" and upload everything in it as results
    output_dir = FLAGS.output_dir
    bucket = FLAGS.S3_bucket

    # enumerate local files recursively
    for root, _, files in os.walk(output_dir):
        files = [f for f in files if f not in [".gitkeep", ".DS_Store"]]

        for fname in files:
            local_path = os.path.join(root, fname)
            relative_path = os.path.relpath(local_path, output_dir)
            key = os.path.join(group_name, task_name, relative_path)

            # pylint: disable=bare-except
            try:
                # Will throw an error if object does not exist
                client.head_object(Bucket=bucket, Key=key)
            # TODO: fix this by using something similar to:
            # https://stackoverflow.com/questions/33842944/check-if-a-key-exists-in-a-bucket-in-s3-using-boto3
            except:
                client.upload_file(local_path, bucket, key)


def download():
    """
    Method will download all remote results which are locally not present
    from the S3_BUCKET on AWS S3

    Args:
    @param group_name
    @param task_name
    @param bucket
    """
    # Use results_dir which was passed via absl flags and defaults to
    # "git_root/results" and upload everything in it as results
    results_dir = FLAGS.results_dir
    bucket = FLAGS.S3_bucket

    # Get list of all files which where uploaded to the bucket which contain
    # the group_name => integration_test
    all_objs = client.list_objects_v2(Bucket=bucket)
    actual_objs = [obj["Key"] for obj in all_objs["Contents"]]

    already_downloaded_files = listdir_recursive(results_dir)

    # enumerate local files recursively
    for key in actual_objs:
        if key not in already_downloaded_files:
            fname = os.path.join(results_dir, key)
            dname = os.path.dirname(fname)

            # Instead of checking just create with exist_ok
            os.makedirs(dname, exist_ok=True)

            client.download_file(Bucket=bucket, Key=key, Filename=fname)
