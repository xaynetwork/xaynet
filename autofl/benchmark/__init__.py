from os import path

from absl import flags

module_dir = path.dirname(__file__)  # directory in which this module resides
root_dir = path.abspath(path.join(module_dir, "../.."))  # project root dir
output_dir_default = path.abspath(path.join(root_dir, "output"))

# following: https://abseil.io/docs/cpp/guides/flags#flags-best-practices
# we will define our flags in this file
FLAGS = flags.FLAGS


flags.DEFINE_string(
    "output_dir", output_dir_default, "Output directory as absolute path"
)

flags.DEFINE_enum("benchmark_type", "fl", ["fl", "ea"], "Type of benchmark to run")

flags.DEFINE_string(
    "benchmark_name",
    None,
    "Name of the benchmark to be run. Example: bench_fl.py"
    + " - other modules should use it in similar ways",
)
