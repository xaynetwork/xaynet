from absl import flags

import xain.config

from .benchmark import benchmarks

config = xain.config.load()

flags.DEFINE_string(
    "benchmark_name",
    config.get("Benchmark", "default_benchmark_name", fallback=None),
    f"One of: {[k for k in benchmarks]}",
)

flags.DEFINE_string(
    "group_name", None, "Group name used to gather the tasks of one benchmark"
)
