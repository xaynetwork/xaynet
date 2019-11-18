"""Extensive benchmark suite to evaluate convergence properties of federated learning
in different settings. The main module in which those scenarious are configured is
`benchmark.py`.
"""
from absl import flags

from .benchmark import benchmarks

flags.DEFINE_string("benchmark_name", None, f"One of: {[k for k in benchmarks]}")

flags.DEFINE_string(
    "group_name", None, "Group name used to gather the tasks of one benchmark"
)
