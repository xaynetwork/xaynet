from os import path

from absl import flags

module_dir = path.dirname(__file__)  # directory in which this module resides
root_dir = path.abspath(path.join(module_dir, ".."))  # project root dir
output_dir_default = path.abspath(path.join(root_dir, "output"))
results_dir_default = path.abspath(path.join(root_dir, "results"))

# following: https://abseil.io/docs/cpp/guides/flags#flags-best-practices
# we will define our flags in this file
FLAGS = flags.FLAGS

flags.DEFINE_string(
    "output_dir", output_dir_default, "Output directory as absolute path"
)

flags.DEFINE_string(
    "results_dir", results_dir_default, "Results directory as absolute path"
)

flags.DEFINE_string(
    "S3_bucket", "xain-results", "Bucket name for the results to be uploaded to"
)
