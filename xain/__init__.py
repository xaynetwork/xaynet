import os

from absl import flags, logging

import xain.config

if not xain.config.check_config_file_exists():
    xain.config.init_config()

c = xain.config.load()

# following: https://abseil.io/docs/cpp/guides/flags#flags-best-practices
# we will define our flags in this file
FLAGS = flags.FLAGS


flags.DEFINE_string(
    "output_dir", c.get("Path", "output_dir"), "Output directory as absolute path"
)

flags.DEFINE_string(
    "results_dir", c.get("Path", "results_dir"), "Results directory as absolute path"
)

flags.DEFINE_string(
    "S3_bucket",
    c.get("S3", "results_bucket", fallback=None),
    "Bucket name for the results to be uploaded to",
)
