import os

import boto3
from absl import flags

from benchmarks.helpers.storage import listdir_recursive
from xain_fl.logger import get_logger

FLAGS = flags.FLAGS

client = boto3.client("s3")


logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


def push(group_name: str, task_name: str, output_dir: str = None):
    """Push everything in output directory to the S3_results_bucket on AWS S3

    Args:
        group_name (str)
        task_name (str)
        output_dir (str): If None FLAGS.output_dir will be used
    """
    # If no output_dir was explicitly set use output_dir which was passed via absl flags
    # and defaults to "git_root/output" and upload everything in it as results
    if output_dir is None:
        output_dir = FLAGS.output_dir

    bucket = FLAGS.S3_results_bucket

    ignored_files = [".gitkeep", ".DS_Store"]
    files = listdir_recursive(output_dir)

    for fname in files:
        if fname in ignored_files:
            continue

        local_path = os.path.join(output_dir, fname)
        key = os.path.join(group_name, task_name, fname)

        # pylint: disable=bare-except
        try:
            # Will throw an error if object does not exist
            client.head_object(Bucket=bucket, Key=key)
            logger.info("Key is already uploaded", key=key)
        # TODO: fix this by using something similar to:
        # https://stackoverflow.com/questions/33842944/check-if-a-key-exists-in-a-bucket-in-s3-using-boto3
        except:
            logger.info(
                "Uploading file to bucket",
                local_path=local_path,
                bucket=bucket,
                key=key,
            )
            client.upload_file(local_path, bucket, key)


def download():
    """Download all remote results which are locally not present from the S3_BUCKET on AWS S3"""
    # Use results_dir which was passed via absl flags and defaults to
    # "git_root/results" and upload everything in it as results
    results_dir = FLAGS.results_dir
    bucket = FLAGS.S3_results_bucket

    # Get list of all files which where uploaded to the bucket which contain
    # the group_name => integration_test
    all_objs = client.list_objects_v2(Bucket=bucket)

    if "Contents" not in all_objs:
        logger.info("No results to download")
        return

    actual_objs = [obj["Key"] for obj in all_objs["Contents"]]

    already_downloaded_files = listdir_recursive(results_dir)

    # enumerate local files recursively
    for key in actual_objs:
        if key not in already_downloaded_files:
            fname = os.path.join(results_dir, key)
            dname = os.path.dirname(fname)

            # Instead of checking just create with exist_ok
            os.makedirs(dname, exist_ok=True)

            logger.info(
                "Downloading file from bucket", filename=fname, bucket=bucket, key=key
            )

            client.download_file(Bucket=bucket, Key=key, Filename=fname)
