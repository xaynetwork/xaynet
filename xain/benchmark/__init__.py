from absl import flags

from .benchmark import benchmarks

flags.DEFINE_string("benchmark_name", None, f"One of: {[k for k in benchmarks]}")
