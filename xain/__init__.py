import os

from absl import flags

from xain.helpers import project

root_dir = project.root()
output_dir_default = os.path.abspath(os.path.join(root_dir, "output"))
results_dir_default = os.path.abspath(os.path.join(root_dir, "results"))

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

os.environ["PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION"] = "python"
