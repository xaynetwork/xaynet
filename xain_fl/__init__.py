import os

from absl import flags

import xain_fl.config

if not xain_fl.config.check_config_file_exists():
    xain_fl.config.init_config()

c = xain_fl.config.load()

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
    "S3_results_bucket",
    c.get("S3", "results_bucket", fallback=None),
    "Bucket name for the results to be uploaded to",
)

os.environ["PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION"] = "python"
