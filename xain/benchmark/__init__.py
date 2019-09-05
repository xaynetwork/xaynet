from absl import flags

from .benchmark import benchmarks

flags.DEFINE_string(
    "benchmark_name",
    "flul-fashion-mnist-100p-iid-balanced",
    f"One of: {[k for k in benchmarks]}",
)

flags.DEFINE_string(
    "group_name", None, "Group name used to gather the tasks of one benchmark"
)
