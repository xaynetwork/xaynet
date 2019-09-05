from absl import flags

from .benchmark import benchmarks

flags.DEFINE_string(
    "benchmark_name",
    "flul-fashion-mnist-100p-iid-balanced",
    f"One of: {[k for k in benchmarks]}",
)
