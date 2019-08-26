from absl import flags

flags.DEFINE_enum("benchmark_type", "fl", ["fl", "ea"], "Type of benchmark to run")

flags.DEFINE_string(
    "benchmark_name", None, "Name of the benchmark to be run e.g. 'integration_test'"
)

flags.DEFINE_string("group_name", None, "Group name to be plotted")
